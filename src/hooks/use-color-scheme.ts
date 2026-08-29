import { createSignal, onSettled } from "solid-js";

const getScheme = () => document.documentElement.style.getPropertyValue("color-scheme") || "dark";

let cached = getScheme();
const listeners = new Set<() => void>();

window.addEventListener("deathpush:theme-applied", () => {
  const next = getScheme();
  if (next !== cached) {
    cached = next;
    for (const fn of listeners) fn();
  }
});

const subscribe = (cb: () => void) => {
  listeners.add(cb);
  return () => listeners.delete(cb);
};

export const useColorScheme = () => {
  const [scheme, setScheme] = createSignal(cached);

  onSettled(() => {
    return subscribe(() => setScheme(() => cached));
  });

  return scheme;
};
