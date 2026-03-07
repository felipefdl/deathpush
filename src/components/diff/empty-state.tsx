import { useColorScheme } from "../../hooks/use-color-scheme";

export const EmptyState = () => {
  const colorScheme = useColorScheme();
  const isDark = colorScheme === "dark";

  return (
    <div className="diff-empty-state">
      <img
        className="diff-empty-watermark"
        src={isDark ? "/deathpush-white.png" : "/deathpush-black.png"}
        alt=""
      />
      <p style={{ opacity: 0.4, marginTop: 12 }}>Select a file to view changes</p>
    </div>
  );
};
