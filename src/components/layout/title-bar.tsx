import { repositoryStore } from "../../stores/repository-store";
import { useStore } from "../../lib/use-store";
import { IS_MACOS } from "../../lib/platform";

type TitleBarProps = {
  root?: string;
  branch?: string | null;
};

export const TitleBar = (props: TitleBarProps) => {
  if (!IS_MACOS) return null;

  const status = useStore(repositoryStore, (s) => s.status);

  const titleText = () => {
    const root = status()?.root ?? props.root;
    const repoName = root ? (root.split("/").filter(Boolean).pop() ?? "DeathPush") : "DeathPush";
    const branch = status()?.headBranch ?? props.branch;
    return `${repoName}${branch ? ` - ${branch}` : ""}`;
  };

  return (
    <div class="title-bar" data-tauri-drag-region>
      <span class="title-bar-text" data-tauri-drag-region>
        {titleText()}
      </span>
    </div>
  );
};
