import Foundation

@Observable
final class RepositoryService {
  var status: RepositoryStatus?
  var branches: [BranchEntry] = []
  var stashes: [StashEntry] = []
  var tags: [TagEntry] = []
  var commitLog: [CommitEntry] = []
  var lastCommitInfo: LastCommitInfo?
  var nestedRepos: [DiscoveredRepo] = []
  var error: String?
  var isLoading = false
  var operationError: String?
  var operationName: String?

  let sessionId: String

  var repoName: String {
    guard let root = status?.root else { return "DeathPush" }
    return URL(fileURLWithPath: root).lastPathComponent
  }

  var currentBranch: String? {
    status?.headBranch
  }

  var ahead: UInt32 {
    status?.ahead ?? 0
  }

  var behind: UInt32 {
    status?.behind ?? 0
  }

  var operationState: RepoOperationState {
    status?.operationState ?? .clean
  }

  var totalChanges: Int {
    status?.groups.reduce(0) { $0 + $1.files.count } ?? 0
  }

  var resourceGroups: [ResourceGroup] {
    status?.groups ?? []
  }

  init() {
    sessionId = UUID().uuidString
    createSession(sessionId: sessionId)
  }

  func destroy() {
    destroySession(sessionId: sessionId)
  }

  @MainActor
  func performOperation(_ name: String, _ work: @escaping () throws -> Void) async {
    isLoading = true
    operationName = name
    operationError = nil
    do {
      try work()
    } catch {
      operationError = error.localizedDescription
    }
    isLoading = false
    operationName = nil
  }

  // MARK: - Repository Lifecycle

  @discardableResult
  func open(path: String) throws -> RepositoryStatus {
    let result = try openRepository(sessionId: sessionId, path: path)
    status = result
    return result
  }

  @discardableResult
  func initRepo(path: String) throws -> RepositoryStatus {
    let result = try initRepository(sessionId: sessionId, path: path)
    status = result
    return result
  }

  @discardableResult
  func clone(url: String, path: String) throws -> RepositoryStatus {
    let result = try cloneRepository(sessionId: sessionId, url: url, path: path)
    status = result
    return result
  }

  // MARK: - Status

  func refreshStatus() throws {
    status = try getStatus(sessionId: sessionId)
  }

  // MARK: - Staging

  func stageFiles(_ paths: [String]) throws {
    status = try DeathPush.stageFiles(sessionId: sessionId, paths: paths)
  }

  func unstageFiles(_ paths: [String]) throws {
    status = try DeathPush.unstageFiles(sessionId: sessionId, paths: paths)
  }

  func stageAllFiles() throws {
    status = try stageAll(sessionId: sessionId)
  }

  func unstageAllFiles() throws {
    status = try unstageAll(sessionId: sessionId)
  }

  func discardFileChanges(_ paths: [String]) throws {
    status = try discardChanges(sessionId: sessionId, paths: paths)
  }

  // MARK: - Hunk Staging

  func getHunks(path: String, staged: Bool) throws -> FileDiffWithHunks {
    try getFileHunks(sessionId: sessionId, path: path, staged: staged)
  }

  func stageHunkAtIndex(path: String, hunkIndex: UInt32, staged: Bool) throws {
    status = try stageHunk(sessionId: sessionId, path: path, hunkIndex: hunkIndex, staged: staged)
  }

  func discardHunkAtIndex(path: String, hunkIndex: UInt32) throws {
    status = try discardHunk(sessionId: sessionId, path: path, hunkIndex: hunkIndex)
  }

  // MARK: - Commit

  func commitChanges(message: String, amend: Bool = false) throws {
    status = try commit(sessionId: sessionId, message: message, amend: amend)
  }

  func undoCommit() throws {
    status = try undoLastCommit(sessionId: sessionId)
  }

  func lastCommitMessage() throws -> String {
    try getLastCommitMessage(sessionId: sessionId)
  }

  // MARK: - Branches

  func refreshBranches() throws {
    branches = try listBranches(sessionId: sessionId)
  }

  func switchBranch(name: String) throws {
    status = try checkoutBranch(sessionId: sessionId, name: name)
  }

  func createNewBranch(name: String, startPoint: String? = nil) throws {
    status = try createBranch(sessionId: sessionId, name: name, startPoint: startPoint)
  }

  func removeBranch(name: String, force: Bool = false) throws {
    try deleteBranch(sessionId: sessionId, name: name, force: force)
  }

  func renameBranchTo(oldName: String, newName: String) throws {
    status = try renameBranch(sessionId: sessionId, oldName: oldName, newName: newName)
  }

  func removeRemoteBranch(remote: String = "origin", name: String) throws {
    try deleteRemoteBranch(sessionId: sessionId, remote: remote, name: name)
  }

  // MARK: - Remote

  func fetchRemote(remote: String = "origin", prune: Bool = false) throws {
    status = try fetch(sessionId: sessionId, remote: remote, prune: prune)
  }

  func pullRemote(remote: String = "origin", branch: String = "", rebase: Bool = false) throws {
    status = try pull(sessionId: sessionId, remote: remote, branch: branch, rebase: rebase)
  }

  func pushRemote(remote: String = "origin", branch: String = "", force: Bool = false) throws {
    status = try push(sessionId: sessionId, remote: remote, branch: branch, force: force)
  }

  // MARK: - Merge/Rebase

  func mergeBranchInto(name: String) throws {
    status = try mergeBranch(sessionId: sessionId, name: name)
  }

  func abortMerge() throws {
    status = try mergeAbort(sessionId: sessionId)
  }

  func continueMerge() throws {
    status = try mergeContinue(sessionId: sessionId)
  }

  func rebaseBranchOnto(name: String) throws {
    status = try rebaseBranch(sessionId: sessionId, name: name)
  }

  func abortRebase() throws {
    status = try rebaseAbort(sessionId: sessionId)
  }

  func continueRebase() throws {
    status = try rebaseContinue(sessionId: sessionId)
  }

  func skipRebase() throws {
    status = try rebaseSkip(sessionId: sessionId)
  }

  // MARK: - Stash

  func refreshStashes() throws {
    stashes = try stashList(sessionId: sessionId)
  }

  func saveStash(message: String? = nil) throws {
    status = try stashSave(sessionId: sessionId, message: message)
  }

  func saveStashIncludeUntracked(message: String? = nil) throws {
    status = try stashSaveIncludeUntracked(sessionId: sessionId, message: message)
  }

  func saveStashStagedOnly(message: String? = nil) throws {
    status = try stashSaveStaged(sessionId: sessionId, message: message)
  }

  func applyStash(index: UInt32) throws {
    status = try stashApply(sessionId: sessionId, index: index)
  }

  func popStash(index: UInt32) throws {
    status = try stashPop(sessionId: sessionId, index: index)
  }

  func dropStash(index: UInt32) throws {
    stashes = try stashDrop(sessionId: sessionId, index: index)
  }

  func showStash(index: UInt32) throws -> FileDiffWithHunks {
    try stashShow(sessionId: sessionId, index: index)
  }

  // MARK: - Nested Repositories

  func refreshNestedRepos() {
    nestedRepos = (try? discoverRepositories(sessionId: sessionId)) ?? []
  }

  // MARK: - Tags

  func refreshTags() throws {
    tags = try listTags(sessionId: sessionId)
  }

  @discardableResult
  func createNewTag(name: String, message: String? = nil, target: String? = nil) throws -> [TagEntry] {
    let result = try createTag(sessionId: sessionId, name: name, message: message, target: target)
    tags = result
    return result
  }

  @discardableResult
  func removeTag(name: String) throws -> [TagEntry] {
    let result = try deleteTag(sessionId: sessionId, name: name)
    tags = result
    return result
  }

  func pushTagToRemote(remote: String = "origin", name: String) throws {
    try pushTag(sessionId: sessionId, remote: remote, tag: name)
  }

  func removeRemoteTag(remote: String = "origin", name: String) throws {
    try deleteRemoteTag(sessionId: sessionId, remote: remote, name: name)
  }

  // MARK: - Log

  func refreshLog(skip: UInt32 = 0, limit: UInt32 = 50) throws {
    commitLog = try getCommitLog(sessionId: sessionId, skip: skip, limit: limit)
  }

  func loadMoreCommits(limit: UInt32 = 50) throws {
    let skip = UInt32(commitLog.count)
    let more = try getCommitLog(sessionId: sessionId, skip: skip, limit: limit)
    commitLog.append(contentsOf: more)
  }

  // MARK: - Blame & File History

  func blameFile(path: String) throws -> FileBlame {
    try getFileBlame(sessionId: sessionId, path: path)
  }

  func fileLog(path: String, skip: UInt32 = 0, limit: UInt32 = 50) throws -> [CommitEntry] {
    try getFileLog(sessionId: sessionId, path: path, skip: skip, limit: limit)
  }

  // MARK: - Cherry Pick & Reset

  func cherryPickCommit(commitId: String) throws {
    status = try cherryPick(sessionId: sessionId, commitId: commitId)
  }

  func resetToCommitId(_ commitId: String, mode: String = "mixed") throws {
    status = try resetToCommit(sessionId: sessionId, id: commitId, mode: mode)
  }

  // MARK: - Line-Level Staging

  func stageLinesInHunk(path: String, hunkIndex: UInt32, lineStart: UInt32, lineEnd: UInt32, staged: Bool) throws {
    status = try stageLines(sessionId: sessionId, path: path, hunkIndex: hunkIndex, lineStart: lineStart, lineEnd: lineEnd, staged: staged)
  }

  // MARK: - Worktrees

  func listWorktrees() throws -> [WorktreeInfo] {
    try detectWorktrees(sessionId: sessionId)
  }

  // MARK: - Last Commit

  func refreshLastCommitInfo() {
    lastCommitInfo = try? getLastCommitInfo(sessionId: sessionId)
  }

  // MARK: - Explorer

  var explorerCache: [String: [ExplorerEntry]] = [:]
  var explorerExpandedPaths: Set<String> = []

  private let fileContentCache: NSCache<NSString, FileContentCacheEntry> = {
    let cache = NSCache<NSString, FileContentCacheEntry>()
    cache.countLimit = 50
    cache.totalCostLimit = 50 * 1024 * 1024 // ~50MB
    return cache
  }()

  var gitStatusByPath: [String: FileStatus] {
    var map: [String: FileStatus] = [:]
    for group in resourceGroups {
      for file in group.files {
        map[file.path] = file.status
      }
    }
    return map
  }

  func listExplorerDirectory(path: String?) throws -> [ExplorerEntry] {
    let entries = try listDirectory(sessionId: sessionId, path: path)
    let cacheKey = path ?? "__root__"
    explorerCache[cacheKey] = entries
    return entries
  }

  func readExplorerFile(path: String) throws -> FileContent {
    let cacheKey = path as NSString
    if let cached = fileContentCache.object(forKey: cacheKey) {
      return cached.value
    }
    let content = try readFileContent(sessionId: sessionId, path: path)
    if content.fileType == "text" {
      let entry = FileContentCacheEntry(content)
      fileContentCache.setObject(entry, forKey: cacheKey, cost: content.content.utf8.count)
    }
    return content
  }

  func invalidateExplorerCache() {
    explorerCache.removeAll()
  }

  func invalidateFileContentCache() {
    fileContentCache.removeAllObjects()
  }

  func invalidateFileContentCache(path: String) {
    fileContentCache.removeObject(forKey: path as NSString)
  }

  func refreshExplorerExpandedPaths() {
    guard let root = status?.root else { return }
    // Re-fetch root
    explorerCache["__root__"] = try? listDirectory(sessionId: sessionId, path: nil)
    // Re-fetch expanded directories
    for dirPath in explorerExpandedPaths {
      explorerCache[dirPath] = try? listDirectory(sessionId: sessionId, path: dirPath)
    }
  }

  func deleteExplorerEntry(path: String) throws {
    status = try deleteFile(sessionId: sessionId, path: path)
    invalidateExplorerCache()
    refreshExplorerExpandedPaths()
  }

  func renameExplorerEntry(oldPath: String, newName: String) throws {
    try renameEntry(sessionId: sessionId, oldPath: oldPath, newName: newName)
    invalidateExplorerCache()
    refreshExplorerExpandedPaths()
  }

  func createExplorerDirectory(path: String) throws {
    try createDirectory(sessionId: sessionId, path: path)
    invalidateExplorerCache()
    refreshExplorerExpandedPaths()
  }

  func createExplorerFile(path: String) throws {
    try writeFile(sessionId: sessionId, path: path, content: "")
    invalidateExplorerCache()
    refreshExplorerExpandedPaths()
  }

  func duplicateExplorerEntry(path: String) throws -> String {
    let newPath = try duplicateEntry(sessionId: sessionId, path: path)
    invalidateExplorerCache()
    refreshExplorerExpandedPaths()
    return newPath
  }

  func addExplorerEntryToGitignore(pattern: String) throws {
    status = try addToGitignore(sessionId: sessionId, pattern: pattern)
    invalidateExplorerCache()
    refreshExplorerExpandedPaths()
  }

  func deleteExplorerEntries(paths: [String]) throws {
    status = try deleteFiles(sessionId: sessionId, paths: paths)
    invalidateExplorerCache()
    refreshExplorerExpandedPaths()
  }

  func copyExplorerEntries(sources: [String], destinationDir: String, onConflict: String? = nil) throws {
    try copyEntries(sessionId: sessionId, sources: sources, destinationDir: destinationDir, onConflict: onConflict)
    invalidateExplorerCache()
    refreshExplorerExpandedPaths()
  }

  func moveExplorerEntries(sources: [String], destinationDir: String, onConflict: String? = nil) throws {
    try moveEntries(sessionId: sessionId, sources: sources, destinationDir: destinationDir, onConflict: onConflict)
    invalidateExplorerCache()
    refreshExplorerExpandedPaths()
  }

  func importExplorerFiles(sources: [String], destinationDir: String, onConflict: String? = nil) throws {
    try importFiles(sessionId: sessionId, sources: sources, destinationDir: destinationDir, onConflict: onConflict)
    invalidateExplorerCache()
    refreshExplorerExpandedPaths()
  }

  func openFileInEditor(path: String) throws {
    try openInEditor(sessionId: sessionId, path: path)
  }

  // MARK: - Search

  func fuzzyFindExplorerFiles(query: String, maxResults: UInt32 = 50) throws -> [FuzzyFileResult] {
    try fuzzyFindFiles(sessionId: sessionId, query: query, maxResults: maxResults)
  }

  func searchContents(query: String, maxResults: UInt32 = 100) throws -> [ContentSearchResult] {
    try searchFileContents(sessionId: sessionId, query: query, maxResults: maxResults)
  }

  // MARK: - Git Config

  static func getGlobalGitConfig(key: String) throws -> String {
    try getGitConfig(key: key)
  }

  static func setGlobalGitConfig(key: String, value: String) throws {
    try setGitConfig(key: key, value: value)
  }
}

final class FileContentCacheEntry {
  let value: FileContent

  init(_ value: FileContent) {
    self.value = value
  }
}
