// Format a positive integer as Roman numerals (e.g. 2026 → "MMXXVI"). The
// editorial pages render project years in Roman to match the brand voice.
// Returns "" for non-positive input so callers can fall back cleanly.

const MAP: [number, string][] = [
  [1000, "M"], [900, "CM"], [500, "D"], [400, "CD"], [100, "C"],
  [90, "XC"], [50, "L"], [40, "XL"], [10, "X"], [9, "IX"],
  [5, "V"], [4, "IV"], [1, "I"],
];

export function toRoman(year: number): string {
  let n = Math.floor(year);
  if (n <= 0) return "";
  let out = "";
  for (const [value, sym] of MAP) {
    while (n >= value) {
      out += sym;
      n -= value;
    }
  }
  return out;
}
