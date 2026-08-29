import { loader } from "@monaco-editor/react";
import * as monaco from "monaco-editor";
import { css, html, json, typescript } from "monaco-editor";
import editorWorker from "monaco-editor/editor/editor.worker.js?worker";
import jsonWorker from "monaco-editor/language/json/json.worker.js?worker";
import cssWorker from "monaco-editor/language/css/css.worker.js?worker";
import htmlWorker from "monaco-editor/language/html/html.worker.js?worker";
import tsWorker from "monaco-editor/language/typescript/ts.worker.js?worker";

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

typescript.typescriptDefaults.setDiagnosticsOptions({
  noSemanticValidation: true,
  noSyntaxValidation: true,
  noSuggestionDiagnostics: true,
});
typescript.typescriptDefaults.setCompilerOptions({
  target: typescript.ScriptTarget.Latest,
  allowNonTsExtensions: true,
  noLib: true,
});

typescript.javascriptDefaults.setDiagnosticsOptions({
  noSemanticValidation: true,
  noSyntaxValidation: true,
  noSuggestionDiagnostics: true,
});
typescript.javascriptDefaults.setCompilerOptions({
  target: typescript.ScriptTarget.Latest,
  allowNonTsExtensions: true,
  noLib: true,
});

json.jsonDefaults.setDiagnosticsOptions({
  validate: false,
});

css.cssDefaults.setOptions({ validate: false });
css.scssDefaults.setOptions({ validate: false });
css.lessDefaults.setOptions({ validate: false });

html.htmlDefaults.setModeConfiguration({
  diagnostics: false,
});
