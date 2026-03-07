import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export const checkForUpdate = async (): Promise<Update | null> => {
  try {
    return await check();
  } catch {
    return null;
  }
};

export const downloadAndInstallUpdate = async (
  update: Update,
  onProgress?: (downloaded: number, total: number | undefined) => void,
): Promise<void> => {
  let downloaded = 0;
  let contentLength: number | undefined;

  await update.downloadAndInstall((event) => {
    if (event.event === "Started") {
      contentLength = event.data.contentLength;
    } else if (event.event === "Progress") {
      downloaded += event.data.chunkLength;
      onProgress?.(downloaded, contentLength);
    }
  });
  await relaunch();
};
