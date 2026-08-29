import { confirm } from "@tauri-apps/plugin-dialog";
import { terminalsHaveActiveProcess } from "./tauri-commands";

export const confirmWindowClose = async (): Promise<boolean> => {
  let busy = false;
  try {
    busy = await terminalsHaveActiveProcess();
  } catch {
    busy = false;
  }
  if (!busy) return true;
  return confirm("A process is still running in the terminal. Close anyway?", {
    title: "Close Window",
    kind: "warning",
    okLabel: "Close",
    cancelLabel: "Cancel",
  });
};
