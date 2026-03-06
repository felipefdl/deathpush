const MINUTE = 60;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;
const WEEK = 7 * DAY;
const MONTH = 30 * DAY;
const YEAR = 365 * DAY;

export const formatRelativeDate = (isoDate: string): string => {
  const date = new Date(isoDate);
  const now = Date.now();
  const diffSecs = Math.floor((now - date.getTime()) / 1000);

  if (diffSecs < 0) return "just now";
  if (diffSecs < MINUTE) return "just now";
  if (diffSecs < HOUR) {
    const mins = Math.floor(diffSecs / MINUTE);
    return mins === 1 ? "1 minute ago" : `${mins} minutes ago`;
  }
  if (diffSecs < DAY) {
    const hours = Math.floor(diffSecs / HOUR);
    return hours === 1 ? "1 hour ago" : `${hours} hours ago`;
  }
  if (diffSecs < WEEK) {
    const days = Math.floor(diffSecs / DAY);
    return days === 1 ? "1 day ago" : `${days} days ago`;
  }
  if (diffSecs < MONTH) {
    const weeks = Math.floor(diffSecs / WEEK);
    return weeks === 1 ? "1 week ago" : `${weeks} weeks ago`;
  }
  if (diffSecs < YEAR) {
    const months = Math.floor(diffSecs / MONTH);
    return months === 1 ? "1 month ago" : `${months} months ago`;
  }
  const years = Math.floor(diffSecs / YEAR);
  return years === 1 ? "1 year ago" : `${years} years ago`;
};
