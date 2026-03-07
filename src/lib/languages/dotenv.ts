import type { languages } from "monaco-editor";

export const conf: languages.LanguageConfiguration = {
  comments: {
    lineComment: "#",
  },
  autoClosingPairs: [
    { open: '"', close: '"' },
    { open: "'", close: "'" },
  ],
  surroundingPairs: [
    { open: '"', close: '"' },
    { open: "'", close: "'" },
  ],
};

export const language: languages.IMonarchLanguage = {
  tokenizer: {
    root: [
      [/#.*$/, "comment"],
      [/^export\b/, "keyword"],
      [/^[A-Za-z_][\w.]*/, "variable", "@assignment"],
    ],

    assignment: [
      [/=/, "delimiter", "@value"],
      [/$/, "", "@pop"],
    ],

    value: [
      [/"/, "string", "@doubleString"],
      [/'/, "string", "@singleString"],
      [/\$\{[^}]+\}/, "variable"],
      [/\$[A-Za-z_]\w*/, "variable"],
      [/[^#\s$"'][^#$"']*/, "string"],
      [/#.*$/, "comment", "@pop"],
      [/$/, "", "@popall"],
    ],

    doubleString: [
      [/\$\{[^}]+\}/, "variable"],
      [/\$[A-Za-z_]\w*/, "variable"],
      [/\\[\\"]/, "string.escape"],
      [/[^"\\$]+/, "string"],
      [/"/, "string", "@pop"],
    ],

    singleString: [
      [/[^']+/, "string"],
      [/'/, "string", "@pop"],
    ],
  },
};
