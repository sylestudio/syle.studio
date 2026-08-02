import { createHash } from "node:crypto";
import { readdir, readFile } from "node:fs/promises";
import { extname, join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";

const webRoot = fileURLToPath(new URL("..", import.meta.url));
const sourceRoot = join(webRoot, "media-src", "img");
const outputRoot = join(webRoot, "public", "project-media");
const manifestPath = join(webRoot, "src", "generated", "static-images.json");
const rasterExtensions = new Set([".jpg", ".jpeg", ".png", ".webp"]);

const walk = async (directory) => {
  const output = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) output.push(...(await walk(path)));
    else if (rasterExtensions.has(extname(entry.name).toLowerCase())) output.push(path);
  }
  return output;
};

const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");
const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
const sourceFiles = await walk(sourceRoot);
const expectedKeys = new Set();

for (const sourcePath of sourceFiles) {
  const key = `/img/${relative(sourceRoot, sourcePath).split(sep).join("/")}`;
  expectedKeys.add(key);
  const entry = manifest[key];
  if (!entry) throw new Error(`Missing generated manifest entry: ${key}`);
  if (!Number.isInteger(entry.pipelineVersion) || entry.pipelineVersion < 1) {
    throw new Error(`Missing pipeline version for generated image: ${key}`);
  }
  const sourceDigest = sha256(await readFile(sourcePath));
  if (entry.sourceSha256 !== sourceDigest) {
    throw new Error(`Source changed without regenerating responsive images: ${key}`);
  }
  if (!entry.width || !entry.height || !entry.variants?.length) {
    throw new Error(`Incomplete generated manifest entry: ${key}`);
  }
  for (const variant of entry.variants) {
    const prefix = "/project-media/";
    if (!variant.path.startsWith(prefix)) {
      throw new Error(`Unsafe generated path for ${key}: ${variant.path}`);
    }
    const filename = variant.path.slice(prefix.length);
    if (!/^[a-f0-9]{64}\.(?:avif|jpeg)$/.test(filename)) {
      throw new Error(`Invalid immutable derivative name for ${key}: ${filename}`);
    }
    const bytes = await readFile(join(outputRoot, filename));
    const expectedDigest = filename.split(".")[0];
    if (sha256(bytes) !== expectedDigest) {
      throw new Error(`Generated derivative hash mismatch: ${variant.path}`);
    }
  }
}

for (const key of Object.keys(manifest)) {
  if (!expectedKeys.has(key)) throw new Error(`Stale generated manifest entry: ${key}`);
}

console.log(`Verified ${expectedKeys.size} responsive source images.`);
