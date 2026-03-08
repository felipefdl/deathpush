export const throttle = <T extends (...args: never[]) => void>(fn: T, ms: number): T => {
  let lastRun = 0;
  let timer: ReturnType<typeof setTimeout> | null = null;

  const throttled = (...args: Parameters<T>) => {
    const now = Date.now();
    const remaining = ms - (now - lastRun);

    if (remaining <= 0) {
      if (timer) {
        clearTimeout(timer);
        timer = null;
      }
      lastRun = now;
      fn(...args);
    } else if (!timer) {
      timer = setTimeout(() => {
        lastRun = Date.now();
        timer = null;
        fn(...args);
      }, remaining);
    }
  };

  return throttled as T;
};
