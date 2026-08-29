import { useCallback, useEffect, useMemo, useState } from "react";
import "./App.css";
import { useTranslation } from "react-i18next";
import { Toaster } from "sonner";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import ExplorePage from "./components/skills/ExplorePage";
import FilterBar from "./components/skills/FilterBar";
import SkillDetailView from "./components/skills/SkillDetailView";
import Header from "./components/skills/Header";
import LoadingOverlay from "./components/skills/LoadingOverlay";
import SkillsList from "./components/skills/SkillsList";
import Modal from "./components/shared/Modal";
import AddSkillModal from "./components/skills/modals/AddSkillModal";
import DeleteModal from "./components/skills/modals/DeleteModal";
import GitPickModal from "./components/skills/modals/GitPickModal";
import LocalPickModal from "./components/skills/modals/LocalPickModal";
import ImportModal from "./components/skills/modals/ImportModal";
import NewToolsModal from "./components/skills/modals/NewToolsModal";
import SharedDirModal from "./components/skills/modals/SharedDirModal";
import ToolConfigModal from "./components/shared/ToolConfigModal";
import SettingsPage from "./components/skills/SettingsPage";
import ProjectsPage from "./components/projects/ProjectsPage";
import { useAddSkillFlow } from "./hooks/useAddSkillFlow";
import { useExploreState } from "./hooks/useExploreState";
import { useSettingsState } from "./hooks/useSettingsState";
import { useSkillLibrary } from "./hooks/useSkillLibrary";
import { useStatusReporter } from "./hooks/useStatusReporter";
import { useSyncOrchestration } from "./hooks/useSyncOrchestration";
import { useUpdateChecker } from "./hooks/useUpdateChecker";
import { invokeTauri, isTauri } from "./lib/tauri";
import type { ManagedSkill } from "./components/skills/types";

// App is the binder: it owns only i18n and view/navigation state, composes
// the per-world hooks (each returning that world's data + actions), and wires
// their interfaces together. State logic lives in src/hooks/, not here.
function App() {
  const { t, i18n } = useTranslation();
  const language = i18n.resolvedLanguage ?? i18n.language ?? "en";
  const languageStorageKey = "skills-language";
  const groupByRepoStorageKey = "skills-groupByRepo";
  const viewModeStorageKey = "skills-viewMode";
  const toggleLanguage = useCallback(() => {
    void i18n.changeLanguage(language === "en" ? "zh" : "en");
  }, [i18n, language]);

  // View/navigation state (stays in the binder: it is what App composes for).
  const [activeView, setActiveView] = useState<
    | "myskills"
    | "explore"
    | "detail"
    | "settings"
    | "projects"
    | "explore-detail"
  >("myskills");
  const [detailSkill, setDetailSkill] = useState<ManagedSkill | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [sortBy, setSortBy] = useState<"name" | "updated" | "added">("name");
  const [groupByRepo, setGroupByRepo] = useState(() => {
    try {
      return window.localStorage.getItem(groupByRepoStorageKey) === "true";
    } catch {
      return false;
    }
  });
  const [viewMode, setViewMode] = useState<"list" | "auto-grid" | "dense-grid">(
    () => {
      try {
        const stored = window.localStorage.getItem(viewModeStorageKey);
        if (
          stored === "list" ||
          stored === "auto-grid" ||
          stored === "dense-grid"
        )
          return stored;
      } catch {
        // ignore storage failures
      }
      return "list";
    },
  );

  useEffect(() => {
    if (typeof window === "undefined") return;
    if (language !== "en" && language !== "zh") return;
    try {
      window.localStorage.setItem(languageStorageKey, language);
    } catch {
      // ignore storage failures
    }
  }, [language, languageStorageKey]);

  useEffect(() => {
    if (typeof window === "undefined") return;
    try {
      window.localStorage.setItem(groupByRepoStorageKey, String(groupByRepo));
    } catch {
      // ignore storage failures
    }
  }, [groupByRepo, groupByRepoStorageKey]);

  useEffect(() => {
    if (typeof window === "undefined") return;
    try {
      window.localStorage.setItem(viewModeStorageKey, viewMode);
    } catch {
      // ignore storage failures
    }
  }, [viewMode, viewModeStorageKey]);

  // World hooks, wired in dependency order: reporter → sync → library →
  // settings/explore/addFlow. Hooks never import each other; every
  // cross-world need flows through the interfaces passed here.
  const reporter = useStatusReporter(t);
  const updates = useUpdateChecker(t);
  const sync = useSyncOrchestration({ t, reporter });
  const library = useSkillLibrary({ t, reporter, sync });

  const openExploreDetail = useCallback((skill: ManagedSkill) => {
    setDetailSkill(skill);
    setActiveView("explore-detail");
  }, []);

  const settings = useSettingsState({
    t,
    reporter,
    onManagedSkillsChanged: library.loadManagedSkills,
  });
  const explore = useExploreState({
    t,
    reporter,
    onOpenExploreDetail: openExploreDetail,
  });
  const addFlow = useAddSkillFlow({ t, reporter, sync, library });

  const { loading, loadingStartAt, actionMessage, cancelLoading } = reporter;
  const {
    updateAvailableVersion,
    updateBody,
    updateInstalling,
    updateDone,
    dismissUpdate,
    dismissUpdateForever,
    updateNow,
  } = updates;

  const formatRelative = (ms: number | null | undefined) => {
    if (!ms) return t("relative.empty");
    const diff = Date.now() - ms;
    if (diff < 0) return t("relative.empty");
    const minutes = Math.floor(diff / 60000);
    if (minutes < 1) return t("relative.justNow");
    if (minutes < 60) {
      return t("relative.minutesAgo", { minutes });
    }
    const hours = Math.floor(minutes / 60);
    if (hours < 24) {
      return t("relative.hoursAgo", { hours });
    }
    const days = Math.floor(hours / 24);
    return t("relative.daysAgo", { days });
  };

  const getSkillSourceLabel = (skill: ManagedSkill) => {
    const key = skill.source_type.toLowerCase();
    if (key.includes("git") && skill.source_ref) {
      return skill.source_ref;
    }
    return skill.central_path;
  };

  const getGithubInfo = (url: string | null | undefined) => {
    if (!url) return null;
    const normalized = url.replace(/^git\+/, "");
    try {
      const parsed = new URL(normalized);
      if (!parsed.hostname.includes("github.com")) return null;
      const parts = parsed.pathname.split("/").filter(Boolean);
      const owner = parts[0];
      const repo = parts[1]?.replace(/\.git$/, "");
      if (!owner || !repo) return null;
      return {
        label: `${owner}/${repo}`,
        href: `https://github.com/${owner}/${repo}`,
      };
    } catch {
      const match = normalized.match(/github\.com\/([^/]+)\/([^/#?]+)/i);
      if (!match) return null;
      const owner = match[1];
      const repo = match[2].replace(/\.git$/, "");
      return {
        label: `${owner}/${repo}`,
        href: `https://github.com/${owner}/${repo}`,
      };
    }
  };

  const visibleSkills = useMemo(() => {
    const query = searchQuery.trim().toLowerCase();
    const wildcardPattern = query.includes("*")
      ? new RegExp(
          query
            .split("*")
            .map((part) => part.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"))
            .join(".*"),
        )
      : null;
    const matchesQuery = (value: string) =>
      wildcardPattern ? wildcardPattern.test(value) : value.includes(query);
    const filtered = library.managedSkills.filter((skill) => {
      if (!query) return true;
      return (
        matchesQuery(skill.name.toLowerCase()) ||
        matchesQuery(skill.source_ref?.toLowerCase() ?? "") ||
        matchesQuery(skill.central_path.toLowerCase()) ||
        matchesQuery(skill.source_type.toLowerCase())
      );
    });
    const sorted = [...filtered].sort((a, b) => {
      if (sortBy === "name") {
        return a.name.localeCompare(b.name);
      }
      if (sortBy === "added") {
        return (b.created_at ?? 0) - (a.created_at ?? 0);
      }
      return (b.updated_at ?? 0) - (a.updated_at ?? 0);
    });
    return sorted;
  }, [library.managedSkills, searchQuery, sortBy]);

  const handleOpenSettings = useCallback(() => {
    setActiveView("settings");
  }, []);

  const handleCloseSettings = useCallback(() => {
    setActiveView("myskills");
  }, []);

  const { loadFeaturedSkills, loadHiddenSkills } = explore;
  const handleViewChange = useCallback(
    (view: "myskills" | "explore" | "projects") => {
      setActiveView(view);
      if (view === "explore") {
        loadFeaturedSkills();
        loadHiddenSkills();
      }
      if (view === "myskills") {
        setDetailSkill(null);
      }
    },
    [loadFeaturedSkills, loadHiddenSkills],
  );

  const handleOpenDetail = useCallback((skill: ManagedSkill) => {
    setDetailSkill(skill);
    setActiveView("detail");
  }, []);

  const handleBackToList = useCallback(() => {
    setDetailSkill(null);
    setActiveView("myskills");
  }, []);

  const handleBackToExplore = useCallback(() => {
    setDetailSkill(null);
    setActiveView("explore");
  }, []);

  const { handleExploreInstall } = addFlow;
  const handleExploreInstallFromDetail = useCallback(() => {
    if (!detailSkill?.source_ref) return;
    const sourceUrl = detailSkill.source_ref;
    handleExploreInstall(sourceUrl);
    setDetailSkill(null);
    setActiveView("explore");
  }, [detailSkill, handleExploreInstall]);

  // "Sync all to the new tools" spans two worlds — enable the targets (sync)
  // and push every managed skill (library) — so the binder composes it.
  const { relevantNewlyInstalled, enableTargetsFor, setShowNewToolsModal } =
    sync;
  const { syncAllManagedToTools } = library;
  const handleSyncAllNewTools = useCallback(() => {
    if (relevantNewlyInstalled.length === 0) return;
    enableTargetsFor(relevantNewlyInstalled);
    setShowNewToolsModal(false);
    void syncAllManagedToTools(relevantNewlyInstalled);
  }, [
    enableTargetsFor,
    relevantNewlyInstalled,
    setShowNewToolsModal,
    syncAllManagedToTools,
  ]);

  const handleSortChange = useCallback(
    (value: "name" | "updated" | "added") => {
      setSortBy(value);
    },
    [],
  );

  const handleSearchChange = useCallback((value: string) => {
    setSearchQuery(value);
  }, []);

  return (
    <div className="skills-app">
      <Toaster
        position="top-right"
        richColors
        toastOptions={{ duration: 1800 }}
      />
      <LoadingOverlay
        loading={loading}
        actionMessage={actionMessage}
        loadingStartAt={loadingStartAt}
        onCancel={cancelLoading}
        t={t}
      />

      <Header
        language={language}
        loading={loading}
        activeView={activeView}
        onToggleLanguage={toggleLanguage}
        onOpenSettings={handleOpenSettings}
        onViewChange={handleViewChange}
        t={t}
      />

      <main className="skills-main">
        {(activeView === "detail" || activeView === "explore-detail") &&
        detailSkill ? (
          <SkillDetailView
            skill={detailSkill}
            onBack={
              activeView === "explore-detail"
                ? handleBackToExplore
                : handleBackToList
            }
            invokeTauri={invokeTauri}
            formatRelative={formatRelative}
            t={t}
            isExplorePreview={activeView === "explore-detail"}
            onInstall={
              activeView === "explore-detail"
                ? handleExploreInstallFromDetail
                : undefined
            }
          />
        ) : activeView === "myskills" ? (
          <div className="dashboard-stack">
            <FilterBar
              sortBy={sortBy}
              searchQuery={searchQuery}
              loading={loading}
              onSortChange={handleSortChange}
              onSearchChange={handleSearchChange}
              onRefresh={library.handleRefresh}
              autoSyncEnabled={sync.autoSyncEnabled}
              onAutoSyncChange={sync.handleAutoSyncToggle}
              onUnsyncAll={library.handleUnsyncAll}
              onConfigureTools={sync.handleOpenToolConfig}
              groupByRepo={groupByRepo}
              onGroupByRepoChange={setGroupByRepo}
              viewMode={viewMode}
              onViewModeChange={setViewMode}
              t={t}
            />
            <SkillsList
              plan={addFlow.plan}
              visibleSkills={visibleSkills}
              groupByRepo={groupByRepo}
              viewMode={viewMode}
              installedTools={sync.installedTools}
              loading={loading}
              getGithubInfo={getGithubInfo}
              getSkillSourceLabel={getSkillSourceLabel}
              formatRelative={formatRelative}
              onReviewImport={addFlow.handleReviewImport}
              onUpdateSkill={library.handleUpdateSkill}
              onDeleteSkill={library.handleDeletePrompt}
              onToggleTool={library.handleToggleToolForSkill}
              onUnsyncSkill={library.handleUnsyncSkill}
              onSyncSkillToAllTools={library.handleSyncSkillToAllTools}
              onOpenDetail={handleOpenDetail}
              t={t}
            />
          </div>
        ) : activeView === "settings" ? (
          <SettingsPage
            isTauri={isTauri}
            language={language}
            storagePath={settings.storagePath}
            gitCacheCleanupDays={settings.gitCacheCleanupDays}
            gitCacheTtlSecs={settings.gitCacheTtlSecs}
            themePreference={settings.themePreference}
            zoomLevel={settings.zoomLevel}
            onPickStoragePath={settings.handlePickStoragePath}
            onToggleLanguage={toggleLanguage}
            onThemeChange={settings.handleThemeChange}
            onZoomLevelChange={settings.handleZoomLevelChange}
            onGitCacheCleanupDaysChange={settings.handleGitCacheCleanupDaysChange}
            onGitCacheTtlSecsChange={settings.handleGitCacheTtlSecsChange}
            onClearGitCacheNow={settings.handleClearGitCacheNow}
            githubToken={settings.githubToken}
            onGithubTokenChange={settings.handleGithubTokenChange}
            onBack={handleCloseSettings}
            t={t}
          />
        ) : activeView === "projects" ? (
          <ProjectsPage />
        ) : (
          <ExplorePage
            featuredSkills={explore.featuredSkills}
            featuredLoading={explore.featuredLoading}
            exploreFilter={explore.exploreFilter}
            searchResults={explore.searchResults}
            searchLoading={explore.searchLoading}
            managedSkills={library.managedSkills}
            loading={loading}
            hiddenSkills={explore.hiddenSkills}
            showHidden={explore.showHidden}
            onShowHiddenChange={explore.setShowHidden}
            onHideSkill={explore.handleHideSkill}
            onUnhideSkill={explore.handleUnhideSkill}
            onExploreFilterChange={explore.handleExploreFilterChange}
            onInstallSkill={addFlow.handleExploreInstall}
            onViewSkill={explore.handleOpenExploreDetail}
            onOpenManualAdd={addFlow.handleOpenAdd}
            t={t}
          />
        )}
      </main>

      <AddSkillModal
        open={addFlow.showAddModal}
        loading={loading}
        canClose={!loading}
        addModalTab={addFlow.addModalTab}
        localPath={addFlow.localPath}
        localName={addFlow.localName}
        gitUrl={addFlow.gitUrl}
        gitName={addFlow.gitName}
        syncTargets={sync.syncTargets}
        installedTools={sync.installedTools}
        toolStatus={sync.toolStatus}
        onRequestClose={addFlow.handleCloseAdd}
        onTabChange={addFlow.setAddModalTab}
        onLocalPathChange={addFlow.setLocalPath}
        onPickLocalPath={addFlow.handlePickLocalPath}
        onLocalNameChange={addFlow.setLocalName}
        onGitUrlChange={addFlow.setGitUrl}
        onGitNameChange={addFlow.setGitName}
        onSyncTargetChange={sync.handleSyncTargetChange}
        onSubmit={
          addFlow.addModalTab === "local"
            ? addFlow.handleCreateLocal
            : addFlow.handleCreateGit
        }
        t={t}
      />

      {addFlow.showImportModal && addFlow.plan ? (
        <ImportModal
          open={addFlow.showImportModal}
          loading={loading}
          plan={addFlow.plan}
          selected={addFlow.selected}
          variantChoice={addFlow.variantChoice}
          onRequestClose={addFlow.handleCloseImport}
          onToggleAll={addFlow.toggleAll}
          onToggleGroup={addFlow.handleToggleGroup}
          onSelectVariant={addFlow.handleSelectVariant}
          onImport={addFlow.handleImport}
          t={t}
        />
      ) : null}

      <SharedDirModal
        open={Boolean(library.pendingSharedToggle)}
        loading={loading}
        toolLabel={library.pendingSharedLabels?.toolLabel ?? ""}
        otherLabels={library.pendingSharedLabels?.otherLabels ?? ""}
        onRequestClose={library.handleSharedCancel}
        onConfirm={library.handleSharedConfirm}
        t={t}
      />

      <ToolConfigModal
        open={sync.showToolConfigModal}
        loading={loading}
        toolStatus={sync.toolStatus}
        savedSelection={sync.globalSelectedTools}
        scanSelectedOnly={sync.scanSelectedToolsOnly}
        labels={{
          title: t("globalToolConfigTitle"),
          description: t("globalToolConfigDesc"),
          confirmLabel: t("globalToolConfigConfirm"),
          scanToggleLabel: t("globalToolConfigScanSelectedOnly"),
        }}
        onConfirm={sync.handleToolConfigConfirm}
        onRequestClose={sync.handleCloseToolConfig}
        t={t}
      />

      <NewToolsModal
        open={Boolean(sync.showNewToolsModal && sync.newlyInstalledToolsText)}
        loading={loading}
        toolsLabelText={sync.newlyInstalledToolsText}
        onLater={sync.handleCloseNewTools}
        onSyncAll={handleSyncAllNewTools}
        t={t}
      />

      <DeleteModal
        open={Boolean(library.pendingDeleteId)}
        loading={loading}
        skillName={library.pendingDeleteSkill?.name ?? null}
        onRequestClose={library.handleCloseDelete}
        onConfirm={() => {
          if (library.pendingDeleteSkill)
            void library.handleDeleteManaged(library.pendingDeleteSkill);
        }}
        t={t}
      />

      <LocalPickModal
        open={addFlow.showLocalPickModal}
        loading={loading}
        localCandidates={addFlow.localCandidates}
        localCandidateSelected={addFlow.localCandidateSelected}
        onRequestClose={addFlow.handleCloseLocalPick}
        onCancel={addFlow.handleCancelLocalPick}
        onToggleAll={addFlow.handleToggleAllLocalCandidates}
        onToggleCandidate={addFlow.handleToggleLocalCandidate}
        onInstall={addFlow.handleInstallSelectedLocalCandidates}
        t={t}
      />

      <GitPickModal
        open={addFlow.showGitPickModal}
        loading={loading}
        gitCandidates={addFlow.gitCandidates}
        gitCandidateSelected={addFlow.gitCandidateSelected}
        onRequestClose={addFlow.handleCloseGitPick}
        onCancel={addFlow.handleCancelGitPick}
        onToggleAll={addFlow.handleToggleAllGitCandidates}
        onToggleCandidate={addFlow.handleToggleGitCandidate}
        onInstall={addFlow.handleInstallSelectedCandidates}
        t={t}
      />

      {updateAvailableVersion && (
        <Modal
          open
          plain
          className="update-modal"
          onRequestClose={updateInstalling ? undefined : dismissUpdate}
        >
            {!updateInstalling && !updateDone && (
              <button
                className="modal-close update-modal-close"
                type="button"
                onClick={dismissUpdate}
                aria-label={t("close")}
              >
                ✕
              </button>
            )}
            <div className="update-modal-body">
              <div className="update-modal-title">
                {updateDone
                  ? t("updateInstalledRestart")
                  : t("updateAvailable")}
              </div>
              {!updateDone && (
                <div className="update-modal-text">
                  {t("updateBannerText", { version: updateAvailableVersion })}
                </div>
              )}
              {!updateDone && updateBody && (
                <div className="update-modal-notes">
                  <Markdown remarkPlugins={[remarkGfm]}>{updateBody}</Markdown>
                </div>
              )}
            </div>
            <div className="update-modal-actions">
              {updateDone ? (
                <button
                  className="btn btn-primary"
                  type="button"
                  onClick={dismissUpdate}
                >
                  {t("done")}
                </button>
              ) : (
                <>
                  <button
                    className="btn btn-primary"
                    type="button"
                    disabled={updateInstalling}
                    onClick={updateNow}
                  >
                    {updateInstalling ? t("installingUpdate") : t("updateNow")}
                  </button>
                  {!updateInstalling && (
                    <button
                      className="btn btn-secondary"
                      type="button"
                      onClick={dismissUpdateForever}
                    >
                      {t("updateBannerDismiss")}
                    </button>
                  )}
                </>
              )}
            </div>
        </Modal>
      )}
    </div>
  );
}

export default App;
