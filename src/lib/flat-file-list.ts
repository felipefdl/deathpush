import type { ResourceGroup, ResourceGroupKind } from "./git-types";

export interface FlatFileEntry {
  path: string;
  groupKind: ResourceGroupKind;
}

export const buildFlatFileList = (groups: ResourceGroup[], filter: string): FlatFileEntry[] => {
  const result: FlatFileEntry[] = [];
  const lower = filter.toLowerCase();
  for (const group of groups) {
    const files = filter
      ? group.files.filter((f) => f.path.toLowerCase().includes(lower))
      : group.files;
    for (const file of files) {
      result.push({ path: file.path, groupKind: group.kind });
    }
  }
  return result;
};
