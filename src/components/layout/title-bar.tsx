import { repositoryStore } from "../../stores/repository-store";
import { useStore } from "../../lib/use-store";
import { IS_MACOS } from "../../lib/platform";

export const TitleBar = () => {
  if (!IS_MACOS) return null;

  const status = useStore(repositoryStore, (s) => s.status);

  const repoName = () =>
    status()?.root ? (status()!.root.split("/").filter(Boolean).pop() ?? "DeathPush") : "DeathPush";

  const branch = () => (status()?.headBranch ? ` - ${status()!.headBranch}` : "");

  return (
    <div class="title-bar" data-tauri-drag-region>
      <span class="title-bar-text" data-tauri-drag-region>
        {repoName()}
        {branch()}
      </span>
    </div>
  );
};
