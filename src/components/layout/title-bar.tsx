import { useRepositoryStore } from "../../stores/repository-store";

export const TitleBar = () => {
  const { status } = useRepositoryStore();

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
