import {
  FileTree,
  prepareFileTreeInput,
  themeToTreeStyles,
  type FileTreeBuiltInIconSet,
  type FileTreeDensityKeyword,
  type FileTreeDirectoryHandle,
  type FileTreeOptions,
  type GitStatusEntry,
} from "@pierre/trees";
import { createEffect, createSignal, onSettled } from "solid-js";
import { themeStore } from "../../stores/theme-store";
import { settingsStore } from "../../stores/settings-store";
import { useStore } from "../../lib/use-store";

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
};

type TreeStateSnapshot = {
  expandedPaths: readonly string[];
  focusedPath: string | null;
  selectedPaths: readonly string[];
};

const EMPTY_TREE_STATE: TreeStateSnapshot = {
  expandedPaths: [],
  focusedPath: null,
  selectedPaths: [],
};

const collectDirectoryPaths = (paths: readonly string[]): string[] => {
  const directories = new Set<string>();
  for (const path of paths) {
    const normalized = path.endsWith("/") ? path.slice(0, -1) : path;
    const segments = normalized.split("/");
    const parentCount = path.endsWith("/") ? segments.length : segments.length - 1;
    for (let index = 1; index <= parentCount; index += 1) {
      directories.add(`${segments.slice(0, index).join("/")}/`);
    }
  }
  return [...directories];
};
const cssPropertyName = (property: string): string =>
  property.startsWith("--") ? property : property.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`);

const snapshotTreeState = (model: FileTree, paths: readonly string[]): TreeStateSnapshot => ({
  expandedPaths: collectDirectoryPaths(paths).filter((path) => {
    const item = model.getItem(path);
    return item?.isDirectory() === true && (item as FileTreeDirectoryHandle).isExpanded();
  }),
  focusedPath: model.getFocusedPath(),
  selectedPaths: model.getSelectedPaths(),
});

const restoreSelection = (model: FileTree, snapshot: TreeStateSnapshot): void => {
  for (const path of snapshot.selectedPaths) model.getItem(path)?.select();
  if (snapshot.focusedPath) model.focusPath(snapshot.focusedPath);
};

export const FileTreeHost = (props: FileTreeHostProps) => {
  const density = useStore(settingsStore, (state) => state.settings.ui.treeDensity);
  const icons = useStore(settingsStore, (state) => state.settings.ui.treeIcons);
  const theme = useStore(themeStore, (state) => state.currentTheme);
  const [ready, setReady] = createSignal(false);
  let container!: HTMLDivElement;
  let model: FileTree | undefined;
  let currentPaths: readonly string[] = [];
  let retainedState = EMPTY_TREE_STATE;
  let appliedThemeProperties: string[] = [];

  onSettled(() => {
    setReady(true);
    return () => setReady(false);
  });

  createEffect(
    () => [ready(), density()] as const,
    ([isReady, currentDensity]) => {
      if (!isReady) return;

      const tree = new FileTree({
        ...props.options,
        preparedInput: prepareFileTreeInput(props.paths),
        density: currentDensity as FileTreeDensityKeyword,
        icons: icons() as FileTreeBuiltInIconSet,
        gitStatus: props.gitStatus,
        initialExpandedPaths: retainedState.expandedPaths,
        initialSelectedPaths: retainedState.selectedPaths,
      });
      model = tree;
      currentPaths = props.paths;
      tree.render({ containerWrapper: container });
      const treeContainer = tree.getFileTreeContainer();
      if (treeContainer) treeContainer.style.height = "100%";
      if (retainedState.focusedPath) tree.focusPath(retainedState.focusedPath);
      props.modelRef?.(tree);

      return () => {
        retainedState = snapshotTreeState(tree, currentPaths);
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
      const snapshot = snapshotTreeState(model, currentPaths);
      model.resetPaths({
        preparedInput: prepareFileTreeInput(paths),
        initialExpandedPaths: snapshot.expandedPaths,
      });
      currentPaths = paths;
      restoreSelection(model, snapshot);
    }
  );

  createEffect(
    () => icons(),
    (currentIcons) => model?.setIcons(currentIcons)
  );

  createEffect(
    () => props.gitStatus,
    (gitStatus) => model?.setGitStatus(gitStatus)
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
