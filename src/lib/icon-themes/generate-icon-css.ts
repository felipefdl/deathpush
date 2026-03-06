import type { IconThemeJson, IconsAssociation, IconDefinition } from "./icon-theme-types";

const FONT_COLOR_RE = /^#[0-9a-fA-F]{3,8}$/;
const FONT_SIZE_RE = /^[0-9]+(%|px|em|rem)$/;
const EM_QUAD = "'\\2001'";

const selectorEscape = (str: string): string => str.replace(/[\s]/g, "/");

const cssEscapeClassName = (str: string): string =>
  str.replace(/[^a-zA-Z0-9_-]/g, (c) => `\\${c}`);

const cssClassName = (str: string): string =>
  cssEscapeClassName(selectorEscape(str));

const cssUrl = (path: string): string => `url('${path.replace(/'/g, "\\'")}')`;

interface GenerateResult {
  cssContent: string;
  hasFileIcons: boolean;
  hasFolderIcons: boolean;
  hidesExplorerArrows: boolean;
}

export const generateIconThemeCss = (json: IconThemeJson, assetBasePath: string): GenerateResult => {
  const result = { cssContent: "", hasFileIcons: false, hasFolderIcons: false, hidesExplorerArrows: !!json.hidesExplorerArrows };

  if (!json.iconDefinitions) return result;

  const selectorsByDefId: Record<string, string[]> = {};

  const resolvePath = (iconPath: string): string => {
    if (iconPath.startsWith("/") || iconPath.startsWith("http")) return iconPath;
    return `${assetBasePath}/${iconPath}`;
  };

  const addSelector = (selector: string, defId: string) => {
    if (!defId) return;
    if (!selectorsByDefId[defId]) selectorsByDefId[defId] = [];
    selectorsByDefId[defId].push(selector);
  };

  const collectSelectors = (assoc: IconsAssociation | undefined, baseClass?: string) => {
    if (!assoc) return;

    let qualifier = ".show-file-icons";
    if (baseClass) qualifier = `${baseClass} ${qualifier}`;

    if (assoc.file) {
      addSelector(`${qualifier} .file-icon::before`, assoc.file);
      result.hasFileIcons = true;
    }

    if (assoc.folder) {
      addSelector(`${qualifier} .folder-icon::before`, assoc.folder);
      result.hasFolderIcons = true;
    }

    if (assoc.folderExpanded) {
      addSelector(`${qualifier} .folder-icon.folder-icon-expanded::before`, assoc.folderExpanded);
      result.hasFolderIcons = true;
    }

    const rootFolder = assoc.rootFolder || assoc.folder;
    if (rootFolder) {
      addSelector(`${qualifier} .rootfolder-icon::before`, rootFolder);
      result.hasFolderIcons = true;
    }

    const rootFolderExpanded = assoc.rootFolderExpanded || assoc.folderExpanded;
    if (rootFolderExpanded) {
      addSelector(`${qualifier} .rootfolder-icon.folder-icon-expanded::before`, rootFolderExpanded);
      result.hasFolderIcons = true;
    }

    if (assoc.folderNames) {
      for (const key in assoc.folderNames) {
        const name = cssClassName(key.toLowerCase());
        addSelector(`${qualifier} .${name}-name-folder-icon.folder-icon::before`, assoc.folderNames[key]);
        result.hasFolderIcons = true;
      }
    }

    if (assoc.folderNamesExpanded) {
      for (const key in assoc.folderNamesExpanded) {
        const name = cssClassName(key.toLowerCase());
        addSelector(`${qualifier} .${name}-name-folder-icon.folder-icon.folder-icon-expanded::before`, assoc.folderNamesExpanded[key]);
        result.hasFolderIcons = true;
      }
    }

    if (assoc.rootFolderNames) {
      for (const key in assoc.rootFolderNames) {
        const name = cssClassName(key.toLowerCase());
        addSelector(`${qualifier} .${name}-root-name-folder-icon.rootfolder-icon::before`, assoc.rootFolderNames[key]);
        result.hasFolderIcons = true;
      }
    }

    if (assoc.rootFolderNamesExpanded) {
      for (const key in assoc.rootFolderNamesExpanded) {
        const name = cssClassName(key.toLowerCase());
        addSelector(`${qualifier} .${name}-root-name-folder-icon.rootfolder-icon.folder-icon-expanded::before`, assoc.rootFolderNamesExpanded[key]);
        result.hasFolderIcons = true;
      }
    }

    if (assoc.languageIds) {
      for (const langId in assoc.languageIds) {
        const escaped = cssClassName(langId);
        addSelector(`${qualifier} .${escaped}-lang-file-icon.file-icon::before`, assoc.languageIds[langId]);
        result.hasFileIcons = true;
      }
    }

    if (assoc.fileExtensions) {
      for (const key in assoc.fileExtensions) {
        const rawName = selectorEscape(key.toLowerCase());
        const segments = rawName.split(".");
        const selectorParts = segments.map((_, i) => {
          const joined = segments.slice(i).join(".");
          return `.${cssEscapeClassName(joined)}-ext-file-icon`;
        });
        selectorParts.push(".ext-file-icon");
        addSelector(`${qualifier} ${selectorParts.join("")}.file-icon::before`, assoc.fileExtensions[key]);
        result.hasFileIcons = true;
      }
    }

    if (assoc.fileNames) {
      for (const key in assoc.fileNames) {
        const rawFileName = selectorEscape(key.toLowerCase());
        const escapedFileName = cssEscapeClassName(rawFileName);
        const parts: string[] = [`.${escapedFileName}-name-file-icon`, ".name-file-icon"];
        const segments = rawFileName.split(".");
        if (segments.length > 1) {
          for (let i = 1; i < segments.length; i++) {
            const joined = segments.slice(i).join(".");
            parts.push(`.${cssEscapeClassName(joined)}-ext-file-icon`);
          }
          parts.push(".ext-file-icon");
        }
        addSelector(`${qualifier} ${parts.join("")}.file-icon::before`, assoc.fileNames[key]);
        result.hasFileIcons = true;
      }
    }
  };

  collectSelectors(json);
  collectSelectors(json.light, ".vs");
  collectSelectors(json.highContrast, ".hc-black");
  collectSelectors(json.highContrast, ".hc-light");

  if (!result.hasFileIcons && !result.hasFolderIcons) return result;

  const cssRules: string[] = [];

  const fonts = json.fonts;
  const fontSizes = new Map<string, string>();
  if (Array.isArray(fonts) && fonts.length > 0) {
    const defaultFontSize = normalizeFontSize(fonts[0].size) || "150%";

    for (const font of fonts) {
      const fontSrcs = font.src.map(
        (s) => `${cssUrl(resolvePath(s.path))} format('${s.format}')`,
      );
      cssRules.push(
        `@font-face { src: ${fontSrcs.join(", ")}; font-family: '${font.id}'; font-weight: ${font.weight || "normal"}; font-style: ${font.style || "normal"}; font-display: block; }`,
      );
      const fontSize = normalizeFontSize(font.size);
      if (fontSize && fontSize !== defaultFontSize) {
        fontSizes.set(font.id, fontSize);
      }
    }

    cssRules.push(
      `.show-file-icons .file-icon::before, .show-file-icons .folder-icon::before, .show-file-icons .rootfolder-icon::before { font-family: '${fonts[0].id}'; font-size: ${defaultFontSize}; }`,
    );
  }

  for (const defId in selectorsByDefId) {
    const selectors = selectorsByDefId[defId];
    const definition: IconDefinition | undefined = json.iconDefinitions[defId];
    if (!definition) continue;

    if (definition.iconPath) {
      cssRules.push(
        `${selectors.join(", ")} { content: ${EM_QUAD}; background-image: ${cssUrl(resolvePath(definition.iconPath))}; }`,
      );
    } else if (definition.fontCharacter || definition.fontColor) {
      const body: string[] = [];
      if (definition.fontColor && FONT_COLOR_RE.test(definition.fontColor)) {
        body.push(`color: ${definition.fontColor};`);
      }
      if (definition.fontCharacter) {
        body.push(`content: '${definition.fontCharacter}';`);
      }
      const fontSize = definition.fontSize ?? (definition.fontId ? fontSizes.get(definition.fontId) : undefined);
      if (fontSize && FONT_SIZE_RE.test(fontSize)) {
        body.push(`font-size: ${fontSize};`);
      }
      if (definition.fontId) {
        body.push(`font-family: '${definition.fontId}';`);
      }
      cssRules.push(`${selectors.join(", ")} { ${body.join(" ")} }`);
    }
  }

  result.cssContent = cssRules.join("\n");
  return result;
};

const normalizeFontSize = (size: string | undefined): string | undefined => {
  if (!size) return undefined;
  if (size.endsWith("px")) {
    const value = parseInt(size, 10);
    if (!isNaN(value)) return `${Math.round((value / 13) * 100)}%`;
  }
  return size;
};
