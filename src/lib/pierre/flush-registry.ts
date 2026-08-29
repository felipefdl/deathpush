const flushers = new Map<string, () => Promise<void>>();

export const registerFlusher = (path: string, flush: () => Promise<void>): (() => void) => {
  flushers.set(path, flush);
  return () => {
    if (flushers.get(path) === flush) flushers.delete(path);
  };
};

export const flushAll = async (): Promise<void> => {
  await Promise.all([...flushers.values()].map((fn) => fn()));
};

export const flushPath = async (path: string): Promise<void> => {
  await flushers.get(path)?.();
};
