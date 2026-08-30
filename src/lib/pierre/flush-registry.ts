const flushers = new Map<string, Set<() => Promise<void>>>();
const inFlight = new Set<Promise<void>>();
const inFlightByPath = new Map<string, Set<Promise<void>>>();

const track = (path: string, work: Promise<void>): Promise<void> => {
  inFlight.add(work);
  let pathSet = inFlightByPath.get(path);
  if (!pathSet) {
    pathSet = new Set();
    inFlightByPath.set(path, pathSet);
  }
  pathSet.add(work);
  void work.finally(() => {
    inFlight.delete(work);
    const current = inFlightByPath.get(path);
    current?.delete(work);
    if (current && current.size === 0) inFlightByPath.delete(path);
  });
  return work;
};

export const registerFlusher = (path: string, flush: () => Promise<void>): (() => void) => {
  let set = flushers.get(path);
  if (!set) {
    set = new Set();
    flushers.set(path, set);
  }
  set.add(flush);
  return () => {
    const current = flushers.get(path);
    if (!current) return;
    current.delete(flush);
    if (current.size === 0) flushers.delete(path);
  };
};

export const trackPendingFlush = (path: string, flush: Promise<void>): Promise<void> => track(path, flush);

export const flushAll = async (): Promise<void> => {
  await Promise.all([...flushers.entries()].flatMap(([path, set]) => [...set].map((fn) => track(path, fn()))));
  while (inFlight.size > 0) {
    await Promise.all(inFlight);
  }
};

export const flushPath = async (path: string): Promise<void> => {
  const fns = [...(flushers.get(path) ?? [])];
  const pending = [...(inFlightByPath.get(path) ?? [])];
  await Promise.all([...fns.map((fn) => track(path, fn())), ...pending]);
};

export const flushPaths = async (paths: string[]): Promise<void> => {
  await Promise.all(paths.map((next) => flushPath(next)));
};
