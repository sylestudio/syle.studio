import { describe, it, expect } from "vitest";
import { toRoman } from "./roman";

describe("toRoman", () => {
  it("renders the project year", () => {
    expect(toRoman(2026)).toBe("MMXXVI");
  });
  it("handles subtractive forms", () => {
    expect(toRoman(4)).toBe("IV");
    expect(toRoman(9)).toBe("IX");
    expect(toRoman(40)).toBe("XL");
    expect(toRoman(1990)).toBe("MCMXC");
  });
  it("returns empty for non-positive input", () => {
    expect(toRoman(0)).toBe("");
    expect(toRoman(-5)).toBe("");
  });
});
