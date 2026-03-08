import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { execSync } from "child_process";
import { readFileSync, existsSync } from "fs";
import { join } from "path";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

const gitHash = execSync("git rev-parse --short=4 HEAD").toString().trim();
const pkg = JSON.parse(readFileSync("package.json", "utf-8"));

type LicenseEntry = { name: string; license: string; url: string; category: "npm" | "rust" | "asset" };

function collectLicenses(): LicenseEntry[] {
  const entries: LicenseEntry[] = [];
  const seen = new Set<string>();

  const add = (entry: LicenseEntry) => {
    if (seen.has(entry.name)) return;
    seen.add(entry.name);
    entries.push(entry);
  };

  // npm dependencies
  for (const name of Object.keys(pkg.dependencies ?? {})) {
    const pkgPath = join("node_modules", name, "package.json");
    if (!existsSync(pkgPath)) continue;
    try {
      const depPkg = JSON.parse(readFileSync(pkgPath, "utf-8"));
      const repo = depPkg.repository;
      const url = typeof repo === "string" ? repo : repo?.url ?? "";
      add({
        name: depPkg.name ?? name,
        license: depPkg.license ?? "Unknown",
        url: url.replace(/^git\+/, "").replace(/\.git$/, ""),
        category: "npm",
      });
    } catch {
      // skip unreadable packages
    }
  }

  // Cargo dependencies
  try {
    const raw = execSync("cargo metadata --format-version 1 --manifest-path src-tauri/Cargo.toml", {
      encoding: "utf-8",
      maxBuffer: 10 * 1024 * 1024,
    });
    const meta = JSON.parse(raw);
    for (const dep of meta.packages ?? []) {
      if (dep.name === "deathpush") continue;
      add({
        name: dep.name,
        license: dep.license ?? "Unknown",
        url: dep.repository ?? "",
        category: "rust",
      });
    }
  } catch {
    // cargo metadata may fail in CI or without Rust toolchain
  }

  // Manual asset entries
  add({ name: "Seti Icon Theme", license: "MIT", url: "https://github.com/jesseweed/seti-ui", category: "asset" });
  add({
    name: "Material Icon Theme",
    license: "MIT",
    url: "https://github.com/PKief/vscode-material-icon-theme",
    category: "asset",
  });
  add({
    name: "MesloLGS Nerd Font Mono",
    license: "Apache-2.0",
    url: "https://github.com/ryanoasis/nerd-fonts",
    category: "asset",
  });

  entries.sort((a, b) => a.name.localeCompare(b.name));
  return entries;
}

const licenses = collectLicenses();

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],
  define: {
    __GIT_HASH__: JSON.stringify(gitHash),
    __APP_VERSION__: JSON.stringify(pkg.version),
    __LICENSES__: JSON.stringify(licenses),
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  build: {
    chunkSizeWarningLimit: 4000,
    rollupOptions: {
      output: {
        manualChunks: {
          "vendor-monaco": ["monaco-editor"],
          "vendor-xterm": ["@xterm/xterm", "@xterm/addon-fit", "@xterm/addon-web-links"],
        },
      },
    },
  },
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
