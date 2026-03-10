import { describe, it, expect } from "vitest";
import { getAuthorInitials, hashAuthorColor } from "./author-utils";

describe("getAuthorInitials", () => {
  it("returns initials for two-word name", () => {
    expect(getAuthorInitials("John Doe")).toBe("JD");
  });

  it("returns single initial for single name", () => {
    expect(getAuthorInitials("Alice")).toBe("A");
  });

  it("returns first two initials for three words", () => {
    expect(getAuthorInitials("John Middle Doe")).toBe("JM");
  });

  it("handles extra whitespace", () => {
    expect(getAuthorInitials("  John   Doe  ")).toBe("JD");
  });

  it("uppercases lowercase names", () => {
    expect(getAuthorInitials("john doe")).toBe("JD");
  });

  it("returns ? for empty string", () => {
    expect(getAuthorInitials("")).toBe("?");
  });
});

describe("hashAuthorColor", () => {
  const AVATAR_HUES = [0, 45, 90, 160, 210, 260, 310, 340];

  it("returns valid hsl string", () => {
    const color = hashAuthorColor("John Doe");
    expect(color).toMatch(/^hsl\(\d+, 50%, 40%\)$/);
  });

  it("is deterministic", () => {
    const a = hashAuthorColor("Alice");
    const b = hashAuthorColor("Alice");
    expect(a).toBe(b);
  });

  it("produces different colors for different names", () => {
    const names = ["Alice", "Bob", "Charlie", "Diana", "Eve", "Frank"];
    const colors = names.map(hashAuthorColor);
    const unique = new Set(colors);
    expect(unique.size).toBeGreaterThan(1);
  });

  it("uses hue from AVATAR_HUES", () => {
    const names = ["Alice", "Bob", "Charlie", "Diana", "Eve"];
    for (const name of names) {
      const color = hashAuthorColor(name);
      const hue = parseInt(color.match(/^hsl\((\d+)/)?.[1] ?? "-1", 10);
      expect(AVATAR_HUES).toContain(hue);
    }
  });
});
