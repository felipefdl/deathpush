export const getAuthorInitials = (name: string): string => {
  const words = name.trim().split(/\s+/);
  if (words.length === 0) return "?";
  if (words.length === 1) return words[0][0]?.toUpperCase() ?? "?";
  return (words[0][0] + words[1][0]).toUpperCase();
};

const AVATAR_HUES = [0, 45, 90, 160, 210, 260, 310, 340];

export const hashAuthorColor = (name: string): string => {
  let hash = 0;
  for (let i = 0; i < name.length; i++) {
    hash = (hash * 31 + name.charCodeAt(i)) | 0;
  }
  const hue = AVATAR_HUES[Math.abs(hash) % AVATAR_HUES.length];
  return `hsl(${hue}, 50%, 40%)`;
};
