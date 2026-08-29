import { useColorScheme } from "../../hooks/use-color-scheme";

export const EmptyState = () => {
  const colorScheme = useColorScheme();

  return (
    <div class="diff-empty-state">
      <img
        class="diff-empty-watermark"
        src={colorScheme() === "dark" ? "/deathpush-white.png" : "/deathpush-black.png"}
        alt=""
      />
      <p style={{ opacity: 0.4, "margin-top": "12px" }}>Select a file to view changes</p>
    </div>
  );
};
