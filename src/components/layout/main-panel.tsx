import type { JSX } from "@solidjs/web";
import { useStore } from "../../lib/use-store";
import { layoutStore } from "../../stores/layout-store";

type MainPanelProps = {
  changesView: JSX.Element;
  historyView: JSX.Element;
  settingsView?: JSX.Element;
  fileView?: JSX.Element;
};

export const MainPanel = (props: MainPanelProps) => {
  const mainView = useStore(layoutStore, (s) => s.mainView);

  return (
    <div style={{ display: "flex", "flex-direction": "column", height: "100%" }}>
      <div style={{ flex: 1, "min-height": 0, display: mainView() === "changes" ? undefined : "none" }}>
        {props.changesView}
      </div>
      <div style={{ flex: 1, "min-height": 0, display: mainView() === "history" ? undefined : "none" }}>
        {props.historyView}
      </div>
      {mainView() === "settings" && props.settingsView && (
        <div style={{ flex: 1, "min-height": 0 }}>{props.settingsView}</div>
      )}
      {mainView() === "file" && props.fileView && <div style={{ flex: 1, "min-height": 0 }}>{props.fileView}</div>}
    </div>
  );
};
