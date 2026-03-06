import { describe, it, expect } from "vitest";
import { getFileIconClasses } from "./get-icon-classes";

describe("getFileIconClasses", () => {
  describe("file icons", () => {
    it("returns correct classes for a basic file", () => {
      const result = getFileIconClasses("app.ts");
      expect(result).toBe("file-icon app.ts-name-file-icon name-file-icon ts-ext-file-icon ext-file-icon typescript-lang-file-icon");
    });

    it("uses only the last path segment for nested paths", () => {
      const result = getFileIconClasses("src/app.ts");
      expect(result).toBe("file-icon app.ts-name-file-icon name-file-icon ts-ext-file-icon ext-file-icon typescript-lang-file-icon");
    });

    it("generates extension classes for each dot segment with double extension", () => {
      const result = getFileIconClasses("archive.tar.gz");
      expect(result).toContain("tar.gz-ext-file-icon");
      expect(result).toContain("gz-ext-file-icon");
      expect(result).toContain("ext-file-icon");
    });

    it("returns file-icon and name classes for files with no extension", () => {
      const result = getFileIconClasses("Makefile");
      expect(result).toBe("file-icon makefile-name-file-icon name-file-icon ext-file-icon");
    });

    it("returns only file-icon for empty string", () => {
      const result = getFileIconClasses("");
      expect(result).toBe("file-icon");
    });

    it("escapes spaces to forward slashes in file names", () => {
      const result = getFileIconClasses("my file.ts");
      expect(result).toContain("my/file.ts-name-file-icon");
      expect(result).toContain("typescript-lang-file-icon");
    });

    it("handles dotfiles like .gitignore", () => {
      const result = getFileIconClasses(".gitignore");
      expect(result).toContain(".gitignore-name-file-icon");
      expect(result).toContain("gitignore-ext-file-icon");
    });

    it("does not append lang class for unknown extensions", () => {
      const result = getFileIconClasses("file.xyz");
      expect(result).not.toContain("-lang-file-icon");
      expect(result).toContain("xyz-ext-file-icon");
    });

    it("generates correct extension chain for multiple dots", () => {
      const result = getFileIconClasses("my.test.spec.ts");
      expect(result).toContain("test.spec.ts-ext-file-icon");
      expect(result).toContain("spec.ts-ext-file-icon");
      expect(result).toContain("ts-ext-file-icon");
      expect(result).toContain("ext-file-icon");
    });

    it("lowercases the file name", () => {
      const result = getFileIconClasses("README.MD");
      expect(result).toContain("readme.md-name-file-icon");
      expect(result).toContain("markdown-lang-file-icon");
    });

    it("defaults kind to file when not specified", () => {
      const result = getFileIconClasses("index.js");
      expect(result).toContain("file-icon");
      expect(result).toContain("javascript-lang-file-icon");
    });

    it("handles deeply nested paths", () => {
      const result = getFileIconClasses("a/b/c/d/main.rs");
      expect(result).toContain("main.rs-name-file-icon");
      expect(result).toContain("rust-lang-file-icon");
    });
  });

  describe("folder icons", () => {
    it("returns folder-icon and name-folder-icon for a basic folder", () => {
      const result = getFileIconClasses("src", "folder");
      expect(result).toBe("folder-icon src-name-folder-icon");
    });

    it("uses only the last path segment for nested folder paths", () => {
      const result = getFileIconClasses("path/to/src", "folder");
      expect(result).toBe("folder-icon src-name-folder-icon");
    });

    it("lowercases folder names", () => {
      const result = getFileIconClasses("MyFolder", "folder");
      expect(result).toBe("folder-icon myfolder-name-folder-icon");
    });
  });

  describe("root folder icons", () => {
    it("returns rootfolder-icon and root-name-folder-icon for a basic root folder", () => {
      const result = getFileIconClasses("project", "root-folder");
      expect(result).toBe("rootfolder-icon project-root-name-folder-icon");
    });

    it("uses only the last path segment for nested root folder paths", () => {
      const result = getFileIconClasses("root/project", "root-folder");
      expect(result).toBe("rootfolder-icon project-root-name-folder-icon");
    });

    it("lowercases root folder names", () => {
      const result = getFileIconClasses("RootProject", "root-folder");
      expect(result).toBe("rootfolder-icon rootproject-root-name-folder-icon");
    });
  });

  describe("language ID mapping", () => {
    it("maps ts to typescript", () => {
      expect(getFileIconClasses("a.ts")).toContain("typescript-lang-file-icon");
    });

    it("maps tsx to typescriptreact", () => {
      expect(getFileIconClasses("a.tsx")).toContain("typescriptreact-lang-file-icon");
    });

    it("maps js to javascript", () => {
      expect(getFileIconClasses("a.js")).toContain("javascript-lang-file-icon");
    });

    it("maps py to python", () => {
      expect(getFileIconClasses("a.py")).toContain("python-lang-file-icon");
    });

    it("maps rs to rust", () => {
      expect(getFileIconClasses("a.rs")).toContain("rust-lang-file-icon");
    });

    it("maps go to go", () => {
      expect(getFileIconClasses("a.go")).toContain("go-lang-file-icon");
    });

    it("maps sh to shellscript", () => {
      expect(getFileIconClasses("a.sh")).toContain("shellscript-lang-file-icon");
    });

    it("maps json to json", () => {
      expect(getFileIconClasses("a.json")).toContain("json-lang-file-icon");
    });

    it("maps md to markdown", () => {
      expect(getFileIconClasses("a.md")).toContain("markdown-lang-file-icon");
    });

    it("maps css to css", () => {
      expect(getFileIconClasses("a.css")).toContain("css-lang-file-icon");
    });

    it("maps yaml to yaml", () => {
      expect(getFileIconClasses("a.yaml")).toContain("yaml-lang-file-icon");
    });

    it("does not add lang class for unknown extension", () => {
      expect(getFileIconClasses("a.unknownext")).not.toContain("-lang-file-icon");
    });
  });
});
