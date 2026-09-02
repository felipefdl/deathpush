import { cleanup, fireEvent, render } from "@solidjs/testing-library";
import { flush, resetErrorHalt } from "solid-js";
import { afterEach, beforeEach, describe, expect, it, vi } from "vite-plus/test";
import type { CommitDetail as CommitDetailData, CommitEntry } from "../../lib/git-types";
import { repositoryStore } from "../../stores/repository-store";
import { CommitDetail } from "./commit-detail";

vi.mock("../../lib/session-client", () => ({
  sendIntent: vi.fn(async () => ({ kind: "snapshot", snapshot: {} })),
}));

const commit = (id: string): CommitEntry => ({
  id,
  shortId: id.slice(0, 7),
  message: `message ${id}`,
  authorName: "Ada",
  authorEmail: "ada@example.com",
  authorDate: "2026-01-01T00:00:00Z",
  parentIds: [],
  avatarUrl: "",
});

const detail = (id: string, path: string): CommitDetailData => ({
  commit: commit(id),
  files: [{ path, status: "modified", oldPath: null }],
});

describe("CommitDetail", () => {
  beforeEach(() => {
    repositoryStore.getState().setCommitDetail(null);
  });

  afterEach(() => {
    cleanup();
    resetErrorHalt();
    repositoryStore.getState().setCommitDetail(null);
  });

  it("clears the selected file when the commit changes", () => {
    repositoryStore.getState().setCommitDetail(detail("aaa1111", "src/a.ts"));
    const result = render(() => <CommitDetail />);
    flush();

    fireEvent.click(result.getByText("src/a.ts"));
    flush();
    expect(result.getByText("src/a.ts").closest(".commit-detail-file")?.classList.contains("selected")).toBe(true);

    repositoryStore.getState().setCommitDetail(detail("bbb2222", "src/a.ts"));
    flush();
    expect(result.getByText("src/a.ts").closest(".commit-detail-file")?.classList.contains("selected")).toBe(false);
  });
});
