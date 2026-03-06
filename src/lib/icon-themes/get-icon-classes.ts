export type FileIconKind = "file" | "folder" | "root-folder";

const selectorEscape = (str: string): string => str.replace(/[\s]/g, "/");

const EXT_TO_LANGUAGE: Record<string, string> = {
  ts: "typescript",
  tsx: "typescriptreact",
  js: "javascript",
  jsx: "javascriptreact",
  mjs: "javascript",
  cjs: "javascript",
  mts: "typescript",
  cts: "typescript",
  py: "python",
  pyw: "python",
  rb: "ruby",
  go: "go",
  java: "java",
  c: "c",
  h: "c",
  cpp: "cpp",
  cc: "cpp",
  cxx: "cpp",
  hpp: "cpp",
  hxx: "cpp",
  cs: "csharp",
  php: "php",
  swift: "swift",
  kt: "kotlin",
  kts: "kotlin",
  scala: "scala",
  rs: "rust",
  r: "r",
  R: "r",
  lua: "lua",
  pl: "perl",
  pm: "perl",
  sh: "shellscript",
  bash: "shellscript",
  zsh: "shellscript",
  fish: "shellscript",
  ps1: "powershell",
  html: "html",
  htm: "html",
  css: "css",
  scss: "scss",
  sass: "sass",
  less: "less",
  json: "json",
  jsonc: "jsonc",
  xml: "xml",
  xsl: "xsl",
  yaml: "yaml",
  yml: "yaml",
  toml: "toml",
  ini: "ini",
  cfg: "ini",
  md: "markdown",
  mdx: "mdx",
  sql: "sql",
  graphql: "graphql",
  gql: "graphql",
  vue: "vue",
  svelte: "svelte",
  dart: "dart",
  ex: "elixir",
  exs: "elixir",
  erl: "erlang",
  hrl: "erlang",
  hs: "haskell",
  lhs: "haskell",
  clj: "clojure",
  cljs: "clojure",
  fs: "fsharp",
  fsx: "fsharp",
  ml: "ocaml",
  mli: "ocaml",
  tf: "terraform",
  tfvars: "terraform",
  dockerfile: "dockerfile",
  proto: "proto3",
  tex: "latex",
  bib: "bibtex",
  zig: "zig",
  nim: "nim",
  v: "v",
  sol: "solidity",
  asm: "asm",
  s: "asm",
  bat: "bat",
  cmd: "bat",
  ps: "powershell",
  psm1: "powershell",
  m: "objective-c",
  mm: "objective-cpp",
  jl: "julia",
  cr: "crystal",
  elm: "elm",
  purs: "purescript",
  wasm: "wasm",
  wat: "wasm",
  prisma: "prisma",
  astro: "astro",
  bicep: "bicep",
  nix: "nix",
};

export const getFileIconClasses = (filePath: string, kind: FileIconKind = "file"): string => {
  const parts = filePath.split("/");
  const rawName = parts[parts.length - 1] || filePath;
  const name = selectorEscape(rawName.toLowerCase());

  if (kind === "root-folder") {
    return `rootfolder-icon ${name}-root-name-folder-icon`;
  }

  if (kind === "folder") {
    return `folder-icon ${name}-name-folder-icon`;
  }

  const classes: string[] = ["file-icon"];

  if (!name) return classes.join(" ");

  classes.push(`${name}-name-file-icon`);
  classes.push("name-file-icon");

  if (name.length <= 255) {
    const dotSegments = name.split(".");
    for (let i = 1; i < dotSegments.length; i++) {
      classes.push(`${dotSegments.slice(i).join(".")}-ext-file-icon`);
    }
  }

  classes.push("ext-file-icon");

  const dotIdx = name.lastIndexOf(".");
  if (dotIdx >= 0) {
    const ext = name.substring(dotIdx + 1);
    const langId = EXT_TO_LANGUAGE[ext];
    if (langId) {
      classes.push(`${selectorEscape(langId)}-lang-file-icon`);
    }
  }

  return classes.join(" ");
};
