import { useEffect, useRef } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

export const useTauriEvent = <T>(event: string, handler: (payload: T) => void) => {
  const handlerRef = useRef(handler);
  handlerRef.current = handler;

  useEffect(() => {
    const unlisten = getCurrentWebviewWindow().listen<T>(event, (e) => handlerRef.current(e.payload));
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [event]);
};
