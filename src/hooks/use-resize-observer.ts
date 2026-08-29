import { createSignal, onCleanup } from "solid-js";

export const useResizeObserver = () => {
  const [height, setHeight] = createSignal(0);
  let observer: ResizeObserver | undefined;

  const ref = (el: HTMLDivElement | undefined) => {
    observer?.disconnect();
    observer = undefined;
    if (!el) return;
    observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setHeight(entry.contentRect.height);
      }
    });
    observer.observe(el);
  };

  onCleanup(() => {
    observer?.disconnect();
    observer = undefined;
  });

  return { ref, height };
};
