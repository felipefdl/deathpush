import { createSignal, onCleanup } from "solid-js";
import type { StoreApi } from "zustand/vanilla";

export type EqualityFn<T> = (a: T, b: T) => boolean;

export const useStore = <T, U>(
  store: StoreApi<T>,
  selector: (state: T) => U,
  equalityFn: EqualityFn<U> = Object.is
): (() => U) => {
  const [selected, setSelected] = createSignal<U>(selector(store.getState()) as Exclude<U, Function>);

  const unsubscribe = store.subscribe((state) => {
    const next = selector(state);
    if (!equalityFn(next, selected())) {
      setSelected(() => next);
    }
  });

  onCleanup(unsubscribe);
  return selected;
};
