import { useCallback, useEffect, useMemo, useState } from "react";
import "./App.css";
import { useTranslation } from "react-i18next";
// The sanctioned second importer of the toast library: the binder mounts the
// Toaster once; useStatusReporter makes every call and owns toast lifetime.
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
import NotificationsModal from "./components/shared/NotificationsModal";
import AddSkillModal from "./components/skills/modals/AddSkillModal";
import DeleteModal from "./components/skills/modals/DeleteModal";
import GitRepointModal from "./components/skills/modals/GitRepointModal";
import InvocationModeModal from "./components/skills/modals/InvocationModeModal";
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
import { usePersistedPreference } from "./hooks/usePersistedPreference";
import { filterAndSortSkills } from "./lib/skillPresentation";
import {
  groupByRepoPreference,
  languagePreference,
  viewModePreference,
} from "./lib/preferences";
import { invokeTauri, isTauri } from "./lib/tauri";
import type { ManagedSkill } from "./components/skills/types";

// App is the binder: it owns only i18n and view/navigation state, composes
// the per-world hooks (each returning that world's data + actions), and wires
// their interfaces together. State logic lives in src/hooks/, not here.
function App() {
  const { t, i18n } = useTranslation();
  const language = i18n.resolvedLanguage ?? i18n.language ?? "en";
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
  const [exploreDetailSkill, setExploreDetailSkill] = useState<ManagedSkill | null>(null);
  const [showNotifications, setShowNotifications] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [sortBy, setSortBy] = useState<"name" | "updated" | "added">("name");
  const [groupByRepo, setGroupByRepo] =
    usePersistedPreference(groupByRepoPreference);
  const [viewMode, setViewMode] = usePersistedPreference(viewModePreference);

  useEffect(() => {
    if (language !== "en" && language !== "zh") return;
    languagePreference.write(language);
  }, [language]);

  // World hooks, wired in dependency order: reporter → sync → library →
  // settings/explore/addFlow. Hooks never import each other; every
  // cross-world need flows through the interfaces passed here.
  const reporter = useStatusReporter(t);
  const updates = useUpdateChecker({ reporter });
  const sync = useSyncOrchestration({ t, reporter });
  const library = useSkillLibrary({ t, reporter, sync });
  const effectiveView = activeView === "detail" && !library.detailSkill ? "myskills" : activeView;

  const openExploreDetail = useCallback((skill: ManagedSkill) => {
    setExploreDetailSkill(skill);
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

  const {
    loading,
    loadingStartAt,
    actionMessage,
    cancelLoading,
    notify,
    notifyError,
    copyToClipboard,
    notifications,
    unreadCount,
    markAllRead,
    clearNotifications,
  } = reporter;
  const {
    updateAvailableVersion,
    updateBody,
    updateInstalling,
    updateDone,
    dismissUpdate,
    dismissUpdateForever,
    updateNow,
  } = updates;

  const visibleSkills = useMemo(
    () =>
      filterAndSortSkills(library.managedSkills, {
        query: searchQuery,
        sort: sortBy,
      }),
    [library.managedSkills, searchQuery, sortBy],
  );

  const handleOpenSettings = useCallback(() => {
    setActiveView("settings");
  }, []);

  const handleCloseSettings = useCallback(() => {
    setActiveView("myskills");
  }, []);

  // Opening the history marks it read.
  const handleOpenNotifications = useCallback(() => {
    markAllRead();
    setShowNotifications(true);
  }, [markAllRead]);

  const handleCloseNotifications = useCallback(() => {
    setShowNotifications(false);
  }, []);

  const { loadFeaturedSkills, loadHiddenSkills } = explore;
  const { openDetail, closeDetail } = library;
  const detailSkill = effectiveView === "explore-detail"
    ? exploreDetailSkill : library.detailSkill;
  const handleViewChange = useCallback(
    (view: "myskills" | "explore" | "projects") => {
      setActiveView(view);
      if (view === "explore") {
        loadFeaturedSkills();
        loadHiddenSkills();
      }
      if (view === "myskills") {
        closeDetail();
      }
    },
    [closeDetail, loadFeaturedSkills, loadHiddenSkills],
  );

  const handleOpenDetail = useCallback((skill: ManagedSkill) => {
    openDetail(skill.id);
    setActiveView("detail");
  }, [openDetail]);

  const handleBackToList = useCallback(() => {
    closeDetail();
    setActiveView("myskills");
  }, [closeDetail]);

  const handleBackToExplore = useCallback(() => {
    setExploreDetailSkill(null);
    setActiveView("explore");
  }, []);

  const { handleExploreInstall } = addFlow;
  const handleExploreInstallFromDetail = useCallback(() => {
    if (!detailSkill?.source_ref) return;
    const sourceUrl = detailSkill.source_ref;
    handleExploreInstall(sourceUrl);
    setExploreDetailSkill(null);
    setActiveView("explore");
  }, [detailSkill, handleExploreInstall]);

  // "Sync all to the new tools" spans two worlds — enable the targets (sync)
  // and push every managed skill (library) — so the binder composes it.
  const {
    relevantNewlyInstalled,
    effectiveSyncTargetIds,
    enableTargetsFor,
    setShowNewToolsModal,
  } = sync;
  const { syncAllManagedToTools } = library;
  const handleSyncAllNewTools = useCallback(() => {
    if (relevantNewlyInstalled.length === 0) return;
    // A sync target set is the operator's recorded selection, not detection —
    // a newly detected tool outside it is announced but never written to.
    // (The `scan_selected_tools_only` scan setting must not gate a sync.)
    const targets = relevantNewlyInstalled.filter((id) =>
      effectiveSyncTargetIds.includes(id),
    );
    setShowNewToolsModal(false);
    if (targets.length === 0) return;
    enableTargetsFor(targets);
    void syncAllManagedToTools(targets);
  }, [
    effectiveSyncTargetIds,
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
      {/* Toast lifetime is owned by useStatusReporter, per kind. Errors
          never auto-dismiss, so the visible stack is raised above sonner's
          default of 3: a fourth open error shows without hovering. */}
      <Toaster position="top-right" richColors visibleToasts={5} />
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
        activeView={effectiveView}
        unreadNotifications={unreadCount}
        onToggleLanguage={toggleLanguage}
        onOpenNotifications={handleOpenNotifications}
        onOpenSettings={handleOpenSettings}
        onViewChange={handleViewChange}
        t={t}
      />

      <main className="skills-main">
        {(effectiveView === "detail" || effectiveView === "explore-detail") &&
        detailSkill ? (
          <SkillDetailView
            skill={detailSkill}
            onRepoint={library.handleRepointGitSkill}
            actionLoading={loading}
            onBack={
              effectiveView === "explore-detail"
                ? handleBackToExplore
                : handleBackToList
            }
            invokeTauri={invokeTauri}
            notify={notify}
            t={t}
            isExplorePreview={effectiveView === "explore-detail"}
            onInstall={
              effectiveView === "explore-detail"
                ? handleExploreInstallFromDetail
                : undefined
            }
          />
        ) : effectiveView === "myskills" ? (
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
              allTools={sync.tools}
              loading={loading}
              onReviewImport={addFlow.handleReviewImport}
              onUpdateSkill={library.handleUpdateSkill}
              onRepointSkill={library.handleRepointSkill}
              onDetachSkill={library.handleDetachSkill}
              onRestoreSkill={library.handleRestoreSkill}
              onDeleteSkill={library.handleDeletePrompt}
              onToggleTool={library.handleToggleToolForSkill}
              onUnsyncSkill={library.handleUnsyncSkill}
              onSyncSkillToAllTools={library.handleSyncSkillToAllTools}
              onOpenDetail={handleOpenDetail}
              onInvocationClick={library.openInvocationEdit}
              copyToClipboard={copyToClipboard}
              t={t}
            />
          </div>
        ) : effectiveView === "settings" ? (
          <SettingsPage
            isTauri={isTauri}
            language={language}
            storagePath={settings.storagePath}
            gitCacheCleanupDays={settings.gitCacheCleanupDays}
            gitCacheTtlSecs={settings.gitCacheTtlSecs}
            bounds={settings.bounds}
            themePreference={settings.themePreference}
            zoomLevel={settings.zoomLevel}
            onPickStoragePath={settings.handlePickStoragePath}
            onToggleLanguage={toggleLanguage}
            onThemeChange={settings.handleThemeChange}
            onZoomLevelChange={settings.handleZoomLevelChange}
            onGitCacheCleanupDaysChange={settings.handleGitCacheCleanupDaysChange}
            onGitCacheTtlSecsChange={settings.handleGitCacheTtlSecsChange}
            onClearGitCacheNow={settings.handleClearGitCacheNow}
            onOpenLogFolder={settings.handleOpenLogFolder}
            githubToken={settings.githubToken}
            onGithubTokenChange={settings.handleGithubTokenChange}
            onBack={handleCloseSettings}
            t={t}
          />
        ) : effectiveView === "projects" ? (
          <ProjectsPage
            notify={notify}
            notifyError={notifyError}
          />
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
        onSubmit={addFlow.handleCreate}
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

      <NotificationsModal
        open={showNotifications}
        notifications={notifications}
        onRequestClose={handleCloseNotifications}
        onClear={clearNotifications}
        copyToClipboard={copyToClipboard}
        t={t}
      />

      <SharedDirModal
        pending={sync.sharedDirPending}
        loading={loading}
        onCancel={sync.cancelSharedDirConfirmation}
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

      {library.invocationEditSkill ? (
        <InvocationModeModal
          key={library.invocationEditSkill.id}
          skill={library.invocationEditSkill}
          loading={loading}
          onRequestClose={library.closeInvocationEdit}
          onConfirm={(mode) => {
            if (library.invocationEditSkill) void library.setInvocationOverride(library.invocationEditSkill.id, mode);
          }}
          t={t}
        />
      ) : null}

      {library.pendingGitRepointSkill ? (
        <GitRepointModal
          key={library.pendingGitRepointSkill.id}
          skillName={library.pendingGitRepointSkill.name}
          loading={loading}
          onRequestClose={library.handleCloseRepointGitSkill}
          onConfirm={library.handleConfirmRepointGitSkill}
          t={t}
        />
      ) : null}

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
        open={addFlow.local.visible}
        loading={loading}
        localCandidates={addFlow.local.candidates}
        localCandidateSelected={addFlow.local.selected}
        onRequestClose={addFlow.local.close}
        onCancel={addFlow.local.cancel}
        onToggleAll={addFlow.local.toggleAll}
        onToggleCandidate={addFlow.local.toggle}
        onInstall={addFlow.local.install}
        t={t}
      />

      <GitPickModal
        open={addFlow.git.visible}
        loading={loading}
        gitCandidates={addFlow.git.candidates}
        gitCandidateSelected={addFlow.git.selected}
        onRequestClose={addFlow.git.close}
        onCancel={addFlow.git.cancel}
        onToggleAll={addFlow.git.toggleAll}
        onToggleCandidate={addFlow.git.toggle}
        onInstall={addFlow.git.install}
        t={t}
      />

      {updateAvailableVersion && (
        <Modal
          open
          plain
          aria-label={t("appUpdates")}
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
