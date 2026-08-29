const flushers = new Map<string, () => Promise<void>>();
const inFlight = new Set<Promise<void>>();

const track = (work: Promise<void>): Promise<void> => {
  inFlight.add(work);
  void work.finally(() => {
    inFlight.delete(work);
  });
  return work;
};

export const registerFlusher = (path: string, flush: () => Promise<void>): (() => void) => {
  flushers.set(path, flush);
  return () => {
    if (flushers.get(path) === flush) flushers.delete(path);
  };
};

export const trackPendingFlush = (flush: Promise<void>): Promise<void> => track(flush);

export const flushAll = async (): Promise<void> => {
  await Promise.all([...flushers.values()].map((fn) => track(fn())));
  while (inFlight.size > 0) {
    await Promise.all(inFlight);
  }
};

export const flushPath = async (path: string): Promise<void> => {
  const fn = flushers.get(path);
  if (fn) await track(fn());
};

export const flushPaths = async (paths: string[]): Promise<void> => {
  await Promise.all(paths.map((path) => flushPath(path)));
};
