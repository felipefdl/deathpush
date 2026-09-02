import { confirm } from "@tauri-apps/plugin-dialog";
import type { Intent, IntentOutcome, SessionPatch, SessionSnapshot, SessionStatusEvent } from "./git-types";
import { getSessionSnapshot, sessionIntent } from "./tauri-commands";
import { repositoryStore } from "../stores/repository-store";

let pendingClearFile = false;

const isOlderGeneration = (generation: number, current: number): boolean => generation < current;

const isSameGenerationOlderRevision = (
  generation: number,
  revision: number,
  currentGeneration: number,
  currentRevision: number
): boolean => generation === currentGeneration && revision < currentRevision;

const isNewerCursor = (
  generation: number,
  revision: number,
  currentGeneration: number,
  currentRevision: number
): boolean => generation > currentGeneration || (generation === currentGeneration && revision > currentRevision);

const payloadMatchesSession = (result: { sessionGeneration: number; sessionRevision: number }): boolean => {
  const current = repositoryStore.getState();
  if (result.sessionGeneration !== current.sessionGeneration) {
    return false;
  }
  return !isSameGenerationOlderRevision(
    result.sessionGeneration,
    result.sessionRevision,
    current.sessionGeneration,
    current.sessionRevision
  );
};

export const acceptedDiff = (result: IntentOutcome): result is Extract<IntentOutcome, { kind: "diff" }> =>
  result.kind === "diff" && payloadMatchesSession(result);

export const acceptedBlame = (result: IntentOutcome): result is Extract<IntentOutcome, { kind: "blame" }> =>
  result.kind === "blame" && payloadMatchesSession(result);

const isOlderCursor = (
  generation: number,
  revision: number,
  currentGeneration: number,
  currentRevision: number
): boolean => generation < currentGeneration || (generation === currentGeneration && revision < currentRevision);

const applyRepoGroups = (
  repo: SessionSnapshot["repo"],
  groups: SessionSnapshot["groups"],
  statusGeneration: number,
  statusRevision: number
): void => {
  repositoryStore.setState({
    statusGeneration,
    statusRevision,
    status: {
      root: repo.root,
      headBranch: repo.headBranch,
      headCommit: repo.headCommit,
      ahead: repo.ahead,
      behind: repo.behind,
      groups,
      operationState: repo.operationState,
    },
  });
};

const extrasPatch = (extras: SessionStatusEvent["extras"]): Record<string, unknown> => {
  if (!extras) {
    return {};
  }
  return {
    ...(extras.lastCommit !== undefined ? { lastCommit: extras.lastCommit } : {}),
    ...(extras.branches !== undefined ? { branches: extras.branches } : {}),
    ...(extras.tags !== undefined ? { tags: extras.tags } : {}),
    ...(extras.commitLog !== undefined ? { commitLog: extras.commitLog } : {}),
    ...(extras.stashes !== undefined ? { stashes: extras.stashes } : {}),
  };
};

export const applySessionSnapshot = (snapshot: SessionSnapshot): void => {
  const previous = repositoryStore.getState();
  const sameRoot = previous.status === null || previous.status.root === snapshot.repo.root;
  if (isOlderGeneration(snapshot.sessionGeneration, previous.sessionGeneration)) {
    if (
      sameRoot &&
      isNewerCursor(
        snapshot.statusGeneration,
        snapshot.statusRevision,
        previous.statusGeneration,
        previous.statusRevision
      )
    ) {
      applyRepoGroups(snapshot.repo, snapshot.groups, snapshot.statusGeneration, snapshot.statusRevision);
    }
    return;
  }
  if (
    isSameGenerationOlderRevision(
      snapshot.sessionGeneration,
      snapshot.sessionRevision,
      previous.sessionGeneration,
      previous.sessionRevision
    )
  ) {
    if (
      sameRoot &&
      !isOlderCursor(
        snapshot.statusGeneration,
        snapshot.statusRevision,
        previous.statusGeneration,
        previous.statusRevision
      )
    ) {
      applyRepoGroups(snapshot.repo, snapshot.groups, snapshot.statusGeneration, snapshot.statusRevision);
    }
    return;
  }
  const applyGroups =
    !sameRoot ||
    !isOlderCursor(
      snapshot.statusGeneration,
      snapshot.statusRevision,
      previous.statusGeneration,
      previous.statusRevision
    );
  const nextFile = snapshot.selection.file;
  const fileChanged =
    previous.selectedFile?.path !== nextFile?.path ||
    previous.selectedFile?.staged !== nextFile?.staged ||
    previous.selectedFile?.groupKind !== nextFile?.groupKind;
  repositoryStore.setState((state) => ({
    sessionGeneration: snapshot.sessionGeneration,
    sessionRevision: snapshot.sessionRevision,
    ...(applyGroups
      ? {
          statusGeneration: snapshot.statusGeneration,
          statusRevision: snapshot.statusRevision,
          status: {
            root: snapshot.repo.root,
            headBranch: snapshot.repo.headBranch,
            headCommit: snapshot.repo.headCommit,
            ahead: snapshot.repo.ahead,
            behind: snapshot.repo.behind,
            groups: snapshot.groups,
            operationState: snapshot.repo.operationState,
          },
        }
      : {}),
    selectedFile: nextFile,
    selectedLoadId:
      (!nextFile && previous.selectedFile) || (nextFile && fileChanged)
        ? state.selectedLoadId + 1
        : state.selectedLoadId,
    amendMode: snapshot.scm.amendMode,
    commitMessage: snapshot.scm.commitMessage,
    fileFilter: snapshot.scm.fileFilter,
    commitLog: snapshot.commitLog,
    branches: snapshot.branches,
    stashes: snapshot.stashes,
    tags: snapshot.tags,
    selectedCommit: snapshot.selection.commit,
    commitDetail: snapshot.commitDetail,
    fileHistoryPath: snapshot.fileHistoryPath,
    lastCommit: snapshot.lastCommit,
    actions: snapshot.actions,
    error: snapshot.error,
    ...(nextFile ? {} : { diff: null, diffLoadId: null, blame: null, cursorLine: null }),
  }));
};

export const applySessionPatch = (patch: SessionPatch, sessionGeneration: number, sessionRevision: number): void => {
  const previous = repositoryStore.getState();
  if (isOlderGeneration(sessionGeneration, previous.sessionGeneration)) {
    return;
  }
  if (
    isSameGenerationOlderRevision(
      sessionGeneration,
      sessionRevision,
      previous.sessionGeneration,
      previous.sessionRevision
    )
  ) {
    return;
  }
  switch (patch.kind) {
    case "actions":
      repositoryStore.setState({ actions: patch.actions, sessionGeneration, sessionRevision });
      return;
    case "scm":
      repositoryStore.setState({
        amendMode: patch.scm.amendMode,
        commitMessage: patch.scm.commitMessage,
        fileFilter: patch.scm.fileFilter,
        actions: patch.actions,
        sessionGeneration,
        sessionRevision,
      });
      return;
    case "fileHistory":
      repositoryStore.setState({
        fileHistoryPath: patch.path,
        commitLog: patch.commitLog,
        sessionGeneration,
        sessionRevision,
      });
      return;
    case "commitLog":
      repositoryStore.setState({ commitLog: patch.commitLog, sessionGeneration, sessionRevision });
      return;
    case "commit":
      repositoryStore.setState({
        selectedCommit: patch.id,
        commitDetail: patch.detail,
        sessionGeneration,
        sessionRevision,
      });
      return;
  }
};

export const applySessionStatus = (event: SessionStatusEvent): void => {
  const previous = repositoryStore.getState();
  const sameRoot = previous.status === null || previous.status.root === event.repo.root;
  const applySessionFields =
    !isOlderGeneration(event.sessionGeneration, previous.sessionGeneration) &&
    !isSameGenerationOlderRevision(
      event.sessionGeneration,
      event.sessionRevision,
      previous.sessionGeneration,
      previous.sessionRevision
    );
  const applyGroups =
    sameRoot &&
    isNewerCursor(event.statusGeneration, event.statusRevision, previous.statusGeneration, previous.statusRevision);
  if (!applySessionFields && !applyGroups) {
    return;
  }
  const nextFile = pendingClearFile ? previous.selectedFile : event.selection.file;
  const fileChanged =
    previous.selectedFile?.path !== nextFile?.path ||
    previous.selectedFile?.staged !== nextFile?.staged ||
    previous.selectedFile?.groupKind !== nextFile?.groupKind;
  repositoryStore.setState((state) => ({
    ...(applySessionFields
      ? {
          sessionGeneration: event.sessionGeneration,
          sessionRevision: event.sessionRevision,
          selectedFile: nextFile,
          selectedLoadId:
            (!nextFile && previous.selectedFile) || (nextFile && fileChanged)
              ? state.selectedLoadId + 1
              : state.selectedLoadId,
          actions: event.actions,
          selectedCommit: event.selection.commit,
          ...extrasPatch(event.extras),
          ...(nextFile ? {} : { diff: null, diffLoadId: null, blame: null, cursorLine: null }),
        }
      : {}),
    ...(applyGroups
      ? {
          statusGeneration: event.statusGeneration,
          statusRevision: event.statusRevision,
          status: {
            root: event.repo.root,
            headBranch: event.repo.headBranch,
            headCommit: event.repo.headCommit,
            ahead: event.repo.ahead,
            behind: event.repo.behind,
            groups: event.groups,
            operationState: event.repo.operationState,
          },
        }
      : {}),
  }));
};

export const sendIntent = async (intent: Intent): Promise<IntentOutcome> => {
  if (intent.type === "clearFile") {
    pendingClearFile = true;
  }
  try {
    const previous = repositoryStore.getState();
    const root = previous.status?.root;
    const result = await sessionIntent(intent);
    if (result.kind === "snapshot") {
      applySessionSnapshot(result.snapshot);
      return result;
    }
    if (result.kind === "patch") {
      applySessionPatch(result.patch, result.sessionGeneration, result.sessionRevision);
      return result;
    }
    if (result.kind === "diff" || result.kind === "blame") {
      const current = repositoryStore.getState();
      if (
        result.sessionGeneration !== current.sessionGeneration ||
        (root !== undefined && current.status?.root !== root) ||
        isSameGenerationOlderRevision(
          result.sessionGeneration,
          result.sessionRevision,
          current.sessionGeneration,
          current.sessionRevision
        )
      ) {
        return result;
      }
      if (
        isNewerCursor(
          result.sessionGeneration,
          result.sessionRevision,
          current.sessionGeneration,
          current.sessionRevision
        )
      ) {
        repositoryStore.setState({
          sessionGeneration: result.sessionGeneration,
          sessionRevision: result.sessionRevision,
        });
      }
      return result;
    }
    if (result.kind === "ack" && result.sessionGeneration !== undefined && result.sessionRevision !== undefined) {
      const current = repositoryStore.getState();
      if (
        !isOlderGeneration(result.sessionGeneration, current.sessionGeneration) &&
        !isSameGenerationOlderRevision(
          result.sessionGeneration,
          result.sessionRevision,
          current.sessionGeneration,
          current.sessionRevision
        )
      ) {
        repositoryStore.setState({
          sessionGeneration: result.sessionGeneration,
          sessionRevision: result.sessionRevision,
          ...(intent.type === "clearFile"
            ? { selectedFile: null, diff: null, blame: null, selectedLoadId: current.selectedLoadId + 1 }
            : {}),
        });
      }
    }
    return result;
  } finally {
    if (intent.type === "clearFile") {
      pendingClearFile = false;
    }
  }
};

const withConfirmed = (intent: Intent): Intent | null => {
  if (!("confirmed" in intent)) {
    return null;
  }
  return { ...intent, confirmed: true };
};

export const sendDestructiveIntent = async (intent: Intent): Promise<IntentOutcome> => {
  const result = await sendIntent(intent);
  if (result.kind !== "needsConfirmation") {
    return result;
  }
  const confirmed = await confirm(result.message, {
    title: "Confirm",
    kind: "warning",
    okLabel: "Continue",
    cancelLabel: "Cancel",
  });
  if (!confirmed) {
    return result;
  }
  const next = withConfirmed(intent);
  if (!next) {
    return result;
  }
  return sendIntent(next);
};

export const fetchSessionSnapshot = async (): Promise<void> => {
  applySessionSnapshot(await getSessionSnapshot());
};
