//! Generate immutable responsive AVIF/JPEG assets for the Astro project pages.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use syle_core::ingest::{content_key, ingest, INGEST_PIPELINE_VERSION};
use syle_types::ImageFormat;
use uuid::Uuid;

const TARGET_WIDTHS: &[u32] = &[480, 960, 1440, 2400];

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ManifestImage {
    #[serde(default = "current_pipeline_version")]
    pipeline_version: u32,
    source_sha256: String,
    width: u32,
    height: u32,
    thumbhash: String,
    variants: Vec<ManifestVariant>,
}

#[derive(Deserialize, Serialize)]
struct ManifestVariant {
    format: String,
    width: u32,
    path: String,
}

fn current_pipeline_version() -> u32 {
    INGEST_PIPELINE_VERSION
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    anyhow::ensure!(
        args.len() == 3,
        "usage: optimize-static <source-img-dir> <public-output-dir> <manifest.json>"
    );
    let source_root = PathBuf::from(&args[0]);
    let output_root = PathBuf::from(&args[1]);
    let manifest_path = PathBuf::from(&args[2]);
    anyhow::ensure!(source_root.is_dir(), "source directory does not exist");
    anyhow::ensure!(
        output_root.file_name().and_then(|name| name.to_str()) == Some("project-media"),
        "refusing to manage an output directory not named 'project-media'"
    );

    std::fs::create_dir_all(&output_root)?;
    let mut sources = Vec::new();
    collect_images(&source_root, &mut sources)?;
    sources.sort();

    let mut previous: BTreeMap<String, ManifestImage> = std::fs::read(&manifest_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    let mut manifest = BTreeMap::new();
    let mut expected_outputs = BTreeSet::new();
    for source in sources {
        let bytes = std::fs::read(&source)
            .with_context(|| format!("reading source image {}", source.display()))?;
        let source_sha256 = content_key(&bytes);
        let relative = source.strip_prefix(&source_root)?;
        let web_path = format!("/img/{}", path_to_web(relative)?);

        if let Some(entry) = previous.remove(&web_path) {
            if entry.pipeline_version == INGEST_PIPELINE_VERSION
                && entry.source_sha256 == source_sha256
                && manifest_outputs_exist(&output_root, &entry)
            {
                expected_outputs.extend(entry.variants.iter().filter_map(|variant| {
                    variant
                        .path
                        .strip_prefix("/project-media/")
                        .map(str::to_owned)
                }));
                manifest.insert(web_path, entry);
                continue;
            }
        }

        let output = ingest(&bytes, TARGET_WIDTHS)
            .with_context(|| format!("optimizing {}", source.display()))?;
        let mut variants = Vec::with_capacity(output.derivatives.len());

        for derivative in output.derivatives {
            let ext = match derivative.format {
                ImageFormat::Avif => "avif",
                ImageFormat::Jpeg => "jpeg",
            };
            let filename = format!("{}.{}", content_key(&derivative.bytes), ext);
            let destination = output_root.join(&filename);
            atomic_write_if_changed(&destination, &derivative.bytes)?;
            expected_outputs.insert(filename.clone());
            variants.push(ManifestVariant {
                format: ext.to_owned(),
                width: derivative.width,
                path: format!("/project-media/{filename}"),
            });
        }

        manifest.insert(
            web_path,
            ManifestImage {
                pipeline_version: INGEST_PIPELINE_VERSION,
                source_sha256,
                width: output.width,
                height: output.height,
                thumbhash: output.thumbhash,
                variants,
            },
        );
    }

    remove_stale_outputs(&output_root, &expected_outputs)?;
    let json = serde_json::to_vec_pretty(&manifest)?;
    atomic_write_if_changed(&manifest_path, &[json.as_slice(), b"\n"].concat())?;
    eprintln!(
        "optimized {} sources into {} immutable derivatives",
        manifest.len(),
        expected_outputs.len()
    );
    Ok(())
}

fn manifest_outputs_exist(output_root: &Path, entry: &ManifestImage) -> bool {
    !entry.variants.is_empty()
        && entry.variants.iter().all(|variant| {
            let Some(filename) = variant.path.strip_prefix("/project-media/") else {
                return false;
            };
            let Some((expected_hash, _extension)) = filename.split_once('.') else {
                return false;
            };
            std::fs::read(output_root.join(filename))
                .is_ok_and(|bytes| content_key(&bytes) == expected_hash)
        })
}

fn collect_images(dir: &Path, output: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_images(&path, output)?;
        } else if matches!(
            path.extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_lowercase)
                .as_deref(),
            Some("jpg" | "jpeg" | "png" | "webp")
        ) {
            output.push(path);
        }
    }
    Ok(())
}

fn path_to_web(path: &Path) -> Result<String> {
    let parts: Result<Vec<_>, _> = path
        .components()
        .map(|component| match component {
            std::path::Component::Normal(part) => part
                .to_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| anyhow::anyhow!("image path is not UTF-8")),
            _ => Err(anyhow::anyhow!("image path is not normalized")),
        })
        .collect();
    Ok(parts?.join("/"))
}

fn atomic_write_if_changed(path: &Path, bytes: &[u8]) -> Result<()> {
    if std::fs::read(path).is_ok_and(|existing| existing == bytes) {
        return Ok(());
    }
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("output path has no parent"))?;
    std::fs::create_dir_all(parent)?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("invalid output filename"))?;
    let temp = parent.join(format!(".{name}.{}.tmp", Uuid::new_v4().simple()));
    let result = (|| -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        std::fs::rename(&temp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

fn remove_stale_outputs(output_root: &Path, expected: &BTreeSet<String>) -> Result<()> {
    for entry in std::fs::read_dir(output_root)? {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !expected.contains(name) {
            std::fs::remove_file(&path)?;
        }
    }
    Ok(())
}
