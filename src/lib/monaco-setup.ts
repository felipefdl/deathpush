import { loader } from "@monaco-editor/react";
import * as monaco from "monaco-editor";
import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import jsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";
import cssWorker from "monaco-editor/esm/vs/language/css/css.worker?worker";
import htmlWorker from "monaco-editor/esm/vs/language/html/html.worker?worker";
import tsWorker from "monaco-editor/esm/vs/language/typescript/ts.worker?worker";

self.MonacoEnvironment = {
  getWorker(_, label) {
    if (label === "json") return new jsonWorker();
    if (label === "css" || label === "scss" || label === "less") return new cssWorker();
    if (label === "html" || label === "handlebars" || label === "razor") return new htmlWorker();
    if (label === "typescript" || label === "javascript") return new tsWorker();
    return new editorWorker();
  },
};

import { conf as tomlConf, language as tomlLanguage } from "./languages/toml";
import { conf as justfileConf, language as justfileLanguage } from "./languages/justfile";
import { conf as dotenvConf, language as dotenvLanguage } from "./languages/dotenv";

loader.config({ monaco });

monaco.languages.register({ id: "toml", extensions: [".toml"], aliases: ["TOML"] });
monaco.languages.setMonarchTokensProvider("toml", tomlLanguage);
monaco.languages.setLanguageConfiguration("toml", tomlConf);

monaco.languages.register({ id: "justfile", filenames: ["justfile", "Justfile"], aliases: ["Justfile"] });
monaco.languages.setMonarchTokensProvider("justfile", justfileLanguage);
monaco.languages.setLanguageConfiguration("justfile", justfileConf);

monaco.languages.register({ id: "dotenv", filenames: [".env"], aliases: ["dotenv"] });
monaco.languages.setMonarchTokensProvider("dotenv", dotenvLanguage);
monaco.languages.setLanguageConfiguration("dotenv", dotenvConf);

monaco.languages.typescript.typescriptDefaults.setDiagnosticsOptions({
  noSemanticValidation: true,
  noSyntaxValidation: true,
  noSuggestionDiagnostics: true,
});
monaco.languages.typescript.typescriptDefaults.setCompilerOptions({
  target: monaco.languages.typescript.ScriptTarget.Latest,
  allowNonTsExtensions: true,
  noLib: true,
});

monaco.languages.typescript.javascriptDefaults.setDiagnosticsOptions({
  noSemanticValidation: true,
  noSyntaxValidation: true,
  noSuggestionDiagnostics: true,
});
monaco.languages.typescript.javascriptDefaults.setCompilerOptions({
  target: monaco.languages.typescript.ScriptTarget.Latest,
  allowNonTsExtensions: true,
  noLib: true,
});

monaco.languages.json.jsonDefaults.setDiagnosticsOptions({
  validate: false,
});

monaco.languages.css.cssDefaults.setOptions({ validate: false });
monaco.languages.css.scssDefaults.setOptions({ validate: false });
monaco.languages.css.lessDefaults.setOptions({ validate: false });

monaco.languages.html.htmlDefaults.setModeConfiguration({
  diagnostics: false,
});
