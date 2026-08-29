import { createRoot, flush } from "solid-js";
import { createStore } from "zustand/vanilla";
import { afterEach, describe, expect, it } from "vite-plus/test";
import { useStore } from "./use-store";

type CounterState = {
  count: number;
  label: string;
  increment: () => void;
};

const createCounterStore = () =>
  createStore<CounterState>((set) => ({
    count: 0,
    label: "n",
    increment: () => set((state) => ({ count: state.count + 1 })),
  }));

describe("useStore", () => {
  const disposers: Array<() => void> = [];

  afterEach(() => {
    while (disposers.length > 0) {
      disposers.pop()?.();
    }
  });

  it("returns the selected snapshot and updates after flush", () => {
    const store = createCounterStore();
    let selected!: () => number;

    disposers.push(
      createRoot((dispose) => {
        selected = useStore(store, (state) => state.count);
        return dispose;
      })
    );

    expect(selected()).toBe(0);
    store.getState().increment();
    expect(selected()).toBe(0);
    flush();
    expect(selected()).toBe(1);
  });

  it("does not notify when equalityFn treats the next value as equal", () => {
    const store = createCounterStore();
    let selected!: () => { count: number };
    let reads = 0;

    disposers.push(
      createRoot((dispose) => {
        selected = useStore(
          store,
          (state) => {
            reads += 1;
            return { count: state.count };
          },
          (left, right) => left.count === right.count
        );
        return dispose;
      })
    );

    const first = selected();
    store.setState({ label: "changed" });
    flush();
    expect(selected()).toBe(first);
    expect(reads).toBe(2);
  });

  it("unsubscribes on owner disposal", () => {
    const store = createCounterStore();
    let selected!: () => number;

    const dispose = createRoot((innerDispose) => {
      selected = useStore(store, (state) => state.count);
      return innerDispose;
    });
    disposers.push(dispose);

    store.getState().increment();
    flush();
    expect(selected()).toBe(1);

    dispose();
    store.getState().increment();
    flush();
    expect(selected()).toBe(1);
  });
});
