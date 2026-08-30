import { onSettled } from "solid-js";
import type { pierreHostStyle } from "../../lib/pierre/normalize-editor-settings";

export type PierreScrollHostHandle = {
  sync: () => void;
  beginRender: () => void;
  finishRender: () => void;
};

export type PierreScrollHostProps = {
  style: ReturnType<typeof pierreHostStyle>;
  rootRef: (element: HTMLDivElement) => void;
  contentRef: (element: HTMLDivElement) => void;
  handleRef?: (handle: PierreScrollHostHandle) => void;
};

const OVERLAY_SCROLLBAR_SIZE_PX = 11;
export const PierreScrollHost = (props: PierreScrollHostProps) => {
  let root!: HTMLDivElement;
  let content!: HTMLDivElement;
  let scrollbarTrack!: HTMLDivElement;
  let scrollbarThumb!: HTMLDivElement;
  let stopThumbDrag: (() => void) | undefined;
  let horizontalTrack!: HTMLDivElement;
  let horizontalThumb!: HTMLDivElement;
  let horizontalScroller: HTMLElement | undefined;
  let stopHorizontalThumbDrag: (() => void) | undefined;
  let revealFrame: number | undefined;

  const syncScrollThumb = (): void => {
    const viewportHeight = root.clientHeight;
    const contentHeight = root.scrollHeight;
    const trackHeight = scrollbarTrack.clientHeight;
    const hasOverflow = viewportHeight > 0 && trackHeight > 0 && contentHeight > viewportHeight;
    scrollbarThumb.hidden = !hasOverflow;
    scrollbarTrack.style.pointerEvents = hasOverflow ? "auto" : "none";
    if (!hasOverflow) return;

    const thumbHeight = Math.min(trackHeight, Math.max(24, (viewportHeight / contentHeight) * trackHeight));
    const scrollRange = contentHeight - viewportHeight;
    const thumbRange = trackHeight - thumbHeight;
    const thumbOffset = (root.scrollTop / scrollRange) * thumbRange;
    scrollbarThumb.style.height = `${thumbHeight}px`;
    scrollbarThumb.style.transform = `translateY(${thumbOffset}px)`;
  };

  const startThumbDrag = (event: PointerEvent): void => {
    event.preventDefault();
    event.stopPropagation();
    stopThumbDrag?.();

    const startY = event.clientY;
    const startScrollTop = root.scrollTop;
    const scrollRange = root.scrollHeight - root.clientHeight;
    const thumbHeight = Number.parseFloat(scrollbarThumb.style.height);
    const thumbRange = scrollbarTrack.clientHeight - thumbHeight;
    if (scrollRange <= 0 || thumbRange <= 0) return;

    const onPointerMove = (moveEvent: PointerEvent): void => {
      root.scrollTop = startScrollTop + ((moveEvent.clientY - startY) / thumbRange) * scrollRange;
      syncScrollThumb();
    };
    const stop = (): void => {
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerup", stop);
      window.removeEventListener("pointercancel", stop);
      if (stopThumbDrag === stop) stopThumbDrag = undefined;
    };

    stopThumbDrag = stop;
    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", stop);
    window.addEventListener("pointercancel", stop);
  };

  const jumpToTrackPosition = (event: PointerEvent): void => {
    if (event.target !== scrollbarTrack) return;
    const trackRect = scrollbarTrack.getBoundingClientRect();
    const thumbHeight = Number.parseFloat(scrollbarThumb.style.height);
    const thumbRange = scrollbarTrack.clientHeight - thumbHeight;
    const scrollRange = root.scrollHeight - root.clientHeight;
    if (scrollRange <= 0 || thumbRange <= 0) return;

    const thumbOffset = Math.min(thumbRange, Math.max(0, event.clientY - trackRect.top - thumbHeight / 2));
    root.scrollTop = (thumbOffset / thumbRange) * scrollRange;
    syncScrollThumb();
  };

  const findHorizontalScroller = (): HTMLElement | undefined => {
    for (const container of content.querySelectorAll("diffs-container")) {
      for (const code of container.shadowRoot?.querySelectorAll<HTMLElement>("[data-code]") ?? []) {
        if (code.scrollWidth > code.clientWidth) return code;
      }
    }
    return undefined;
  };

  const syncHorizontalThumb = (): void => {
    const nextScroller = findHorizontalScroller();
    if (nextScroller !== horizontalScroller) {
      horizontalScroller?.removeEventListener("scroll", syncHorizontalThumb);
      horizontalScroller = nextScroller;
      horizontalScroller?.addEventListener("scroll", syncHorizontalThumb);
    }

    const viewportWidth = horizontalScroller?.clientWidth ?? 0;
    const contentWidth = horizontalScroller?.scrollWidth ?? 0;
    const trackWidth = horizontalTrack.clientWidth;
    const hasOverflow = viewportWidth > 0 && trackWidth > 0 && contentWidth > viewportWidth;
    horizontalThumb.hidden = !hasOverflow;
    horizontalTrack.style.pointerEvents = hasOverflow ? "auto" : "none";
    scrollbarTrack.style.bottom = hasOverflow ? `${OVERLAY_SCROLLBAR_SIZE_PX}px` : "0";
    content.style.paddingBottom = hasOverflow ? `${OVERLAY_SCROLLBAR_SIZE_PX}px` : "";
    if (!hasOverflow || !horizontalScroller) return;

    const thumbWidth = Math.min(trackWidth, Math.max(24, (viewportWidth / contentWidth) * trackWidth));
    const scrollRange = contentWidth - viewportWidth;
    const thumbRange = trackWidth - thumbWidth;
    const thumbOffset = (horizontalScroller.scrollLeft / scrollRange) * thumbRange;
    horizontalThumb.style.width = `${thumbWidth}px`;
    horizontalThumb.style.transform = `translateX(${thumbOffset}px)`;
  };

  const syncScrollbars = (): void => {
    syncScrollThumb();
    syncHorizontalThumb();
  };

  const beginRender = (): void => {
    if (revealFrame !== undefined) cancelAnimationFrame(revealFrame);
    revealFrame = undefined;
    content.setAttribute("data-pierre-mounting", "");
  };

  const finishRender = (): void => {
    if (revealFrame !== undefined) cancelAnimationFrame(revealFrame);
    revealFrame = requestAnimationFrame(() => {
      revealFrame = undefined;
      content.removeAttribute("data-pierre-mounting");
      syncScrollbars();
    });
  };

  const startHorizontalThumbDrag = (event: PointerEvent): void => {
    if (!horizontalScroller) return;
    event.preventDefault();
    event.stopPropagation();
    stopHorizontalThumbDrag?.();

    const startX = event.clientX;
    const startScrollLeft = horizontalScroller.scrollLeft;
    const scrollRange = horizontalScroller.scrollWidth - horizontalScroller.clientWidth;
    const thumbWidth = Number.parseFloat(horizontalThumb.style.width);
    const thumbRange = horizontalTrack.clientWidth - thumbWidth;
    if (scrollRange <= 0 || thumbRange <= 0) return;

    const scroller = horizontalScroller;
    const onPointerMove = (moveEvent: PointerEvent): void => {
      scroller.scrollLeft = startScrollLeft + ((moveEvent.clientX - startX) / thumbRange) * scrollRange;
      syncHorizontalThumb();
    };
    const stop = (): void => {
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerup", stop);
      window.removeEventListener("pointercancel", stop);
      if (stopHorizontalThumbDrag === stop) stopHorizontalThumbDrag = undefined;
    };

    stopHorizontalThumbDrag = stop;
    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", stop);
    window.addEventListener("pointercancel", stop);
  };

  const jumpToHorizontalTrackPosition = (event: PointerEvent): void => {
    if (event.target !== horizontalTrack || !horizontalScroller) return;
    const trackRect = horizontalTrack.getBoundingClientRect();
    const thumbWidth = Number.parseFloat(horizontalThumb.style.width);
    const thumbRange = horizontalTrack.clientWidth - thumbWidth;
    const scrollRange = horizontalScroller.scrollWidth - horizontalScroller.clientWidth;
    if (scrollRange <= 0 || thumbRange <= 0) return;

    const thumbOffset = Math.min(thumbRange, Math.max(0, event.clientX - trackRect.left - thumbWidth / 2));
    horizontalScroller.scrollLeft = (thumbOffset / thumbRange) * scrollRange;
    syncHorizontalThumb();
  };

  props.handleRef?.({ sync: syncScrollbars, beginRender, finishRender });

  onSettled(() => {
    root.addEventListener("scroll", syncScrollbars);
    const resizeObserver = typeof ResizeObserver === "undefined" ? undefined : new ResizeObserver(syncScrollbars);
    const mutationObserver =
      typeof MutationObserver === "undefined"
        ? undefined
        : new MutationObserver(() => {
            for (const container of content.querySelectorAll("diffs-container")) {
              if (container.shadowRoot) {
                mutationObserver?.observe(container.shadowRoot, { childList: true, subtree: true });
              }
            }
            syncScrollbars();
          });
    mutationObserver?.observe(content, { childList: true, subtree: true });
    resizeObserver?.observe(root);
    resizeObserver?.observe(content);
    syncScrollbars();

    return () => {
      stopThumbDrag?.();
      stopHorizontalThumbDrag?.();
      if (revealFrame !== undefined) cancelAnimationFrame(revealFrame);
      root.removeEventListener("scroll", syncScrollbars);
      horizontalScroller?.removeEventListener("scroll", syncHorizontalThumb);
      resizeObserver?.disconnect();
      mutationObserver?.disconnect();
    };
  });

  return (
    <div class="pierre-file-frame">
      <div
        ref={(element) => {
          root = element;
          props.rootRef(element);
        }}
        class="pierre-file-host"
        style={props.style}
      >
        <div
          ref={(element) => {
            content = element;
            props.contentRef(element);
          }}
          class="pierre-file-content"
        />
      </div>
      <div
        ref={(element) => {
          scrollbarTrack = element;
        }}
        class="pierre-file-scrollbar"
        aria-hidden="true"
        onPointerDown={jumpToTrackPosition}
      >
        <div
          ref={(element) => {
            scrollbarThumb = element;
          }}
          class="pierre-file-scrollbar-thumb"
          hidden
          onPointerDown={startThumbDrag}
        />
      </div>
      <div
        ref={(element) => {
          horizontalTrack = element;
        }}
        class="pierre-file-scrollbar-horizontal"
        aria-hidden="true"
        onPointerDown={jumpToHorizontalTrackPosition}
      >
        <div
          ref={(element) => {
            horizontalThumb = element;
          }}
          class="pierre-file-scrollbar-horizontal-thumb"
          hidden
          onPointerDown={startHorizontalThumbDrag}
        />
      </div>
    </div>
  );
};
