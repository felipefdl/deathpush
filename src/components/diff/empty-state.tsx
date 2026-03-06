import { useThemeStore } from "../../stores/theme-store";

export const EmptyState = () => {
  const themeKind = useThemeStore((s) => s.currentTheme.kind);
  const isDark = themeKind === "dark" || themeKind === "hc-dark";

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
