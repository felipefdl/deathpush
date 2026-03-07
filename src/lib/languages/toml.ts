import type { languages } from "monaco-editor";

export const conf: languages.LanguageConfiguration = {
  comments: {
    lineComment: "#",
  },
  brackets: [
    ["{", "}"],
    ["[", "]"],
  ],
  autoClosingPairs: [
    { open: "{", close: "}" },
    { open: "[", close: "]" },
    { open: '"', close: '"' },
    { open: "'", close: "'" },
  ],
  surroundingPairs: [
    { open: "{", close: "}" },
    { open: "[", close: "]" },
    { open: '"', close: '"' },
    { open: "'", close: "'" },
  ],
};

export const language: languages.IMonarchLanguage = {
  tokenizer: {
    root: [
      [/#.*$/, "comment"],
      [/\[\[/, "metatag", "@arraytable"],
      [/\[/, "metatag", "@table"],
      [/[a-zA-Z_][\w.-]*\s*(?==)/, "key"],
      [/=/, "delimiter"],
      { include: "@value" },
    ],

    table: [
      [/[^\]]+/, "metatag"],
      [/\]/, "metatag", "@pop"],
    ],

    arraytable: [
      [/[^\]]+/, "metatag"],
      [/\]\]/, "metatag", "@pop"],
    ],

    value: [
      [/"""/, "string", "@multilineBasicString"],
      [/'''/, "string", "@multilineLiteralString"],
      [/"/, "string", "@basicString"],
      [/'/, "string", "@literalString"],
      [/true|false/, "keyword"],
      [/\d{4}-\d{2}-\d{2}([T ]\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})?)?/, "number"],
      [/\d{2}:\d{2}:\d{2}(\.\d+)?/, "number"],
      [/[+-]?(0x[0-9a-fA-F_]+|0o[0-7_]+|0b[01_]+|inf|nan)/, "number"],
      [/[+-]?\d[\d_]*(\.\d[\d_]*)?([eE][+-]?\d[\d_]*)?/, "number"],
      [/#.*$/, "comment"],
    ],

    basicString: [
      [/\\[btnfr"\\]|\\u[0-9a-fA-F]{4}|\\U[0-9a-fA-F]{8}/, "string.escape"],
      [/[^"\\]+/, "string"],
      [/"/, "string", "@pop"],
    ],

    literalString: [
      [/[^']+/, "string"],
      [/'/, "string", "@pop"],
    ],

    multilineBasicString: [
      [/\\[btnfr"\\]|\\u[0-9a-fA-F]{4}|\\U[0-9a-fA-F]{8}/, "string.escape"],
      [/"""/, "string", "@pop"],
      [/./, "string"],
    ],

    multilineLiteralString: [
      [/'''/, "string", "@pop"],
      [/./, "string"],
    ],
  },
};
