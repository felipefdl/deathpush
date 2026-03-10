import { useRepositoryStore } from "../../stores/repository-store";
import { IS_MACOS } from "../../lib/platform";

export const TitleBar = () => {
  const { status } = useRepositoryStore();

  if (!IS_MACOS) return null;

  const repoName = status?.root
    ? status.root.split("/").filter(Boolean).pop() ?? "DeathPush"
    : "DeathPush";

  const branch = status?.headBranch ? ` - ${status.headBranch}` : "";

  return (
    <div className="title-bar" data-tauri-drag-region>
      <span className="title-bar-text" data-tauri-drag-region>{repoName}{branch}</span>
    </div>
  );
};
