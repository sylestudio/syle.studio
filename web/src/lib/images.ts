import { thumbHashToDataURL } from "thumbhash";
import type { ImageVariant } from "./api";

/** Decode a lowercase hex string into bytes. */
export function hexToBytes(hex: string): Uint8Array {
  const out = new Uint8Array(Math.floor(hex.length / 2));
  for (let i = 0; i < out.length; i++) {
    out[i] = parseInt(hex.substr(i * 2, 2), 16);
  }
  return out;
}

/** ThumbHash hex (as stored by the API) → inline data-URL placeholder. */
export function thumbDataUrl(hex: string): string {
  if (!hex) return "";
  try {
    return thumbHashToDataURL(hexToBytes(hex));
  } catch {
    return "";
  }
}

export interface Sources {
  avif: string;
  jpeg: string;
  fallback: string;
}

/** Group variants into per-format `srcset` strings + a JPEG fallback. */
export function buildSources(variants: ImageVariant[]): Sources {
  const byFormat = (fmt: "avif" | "jpeg") =>
    variants
      .filter((v) => v.format === fmt)
      .sort((a, b) => a.width - b.width);

  const srcset = (vs: ImageVariant[]) =>
    vs.map((v) => `${v.path} ${v.width}w`).join(", ");

  const jpegs = byFormat("jpeg");
  return {
    avif: srcset(byFormat("avif")),
    jpeg: srcset(jpegs),
    fallback: jpegs.length ? jpegs[jpegs.length - 1].path : "",
  };
}
