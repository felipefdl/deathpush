import { describe, it, expect } from "vitest";
import { generateIconThemeCss } from "./generate-icon-css";
import type { IconThemeJson } from "./icon-theme-types";

const BASE_PATH = "/assets/icons";

const minimal = (overrides: Partial<IconThemeJson> = {}): IconThemeJson => ({
  iconDefinitions: {},
  ...overrides,
});

describe("generateIconThemeCss", () => {
  describe("empty and basic", () => {
    it("returns empty cssContent for empty iconDefinitions", () => {
      const result = generateIconThemeCss(minimal(), BASE_PATH);
      expect(result.cssContent).toBe("");
      expect(result.hasFileIcons).toBe(false);
      expect(result.hasFolderIcons).toBe(false);
    });

    it("generates background-image rule for iconPath definition", () => {
      const result = generateIconThemeCss(minimal({
        iconDefinitions: { _file: { iconPath: "icons/file.svg" } },
        file: "_file",
      }), BASE_PATH);
      expect(result.cssContent).toContain("background-image");
      expect(result.cssContent).toContain(`${BASE_PATH}/icons/file.svg`);
    });

    it("generates content rule for fontCharacter definition", () => {
      const result = generateIconThemeCss(minimal({
        iconDefinitions: { _file: { fontCharacter: "\\E001" } },
        file: "_file",
      }), BASE_PATH);
      expect(result.cssContent).toContain("content: '\\E001'");
    });

    it("includes color for valid hex fontColor", () => {
      const result = generateIconThemeCss(minimal({
        iconDefinitions: { _file: { fontCharacter: "\\E001", fontColor: "#FF0000" } },
        file: "_file",
      }), BASE_PATH);
      expect(result.cssContent).toContain("color: #FF0000;");
    });

    it("excludes color for invalid fontColor", () => {
      const result = generateIconThemeCss(minimal({
        iconDefinitions: { _file: { fontCharacter: "\\E001", fontColor: "notahex" } },
        file: "_file",
      }), BASE_PATH);
      expect(result.cssContent).not.toContain("color:");
    });
  });

  describe("font size through font definitions", () => {
    it("converts px size to percentage", () => {
      const result = generateIconThemeCss(minimal({
        fonts: [{ id: "testfont", src: [{ path: "font.woff2", format: "woff2" }], size: "26px" }],
        iconDefinitions: { _file: { iconPath: "file.svg" } },
        file: "_file",
      }), BASE_PATH);
      expect(result.cssContent).toContain("font-size: 200%");
    });

    it("passes through percentage size unchanged", () => {
      const result = generateIconThemeCss(minimal({
        fonts: [{ id: "testfont", src: [{ path: "font.woff2", format: "woff2" }], size: "120%" }],
        iconDefinitions: { _file: { iconPath: "file.svg" } },
        file: "_file",
      }), BASE_PATH);
      expect(result.cssContent).toContain("font-size: 120%");
    });

    it("works when fonts array is missing", () => {
      const result = generateIconThemeCss(minimal({
        iconDefinitions: { _file: { iconPath: "file.svg" } },
        file: "_file",
      }), BASE_PATH);
      expect(result.cssContent).toContain("background-image");
    });
  });

  describe("selectors", () => {
    it("creates .file-icon selector for file association", () => {
      const result = generateIconThemeCss(minimal({
        iconDefinitions: { _file: { iconPath: "file.svg" } },
        file: "_file",
      }), BASE_PATH);
      expect(result.cssContent).toContain(".show-file-icons .file-icon::before");
    });

    it("creates .folder-icon selector for folder association", () => {
      const result = generateIconThemeCss(minimal({
        iconDefinitions: { _folder: { iconPath: "folder.svg" } },
        folder: "_folder",
      }), BASE_PATH);
      expect(result.cssContent).toContain(".show-file-icons .folder-icon::before");
    });

    it("creates .folder-icon-expanded selector for folderExpanded association", () => {
      const result = generateIconThemeCss(minimal({
        iconDefinitions: { _folderOpen: { iconPath: "folder-open.svg" } },
        folderExpanded: "_folderOpen",
      }), BASE_PATH);
      expect(result.cssContent).toContain(".folder-icon.folder-icon-expanded::before");
    });

    it("creates ext-file-icon chain for fileExtensions", () => {
      const result = generateIconThemeCss(minimal({
        iconDefinitions: { _ts: { iconPath: "ts.svg" } },
        fileExtensions: { ts: "_ts" },
      }), BASE_PATH);
      expect(result.cssContent).toContain(".ts-ext-file-icon");
      expect(result.cssContent).toContain(".ext-file-icon");
      expect(result.cssContent).toContain(".file-icon::before");
    });

    it("creates name-file-icon chain for fileNames", () => {
      const result = generateIconThemeCss(minimal({
        iconDefinitions: { _pkg: { iconPath: "pkg.svg" } },
        fileNames: { "package.json": "_pkg" },
      }), BASE_PATH);
      expect(result.cssContent).toContain("package\\.json-name-file-icon");
      expect(result.cssContent).toContain(".name-file-icon");
    });

    it("creates lang-file-icon selector for languageIds", () => {
      const result = generateIconThemeCss(minimal({
        iconDefinitions: { _ts: { iconPath: "ts.svg" } },
        languageIds: { typescript: "_ts" },
      }), BASE_PATH);
      expect(result.cssContent).toContain(".typescript-lang-file-icon.file-icon::before");
    });

    it("creates name-folder-icon selector for folderNames", () => {
      const result = generateIconThemeCss(minimal({
        iconDefinitions: { _src: { iconPath: "src.svg" } },
        folderNames: { src: "_src" },
      }), BASE_PATH);
      expect(result.cssContent).toContain(".src-name-folder-icon.folder-icon::before");
    });
  });

  describe("light and high contrast variants", () => {
    it("adds .vs qualifier for light association", () => {
      const result = generateIconThemeCss(minimal({
        iconDefinitions: { _file: { iconPath: "file.svg" }, _fileLight: { iconPath: "file-light.svg" } },
        file: "_file",
        light: { file: "_fileLight" },
      }), BASE_PATH);
      expect(result.cssContent).toContain(".vs .show-file-icons .file-icon::before");
    });

    it("adds .hc-black qualifier for highContrast association", () => {
      const result = generateIconThemeCss(minimal({
        iconDefinitions: { _file: { iconPath: "file.svg" }, _fileHc: { iconPath: "file-hc.svg" } },
        file: "_file",
        highContrast: { file: "_fileHc" },
      }), BASE_PATH);
      expect(result.cssContent).toContain(".hc-black .show-file-icons .file-icon::before");
    });
  });

  describe("flags", () => {
    it("sets hasFileIcons true when file definition is present", () => {
      const result = generateIconThemeCss(minimal({
        iconDefinitions: { _file: { iconPath: "file.svg" } },
        file: "_file",
      }), BASE_PATH);
      expect(result.hasFileIcons).toBe(true);
    });

    it("sets hasFolderIcons true when folder definition is present", () => {
      const result = generateIconThemeCss(minimal({
        iconDefinitions: { _folder: { iconPath: "folder.svg" } },
        folder: "_folder",
      }), BASE_PATH);
      expect(result.hasFolderIcons).toBe(true);
    });

    it("passes through hidesExplorerArrows", () => {
      const result = generateIconThemeCss(minimal({
        iconDefinitions: { _file: { iconPath: "file.svg" } },
        file: "_file",
        hidesExplorerArrows: true,
      }), BASE_PATH);
      expect(result.hidesExplorerArrows).toBe(true);
    });
  });

  describe("font-face", () => {
    it("generates @font-face rule for font definition", () => {
      const result = generateIconThemeCss(minimal({
        fonts: [{ id: "myicons", src: [{ path: "icons.woff2", format: "woff2" }], size: "150%" }],
        iconDefinitions: { _file: { iconPath: "file.svg" } },
        file: "_file",
      }), BASE_PATH);
      expect(result.cssContent).toContain("@font-face");
      expect(result.cssContent).toContain("font-family: 'myicons'");
    });

    it("joins multiple font sources", () => {
      const result = generateIconThemeCss(minimal({
        fonts: [{
          id: "myicons",
          src: [
            { path: "icons.woff2", format: "woff2" },
            { path: "icons.woff", format: "woff" },
          ],
          size: "150%",
        }],
        iconDefinitions: { _file: { iconPath: "file.svg" } },
        file: "_file",
      }), BASE_PATH);
      const fontFace = result.cssContent.split("\n").find((l) => l.startsWith("@font-face"));
      expect(fontFace).toContain("icons.woff2");
      expect(fontFace).toContain("icons.woff");
    });
  });

  describe("path resolution", () => {
    it("prepends basePath for relative paths", () => {
      const result = generateIconThemeCss(minimal({
        iconDefinitions: { _file: { iconPath: "icons/file.svg" } },
        file: "_file",
      }), BASE_PATH);
      expect(result.cssContent).toContain(`${BASE_PATH}/icons/file.svg`);
    });

    it("uses absolute path as-is", () => {
      const result = generateIconThemeCss(minimal({
        iconDefinitions: { _file: { iconPath: "/absolute/file.svg" } },
        file: "_file",
      }), BASE_PATH);
      expect(result.cssContent).toContain("/absolute/file.svg");
      expect(result.cssContent).not.toContain(`${BASE_PATH}//absolute/file.svg`);
    });

    it("uses http URL as-is", () => {
      const result = generateIconThemeCss(minimal({
        iconDefinitions: { _file: { iconPath: "https://cdn.example.com/file.svg" } },
        file: "_file",
      }), BASE_PATH);
      expect(result.cssContent).toContain("https://cdn.example.com/file.svg");
      expect(result.cssContent).not.toContain(`${BASE_PATH}/https://`);
    });
  });
});
