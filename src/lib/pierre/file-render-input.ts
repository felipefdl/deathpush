import { areLanguagesAttached, getFiletypeFromFileName } from "@pierre/diffs";

export type PierreFileRenderInput = {
  name: string;
  contents: string;
  cacheKey: string;
  lang?: "text";
};

export const isPierreLanguageReady = (path: string): boolean => {
  const lang = getFiletypeFromFileName(path);
  return lang === "text" || areLanguagesAttached(lang);
};

export const pierreFileRenderInput = (
  path: string,
  contents: string,
  cacheKey: string,
  languageReady: boolean
): PierreFileRenderInput =>
  languageReady ? { name: path, contents, cacheKey } : { name: path, contents, cacheKey, lang: "text" };
