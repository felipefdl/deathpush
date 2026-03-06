import { useEffect } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

export const useTauriEvent = <T>(event: string, handler: (payload: T) => void) => {
  useEffect(() => {
    const unlisten = getCurrentWebviewWindow().listen<T>(event, (e) => handler(e.payload));
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [event, handler]);
};
