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

  it("returns C for copied", () => {
    expect(getStatusLabel("copied")).toBe("C");
  });

  it("returns U for untracked", () => {
    expect(getStatusLabel("untracked")).toBe("U");
  });

  it("returns ! for ignored", () => {
    expect(getStatusLabel("ignored")).toBe("!");
  });

  it("returns T for typeChanged", () => {
    expect(getStatusLabel("typeChanged")).toBe("T");
  });

  it("returns M for indexModified", () => {
    expect(getStatusLabel("indexModified")).toBe("M");
  });

  it("returns A for indexAdded", () => {
    expect(getStatusLabel("indexAdded")).toBe("A");
  });

  it("returns D for indexDeleted", () => {
    expect(getStatusLabel("indexDeleted")).toBe("D");
  });

  it("returns R for indexRenamed", () => {
    expect(getStatusLabel("indexRenamed")).toBe("R");
  });

  it("returns C for indexCopied", () => {
    expect(getStatusLabel("indexCopied")).toBe("C");
  });

  it("returns A for intentToAdd", () => {
    expect(getStatusLabel("intentToAdd")).toBe("A");
  });

  it("returns R for intentToRename", () => {
    expect(getStatusLabel("intentToRename")).toBe("R");
  });

  it("returns ! for bothDeleted", () => {
    expect(getStatusLabel("bothDeleted")).toBe("!");
  });

  it("returns ! for addedByUs", () => {
    expect(getStatusLabel("addedByUs")).toBe("!");
  });

  it("returns ! for deletedByThem", () => {
    expect(getStatusLabel("deletedByThem")).toBe("!");
  });

  it("returns ! for addedByThem", () => {
    expect(getStatusLabel("addedByThem")).toBe("!");
  });

  it("returns ! for deletedByUs", () => {
    expect(getStatusLabel("deletedByUs")).toBe("!");
  });

  it("returns ! for bothAdded", () => {
    expect(getStatusLabel("bothAdded")).toBe("!");
  });

  it("returns ! for bothModified", () => {
    expect(getStatusLabel("bothModified")).toBe("!");
  });
});
