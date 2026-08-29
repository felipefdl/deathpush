import type { EditorKeymap } from "@pierre/diffs/edit";

// Pierre has no no-op command; later custom bindings win, so this replaces Mac ctrl+k kill-line.
export const pierreEditorKeymap: EditorKeymap = [
  {
    bindings: {
      "ctrl+k": "simplifySelection",
    },
  },
];
