import { readFile, readdir, writeFile } from "node:fs/promises";
import { extname, join } from "node:path";
import { fileURLToPath } from "node:url";

const SOURCE_ROOT = fileURLToPath(new URL("../src/", import.meta.url));
const CODICON_SOURCE = new URL("../node_modules/@vscode/codicons/dist/codicon.css", import.meta.url);
const OUTPUT = new URL("../src/styles/codicons.css", import.meta.url);
const COMPUTED_ICON_SAFE_LIST = [
  "add",
  "check",
  "clippy",
  "copy",
  "diff",
  "discard",
  "edit",
  "exclude",
  "files",
  "folder-opened",
  "git-merge",
  "git-pull-request",
  "go-to-file",
  "history",
  "new-file",
  "new-folder",
  "remove",
  "trash",
];

const sourceFiles = async (directory) => {
  const entries = await readdir(directory, { withFileTypes: true });
  const nested = await Promise.all(
    entries.map((entry) => {
      const path = join(directory, entry.name);
      return entry.isDirectory() ? sourceFiles(path) : [path];
    })
  );
  return nested.flat();
};

const usedIcons = new Set(COMPUTED_ICON_SAFE_LIST.map((name) => `codicon-${name}`));
for (const path of await sourceFiles(SOURCE_ROOT)) {
  if (![".ts", ".tsx"].includes(extname(path)) || path.endsWith(".test.ts") || path.endsWith(".test.tsx")) continue;
  const source = await readFile(path, "utf8");
  for (const match of source.matchAll(/\bcodicon-([a-z](?:[a-z0-9-]*[a-z0-9])?)(?![a-z0-9-]|\$\{)/g)) {
    usedIcons.add(`codicon-${match[1]}`);
  }
}

const sourceCss = await readFile(CODICON_SOURCE, "utf8");
const glyphRules = new Map();
for (const line of sourceCss.split("\n")) {
  const match = line.match(/^\.(codicon-[a-z0-9-]+):before \{ content: "(\\[a-f0-9]+)" \}$/);
  if (match) glyphRules.set(match[1], line);
}

const missing = [...usedIcons].filter((name) => !glyphRules.has(name)).sort();
if (missing.length > 0) {
  throw new Error(`Missing Codicon glyphs: ${missing.join(", ")}`);
}

const output = `@font-face {
  font-family: "codicon";
  font-display: block;
  src: url("/codicon/codicon.woff2") format("woff2");
}

.codicon {
  font: normal normal normal 16px/1 codicon;
  display: inline-block;
  text-decoration: none;
  text-rendering: auto;
  text-align: center;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  user-select: none;
}

${[...usedIcons]
  .sort()
  .map((name) => glyphRules.get(name))
  .join("\n")}
`;

await writeFile(OUTPUT, output);
