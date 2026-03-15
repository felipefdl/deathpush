// editor-bridge.js -- Bridge between Swift WKWebView and Monaco Editor (single file viewer)
// Communicates with Swift via window.webkit.messageHandlers.editorBridge

(function () {
  "use strict";

  let editor = null;
  let currentThemeName = null;

  // File extension to Monaco language mapping
  const EXT_TO_LANG = {
    js: "javascript", jsx: "javascript", mjs: "javascript", cjs: "javascript",
    ts: "typescript", tsx: "typescript", mts: "typescript", cts: "typescript",
    json: "json", jsonc: "json",
    html: "html", htm: "html",
    css: "css", scss: "scss", less: "less",
    md: "markdown", mdx: "markdown",
    py: "python", pyw: "python",
    rb: "ruby",
    rs: "rust",
    go: "go",
    java: "java",
    kt: "kotlin", kts: "kotlin",
    swift: "swift",
    c: "c", h: "c",
    cpp: "cpp", cc: "cpp", cxx: "cpp", hpp: "cpp", hxx: "cpp",
    cs: "csharp",
    php: "php",
    sh: "shell", bash: "shell", zsh: "shell",
    sql: "sql",
    xml: "xml", xsl: "xml", xsd: "xml", svg: "xml", plist: "xml",
    yaml: "yaml", yml: "yaml",
    toml: "toml",
    dockerfile: "dockerfile",
    graphql: "graphql", gql: "graphql",
    lua: "lua",
    r: "r",
    dart: "dart",
    scala: "scala",
    clj: "clojure", cljs: "clojure", cljc: "clojure",
    ex: "elixir", exs: "elixir",
    erl: "erlang", hrl: "erlang",
    hs: "haskell",
    vim: "vim",
    ini: "ini", cfg: "ini",
  };

  // Special filename mappings
  const FILENAME_TO_LANG = {
    justfile: "justfile",
    Justfile: "justfile",
    ".env": "dotenv",
    Dockerfile: "dockerfile",
    Makefile: "makefile",
    Rakefile: "ruby",
    Gemfile: "ruby",
    Podfile: "ruby",
  };

  function getLanguageForPath(filePath) {
    if (!filePath) return "plaintext";
    const fileName = filePath.split("/").pop();
    if (FILENAME_TO_LANG[fileName]) return FILENAME_TO_LANG[fileName];
    if (fileName.startsWith(".env")) return "dotenv";
    const ext = fileName.split(".").pop().toLowerCase();
    return EXT_TO_LANG[ext] || "plaintext";
  }

  // Register custom languages (TOML, Justfile, dotenv)
  function registerCustomLanguages(monaco) {
    // TOML
    monaco.languages.register({ id: "toml", extensions: [".toml"] });
    monaco.languages.setMonarchTokensProvider("toml", {
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
    });

    // Justfile
    monaco.languages.register({ id: "justfile", filenames: ["justfile", "Justfile"] });
    monaco.languages.setMonarchTokensProvider("justfile", {
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
          [/[a-zA-Z_][\w-]*/, {
            cases: {
              "@keywords": "keyword",
              "@builtins": "predefined",
              "@default": "identifier",
            },
          }],
          [/[0-9]+/, "number"],
        ],
        interpolation: [
          [/\}\}/, "delimiter.curly", "@pop"],
          [/[a-zA-Z_][\w-]*/, {
            cases: {
              "@builtins": "predefined",
              "@default": "variable",
            },
          }],
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
    });

    // dotenv
    monaco.languages.register({ id: "dotenv", filenames: [".env"] });
    monaco.languages.setMonarchTokensProvider("dotenv", {
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
    });
  }

  // Disable noisy language features
  function disableDiagnostics(monaco) {
    const jsDefaults = monaco.languages.typescript.javascriptDefaults;
    const tsDefaults = monaco.languages.typescript.typescriptDefaults;
    const opts = { noSemanticValidation: true, noSyntaxValidation: true, noSuggestionDiagnostics: true };
    jsDefaults.setDiagnosticsOptions(opts);
    tsDefaults.setDiagnosticsOptions(opts);
    monaco.languages.json?.jsonDefaults?.setDiagnosticsOptions?.({ validate: false });
    monaco.languages.css?.cssDefaults?.setDiagnosticsOptions?.({ validate: false });
    monaco.languages.html?.htmlDefaults?.setOptions?.({ validate: false });
  }

  // Send message to Swift
  function sendToSwift(type, payload) {
    if (window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.editorBridge) {
      window.webkit.messageHandlers.editorBridge.postMessage({ type, ...payload });
    }
  }

  // Initialize Monaco
  require.config({ paths: { vs: "vs" } });
  require(["vs/editor/editor.main"], function (monaco) {
    registerCustomLanguages(monaco);
    disableDiagnostics(monaco);

    editor = monaco.editor.create(document.getElementById("editor"), {
      value: "",
      language: "plaintext",
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
      find: {
        addExtraSpaceOnTop: false,
        autoFindInSelection: "never",
        seedSearchStringFromSelection: "always",
      },
      fontSize: 13,
      fontFamily: "SF Mono, Menlo, Monaco, monospace",
      lineHeight: 20,
      tabSize: 2,
      wordWrap: "off",
      renderWhitespace: "none",
      readOnly: true,
      quickSuggestions: false,
      parameterHints: { enabled: false },
      suggestOnTriggerCharacters: false,
      codeLens: false,
      stickyScroll: { enabled: false },
      hover: { enabled: false },
      inlayHints: { enabled: "off" },
      glyphMargin: false,
      lineNumbersMinChars: 3,
      folding: true,
      matchBrackets: "always",
      occurrencesHighlight: "off",
      selectionHighlight: false,
      links: false,
      lightbulb: { enabled: "off" },
      bracketPairColorization: { enabled: true },
      automaticLayout: true,
    });

    editor.onDidChangeCursorPosition(function(e) {
      sendToSwift("cursorChanged", { line: e.position.lineNumber });
    });

    editor.onDidChangeModelContent(function() {
      sendToSwift("contentChanged", {});
    });

    editor.addAction({
      id: "deathpush.save",
      label: "Save File",
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS],
      run: function(ed) {
        sendToSwift("save", { content: ed.getValue() });
      }
    });

    sendToSwift("ready", {});
  });

  // --- Public API called from Swift via evaluateJavaScript ---

  window.setContent = function (content) {
    if (!editor) return;
    require(["vs/editor/editor.main"], function (monaco) {
      const lang = content.language || getLanguageForPath(content.path);
      const model = editor.getModel();
      if (model) {
        monaco.editor.setModelLanguage(model, lang);
        model.setValue(content.content || "");
      }
    });
  };

  window.setTheme = function (themeData) {
    if (!themeData) return;
    require(["vs/editor/editor.main"], function (monaco) {
      const themeName = themeData.name || "custom-theme";
      const base = themeData.type === "light" ? "vs" : themeData.type === "hcDark" ? "hc-black" : themeData.type === "hcLight" ? "hc-light" : "vs-dark";
      monaco.editor.defineTheme(themeName, {
        base: base,
        inherit: true,
        rules: (themeData.tokenColors || []).flatMap(function (tc) {
          const scopes = Array.isArray(tc.scope) ? tc.scope : (tc.scope ? [tc.scope] : []);
          return scopes.map(function (scope) {
            const rule = { token: scope };
            if (tc.settings.foreground) rule.foreground = tc.settings.foreground.replace("#", "");
            if (tc.settings.background) rule.background = tc.settings.background.replace("#", "");
            if (tc.settings.fontStyle) rule.fontStyle = tc.settings.fontStyle;
            return rule;
          });
        }),
        colors: themeData.colors || {},
      });
      monaco.editor.setTheme(themeName);
      currentThemeName = themeName;

      // Update background to match theme
      const bg = themeData.colors && themeData.colors["editor.background"];
      if (bg) {
        document.body.style.background = bg;
      }
    });
  };

  window.setEditable = function (editable) {
    if (!editor) return;
    editor.updateOptions({ readOnly: !editable });
  };

  window.setEditorOptions = function (opts) {
    if (!editor) return;
    editor.updateOptions(opts);
  };

  window.revealLine = function (line) {
    if (!editor) return;
    editor.revealLineInCenter(line);
    editor.setPosition({ lineNumber: line, column: 1 });
  };
})();
