import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { formatRelativeDate } from "./format-date";

describe("formatRelativeDate", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2025-06-15T12:00:00Z"));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("just now", () => {
    it("returns just now for 0 seconds ago", () => {
      expect(formatRelativeDate("2025-06-15T12:00:00Z")).toBe("just now");
    });

    it("returns just now for 30 seconds ago", () => {
      expect(formatRelativeDate("2025-06-15T11:59:30Z")).toBe("just now");
    });

    it("returns just now for 59 seconds ago", () => {
      expect(formatRelativeDate("2025-06-15T11:59:01Z")).toBe("just now");
    });
  });

  describe("minutes", () => {
    it("returns 1 minute ago for 60 seconds ago", () => {
      expect(formatRelativeDate("2025-06-15T11:59:00Z")).toBe("1 minute ago");
    });

    it("returns 5 minutes ago", () => {
      expect(formatRelativeDate("2025-06-15T11:55:00Z")).toBe("5 minutes ago");
    });

    it("returns 59 minutes ago", () => {
      expect(formatRelativeDate("2025-06-15T11:01:00Z")).toBe("59 minutes ago");
    });
  });

  describe("hours", () => {
    it("returns 1 hour ago", () => {
      expect(formatRelativeDate("2025-06-15T11:00:00Z")).toBe("1 hour ago");
    });

    it("returns 3 hours ago", () => {
      expect(formatRelativeDate("2025-06-15T09:00:00Z")).toBe("3 hours ago");
    });

    it("returns 23 hours ago", () => {
      expect(formatRelativeDate("2025-06-14T13:00:00Z")).toBe("23 hours ago");
    });
  });

  describe("days", () => {
    it("returns 1 day ago", () => {
      expect(formatRelativeDate("2025-06-14T12:00:00Z")).toBe("1 day ago");
    });

    it("returns 6 days ago", () => {
      expect(formatRelativeDate("2025-06-09T12:00:00Z")).toBe("6 days ago");
    });
  });

  describe("weeks", () => {
    it("returns 1 week ago", () => {
      expect(formatRelativeDate("2025-06-08T12:00:00Z")).toBe("1 week ago");
    });

    it("returns 3 weeks ago", () => {
      expect(formatRelativeDate("2025-05-25T12:00:00Z")).toBe("3 weeks ago");
    });
  });

  describe("months", () => {
    it("returns 1 month ago", () => {
      expect(formatRelativeDate("2025-05-16T12:00:00Z")).toBe("1 month ago");
    });

    it("returns 11 months ago", () => {
      expect(formatRelativeDate("2024-07-19T12:00:00Z")).toBe("11 months ago");
    });
  });

  describe("years", () => {
    it("returns 1 year ago", () => {
      expect(formatRelativeDate("2024-06-15T12:00:00Z")).toBe("1 year ago");
    });

    it("returns 3 years ago", () => {
      expect(formatRelativeDate("2022-06-16T12:00:00Z")).toBe("3 years ago");
    });
  });

  describe("edge cases", () => {
    it("returns just now for a future date", () => {
      expect(formatRelativeDate("2025-06-16T12:00:00Z")).toBe("just now");
    });
  });
});
