import { describe, it, expect } from "vitest";
import { getStatusLabel } from "./status-icons";

describe("getStatusLabel", () => {
  it("returns M for modified", () => {
    expect(getStatusLabel("modified")).toBe("M");
  });

  it("returns A for added", () => {
    expect(getStatusLabel("added")).toBe("A");
  });

  it("returns D for deleted", () => {
    expect(getStatusLabel("deleted")).toBe("D");
  });

  it("returns R for renamed", () => {
    expect(getStatusLabel("renamed")).toBe("R");
  });

  it("returns U for untracked", () => {
    expect(getStatusLabel("untracked")).toBe("U");
  });

  it("returns ! for conflict statuses", () => {
    expect(getStatusLabel("bothModified")).toBe("!");
    expect(getStatusLabel("bothAdded")).toBe("!");
    expect(getStatusLabel("bothDeleted")).toBe("!");
  });
});
