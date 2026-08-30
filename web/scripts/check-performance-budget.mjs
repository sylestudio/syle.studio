import { readdir, readFile, stat } from "node:fs/promises";
import { extname, join } from "node:path";
import { fileURLToPath } from "node:url";

const webRoot = fileURLToPath(new URL("..", import.meta.url));
const distRoot = join(webRoot, "dist");
const derivativeRoot = join(distRoot, "project-media");
const maxDerivativeBytes = 1_500_000;
const maxDerivativeBytesPerSource = 1.5 * 1024 * 1024;
const manifest = JSON.parse(
  await readFile(join(webRoot, "src", "generated", "static-images.json"), "utf8"),
);
const sourceCount = Object.keys(manifest).length;
const maxDerivativeSetBytes = sourceCount * maxDerivativeBytesPerSource;

const walk = async (directory) => {
  const output = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) output.push(...(await walk(path)));
    else output.push(path);
  }
  return output;
};

const derivatives = await walk(derivativeRoot);
let derivativeBytes = 0;
for (const path of derivatives) {
  const bytes = (await stat(path)).size;
  derivativeBytes += bytes;
  if (bytes > maxDerivativeBytes) {
    throw new Error(`Derivative exceeds ${maxDerivativeBytes} bytes: ${path} (${bytes})`);
  }
}
if (derivativeBytes > maxDerivativeSetBytes) {
  throw new Error(
    `Derivative set exceeds ${maxDerivativeSetBytes} bytes: ${derivativeBytes}`,
  );
}

const legacyPhotoRoots = ["Dango", "Insomnio", "Obsesion", "Simbolo", "Slide", "Syle"];
for (const root of legacyPhotoRoots) {
  try {
    const files = await walk(join(distRoot, "img", root));
    const raster = files.find((path) =>
      [".jpg", ".jpeg", ".png", ".webp"].includes(extname(path).toLowerCase()),
    );
    if (raster) throw new Error(`Unoptimized public original was deployed: ${raster}`);
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
  }
}

const pagePaths = [
  join(distRoot, "index.html"),
  ...["catedral", "dango", "insomnio", "laobsesion", "rupture", "syle"].map((slug) =>
    join(distRoot, "proyectos", slug, "index.html"),
  ),
];
for (const pagePath of pagePaths) {
  const html = await readFile(pagePath, "utf8");
  const renderedHtml = html.replace(/<!--[\s\S]*?-->/g, "");
  if (/src=["']\/img\/(?:Dango|Insomnio|Obsesion|Simbolo|Slide|Syle)\//.test(renderedHtml)) {
    throw new Error(`Page still references an unoptimized project original: ${pagePath}`);
  }
  const responsiveImages =
    renderedHtml.match(/<img\b[^>]*src="\/project-media\/[^>]+>/g) ?? [];
  if (responsiveImages.length === 0) {
    throw new Error(`Page has no generated responsive images: ${pagePath}`);
  }
  for (const image of responsiveImages) {
    for (const attribute of ["srcset=", "sizes=", "width=", "height=", "loading=", "decoding="]) {
      if (!image.includes(attribute)) {
        throw new Error(`Responsive image lacks ${attribute} in ${pagePath}: ${image}`);
      }
    }
  }
}

console.log(
  `Performance budget passed: ${derivatives.length} derivatives, ` +
    `${(derivativeBytes / 1024 / 1024).toFixed(2)} MiB total, ` +
    `${sourceCount} sources, ` +
    `${maxDerivativeBytes} bytes max each.`,
);
