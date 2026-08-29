import { describe, it, expect } from "vite-plus/test";
import { sha256Utf8 } from "./sha";

describe("sha256Utf8", () => {
  it("hashes UTF-8 text to hex", async () => {
    const hex = await sha256Utf8("hello\n");
    expect(hex).toMatch(/^[0-9a-f]{64}$/);
    expect(hex).toBe(await sha256Utf8("hello\n"));
    expect(hex).not.toBe(await sha256Utf8("hello"));
  });
});
