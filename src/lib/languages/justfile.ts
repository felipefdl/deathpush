import type { languages } from "monaco-editor";

export const conf: languages.LanguageConfiguration = {
  comments: {
    lineComment: "#",
  },
  brackets: [
    ["{", "}"],
    ["(", ")"],
    ["[", "]"],
  ],
  autoClosingPairs: [
    { open: "{", close: "}" },
    { open: "(", close: ")" },
    { open: "[", close: "]" },
    { open: '"', close: '"' },
    { open: "'", close: "'" },
    { open: "`", close: "`" },
  ],
  surroundingPairs: [
    { open: "{", close: "}" },
    { open: "(", close: ")" },
    { open: "[", close: "]" },
    { open: '"', close: '"' },
    { open: "'", close: "'" },
    { open: "`", close: "`" },
  ],
};

export const language: languages.IMonarchLanguage = {
  keywords: ["set", "export", "alias", "if", "else", "import", "mod", "true", "false"],

  builtins: [
    "arch", "os", "os_family", "invocation_directory", "justfile", "justfile_directory",
    "just_executable", "just_pid", "env_var", "env_var_or_default", "env", "error",
    "join", "clean", "path_exists", "parent_directory", "file_name", "file_stem",
    "extension", "without_extension", "absolute_path", "uppercase", "lowercase",
    "trim", "trim_start", "trim_end", "replace", "replace_regex", "quote", "shell",
    "sha256", "sha256_file", "uuid", "capitalize", "kebabcase", "lowercamelcase",
    "uppercamelcase", "shoutykebabcase", "shoutysnakecase", "snakecase", "titlecase",
  ],

  tokenizer: {
    root: [
      [/#[!]?.*$/, "comment"],
      [/^\s*@?[a-zA-Z_][\w-]*(?=\s*:)/, "tag"],
      [/\{\{/, "delimiter.curly", "@interpolation"],
      [/`/, "string", "@backtick"],
      [/"/, "string", "@doubleString"],
      [/'/, "string", "@singleString"],
      [/:=/, "delimiter"],
      [/:/, "delimiter"],
      [
        /[a-zA-Z_][\w-]*/,
        {
          cases: {
            "@keywords": "keyword",
            "@builtins": "predefined",
            "@default": "identifier",
          },
        },
      ],
      [/[0-9]+/, "number"],
    ],

    interpolation: [
      [/\}\}/, "delimiter.curly", "@pop"],
      [
        /[a-zA-Z_][\w-]*/,
        {
          cases: {
            "@builtins": "predefined",
            "@default": "variable",
          },
        },
      ],
      [/"/, "string", "@doubleString"],
      [/'/, "string", "@singleString"],
      [/[0-9]+/, "number"],
      [/[+\-*/()]/, "delimiter"],
    ],

    backtick: [
      [/[^`]+/, "string"],
      [/`/, "string", "@pop"],
    ],

    doubleString: [
      [/\\[\\"]/, "string.escape"],
      [/[^"\\]+/, "string"],
      [/"/, "string", "@pop"],
    ],

    singleString: [
      [/[^']+/, "string"],
      [/'/, "string", "@pop"],
    ],
  },
};
