import { createEffect } from "solid-js";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

export const useTauriEvent = <T>(event: string, handler: (payload: T) => void) => {
  let handlerRef = handler;
  createEffect(
    () => handler,
    (next) => {
      handlerRef = next;
    }
  );

  createEffect(
    () => event,
    (name) => {
      const unlisten = getCurrentWebviewWindow().listen<T>(name, (e) => handlerRef(e.payload));
      return () => {
        void unlisten.then((fn) => fn());
      };
    }
  );
};
