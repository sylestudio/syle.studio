import { describe, it, expect } from "vitest";
import { hexToBytes, buildSources } from "./images";
import type { ImageVariant } from "./api";

describe("hexToBytes", () => {
  it("decodes an even-length hex string", () => {
    expect(Array.from(hexToBytes("00ff10"))).toEqual([0, 255, 16]);
  });
  it("returns empty for empty input", () => {
    expect(hexToBytes("").length).toBe(0);
  });
});

describe("buildSources", () => {
  const variants: ImageVariant[] = [
    { format: "jpeg", width: 960, path: "/m/j960.jpeg" },
    { format: "avif", width: 480, path: "/m/a480.avif" },
    { format: "avif", width: 960, path: "/m/a960.avif" },
    { format: "jpeg", width: 480, path: "/m/j480.jpeg" },
  ];

  it("groups by format and sorts srcset ascending by width", () => {
    const s = buildSources(variants);
    expect(s.avif).toBe("/m/a480.avif 480w, /m/a960.avif 960w");
    expect(s.jpeg).toBe("/m/j480.jpeg 480w, /m/j960.jpeg 960w");
  });

  it("uses the largest jpeg as the <img> fallback", () => {
    expect(buildSources(variants).fallback).toBe("/m/j960.jpeg");
  });
});
