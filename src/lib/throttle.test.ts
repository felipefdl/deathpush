import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { throttle } from "./throttle";

describe("throttle", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("calls immediately on first invocation", () => {
    const fn = vi.fn();
    const throttled = throttle(fn, 100);
    throttled();
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it("throttles subsequent calls within interval", () => {
    const fn = vi.fn();
    const throttled = throttle(fn, 100);
    throttled();
    throttled();
    expect(fn).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(100);
    expect(fn).toHaveBeenCalledTimes(2);
  });

  it("calls after interval expires", () => {
    const fn = vi.fn();
    const throttled = throttle(fn, 100);
    throttled();
    expect(fn).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(101);
    throttled();
    expect(fn).toHaveBeenCalledTimes(2);
  });

  it("passes arguments correctly", () => {
    const fn = vi.fn();
    const throttled = throttle(fn, 100);
    throttled("a", 42);
    expect(fn).toHaveBeenCalledWith("a", 42);
  });

  it("trailing call fires with the args that set the timer", () => {
    const fn = vi.fn();
    const throttled = throttle(fn, 100);
    throttled("first");
    throttled("second");
    vi.advanceTimersByTime(100);
    expect(fn).toHaveBeenCalledTimes(2);
    expect(fn).toHaveBeenLastCalledWith("second");
  });

  it("no trailing call if only one invocation", () => {
    const fn = vi.fn();
    const throttled = throttle(fn, 100);
    throttled();
    vi.advanceTimersByTime(200);
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it("multiple rapid calls only queue one trailing", () => {
    const fn = vi.fn();
    const throttled = throttle(fn, 100);
    throttled();
    throttled();
    throttled();
    throttled();
    throttled();
    expect(fn).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(100);
    expect(fn).toHaveBeenCalledTimes(2);
  });
});
