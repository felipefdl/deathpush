import {
  FileTree,
  prepareFileTreeInput,
  themeToTreeStyles,
  type FileTreeBuiltInIconSet,
  type FileTreeDensityKeyword,
  type FileTreeOptions,
  type GitStatusEntry,
} from "@pierre/trees";
import { createEffect, createSignal, onSettled } from "solid-js";
import { fileTreeClickedFilePath } from "../../lib/explorer-file-activate";
import { throttle } from "../../lib/throttle";
import {
  ancestorDirectoryPaths,
  nextPersistedExpandedPaths,
  restoreExpandedDirectoryPaths,
  restoreSelectedFilePath,
  snapshotExpandedDirectoryPaths,
  type TreeStateModel,
} from "../../lib/tree-state";
import { explorerStore } from "../../stores/explorer-store";
import { themeStore } from "../../stores/theme-store";
import { settingsStore } from "../../stores/settings-store";
import { useStore } from "../../lib/use-store";
import { sameTreePaths } from "../../lib/trees";

type ManagedFileTreeOptions = Omit<
  FileTreeOptions,
  "density" | "gitStatus" | "icons" | "initialExpandedPaths" | "initialSelectedPaths" | "paths" | "preparedInput"
>;

export type FileTreeHostProps = {
  paths: readonly string[];
  gitStatus?: readonly GitStatusEntry[];
  options?: ManagedFileTreeOptions;
  class?: string;
  modelRef?: (model: FileTree | undefined) => void;
  onFileActivate?: (path: string) => void;
  selectedPath?: string | null;
};

const cssPropertyName = (property: string): string =>
  property.startsWith("--") ? property : property.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`);
const asTreeState = (model: FileTree): TreeStateModel => model as unknown as TreeStateModel;

const persistTreeUi = (model: FileTree, paths: readonly string[]): void => {
  const explorer = explorerStore.getState();
  explorer.setTreeExpandedPaths(
    nextPersistedExpandedPaths(explorer.treeExpandedPaths, snapshotExpandedDirectoryPaths(asTreeState(model), paths))
  );
};

const applyPersistedTreeUi = (model: FileTree, selectedPath: string | null): void => {
  const explorer = explorerStore.getState();
  restoreExpandedDirectoryPaths(asTreeState(model), [
    ...explorer.treeExpandedPaths,
    ...ancestorDirectoryPaths(selectedPath ?? ""),
  ]);
  restoreSelectedFilePath(asTreeState(model), selectedPath);
};

export const FileTreeHost = (props: FileTreeHostProps) => {
  const density = useStore(settingsStore, (state) => state.settings.ui.treeDensity);
  const icons = useStore(settingsStore, (state) => state.settings.ui.treeIcons);
  const theme = useStore(themeStore, (state) => state.currentTheme);
  const [ready, setReady] = createSignal(false);
  let container!: HTMLDivElement;
  let model: FileTree | undefined;
  let currentPaths: readonly string[] = [];
  let appliedThemeProperties: string[] = [];

  onSettled(() => {
    setReady(true);
    return () => setReady(false);
  });

  createEffect(
    () => [ready(), density()] as const,
    ([isReady, currentDensity]) => {
      if (!isReady) return;

      const explorer = explorerStore.getState();
      const selectedPath = props.selectedPath ?? null;
      const tree = new FileTree({
        ...props.options,
        preparedInput: prepareFileTreeInput(props.paths),
        density: currentDensity as FileTreeDensityKeyword,
        icons: icons() as FileTreeBuiltInIconSet,
        gitStatus: props.gitStatus,
        initialExpandedPaths: [...explorer.treeExpandedPaths, ...ancestorDirectoryPaths(selectedPath ?? "")],
        initialSelectedPaths: selectedPath ? [selectedPath] : [],
      });
      model = tree;
      currentPaths = props.paths;
      tree.render({ containerWrapper: container });
      const treeContainer = tree.getFileTreeContainer();
      if (treeContainer) treeContainer.style.height = "100%";
      const clickRoot = treeContainer?.shadowRoot ?? treeContainer;
      const handleClick = (event: Event): void => {
        const path = fileTreeClickedFilePath(event);
        if (path) props.onFileActivate?.(path);
      };
      clickRoot?.addEventListener("click", handleClick);
      const persist = throttle(() => persistTreeUi(tree, currentPaths), 100);
      const unsubscribe = tree.subscribe(persist);
      applyPersistedTreeUi(tree, selectedPath);
      props.modelRef?.(tree);

      return () => {
        persist();
        unsubscribe();
        clickRoot?.removeEventListener("click", handleClick);
        if (model === tree) {
          model = undefined;
          props.modelRef?.(undefined);
        }
        tree.cleanUp();
      };
    }
  );

  createEffect(
    () => props.paths,
    (paths) => {
      if (!model || paths === currentPaths) return;
      if (sameTreePaths(paths, currentPaths)) {
        currentPaths = paths;
        return;
      }
      const snapshotExpanded = snapshotExpandedDirectoryPaths(asTreeState(model), currentPaths);
      persistTreeUi(model, currentPaths);
      model.resetPaths({
        preparedInput: prepareFileTreeInput(paths),
        initialExpandedPaths: snapshotExpanded,
      });
      currentPaths = paths;
      applyPersistedTreeUi(model, props.selectedPath ?? null);
    }
  );

  createEffect(
    () => icons(),
    (currentIcons) => model?.setIcons(currentIcons)
  );

  createEffect(
    () => props.gitStatus,
    (gitStatus) => {
      if (!model) return;
      persistTreeUi(model, currentPaths);
      model.setGitStatus(gitStatus);
      applyPersistedTreeUi(model, props.selectedPath ?? null);
      persistTreeUi(model, currentPaths);
    }
  );

  createEffect(
    () => theme(),
    (currentTheme) => {
      for (const property of appliedThemeProperties) container.style.removeProperty(property);
      const styles = themeToTreeStyles(currentTheme);
      appliedThemeProperties = Object.keys(styles).map(cssPropertyName);
      for (const [property, value] of Object.entries(styles)) {
        container.style.setProperty(cssPropertyName(property), value);
      }
    }
  );

  return (
    <div
      ref={(element) => {
        container = element;
      }}
      class={props.class}
      style={{
        height: "100%",
        "min-height": 0,
        "--trees-focus-ring-width-override": "0px",
        "--trees-selected-focused-border-color-override": "transparent",
      }}
    />
  );
};
