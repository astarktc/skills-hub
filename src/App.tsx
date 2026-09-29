import { useCallback, useEffect, useId, useMemo } from "react";
import "./App.css";
import { useTranslation } from "react-i18next";
// The binder mounts the token-themed Toaster once; useStatusReporter makes
// every toast call and owns toast lifetime.
import { Toaster } from "@/components/ui/sonner";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import ExplorePage from "./components/skills/ExplorePage";
import FilterBar from "./components/skills/FilterBar";
import SkillDetailView, { type DetailSubject } from "./components/skills/SkillDetailView";
import Header from "./components/skills/Header";
import LoadingOverlay from "./components/skills/LoadingOverlay";
import SkillsList from "./components/skills/SkillsList";
import NotificationsModal from "./components/shared/NotificationsModal";
import AddSkillModal from "./components/skills/modals/AddSkillModal";
import DeleteModal from "./components/skills/modals/DeleteModal";
import ChangeSourceModal from "./components/skills/modals/ChangeSourceModal";
import InvocationModeModal from "./components/skills/modals/InvocationModeModal";
import GitPickModal from "./components/skills/modals/GitPickModal";
import LocalPickModal from "./components/skills/modals/LocalPickModal";
import ImportModal from "./components/skills/modals/ImportModal";
import NewToolsModal from "./components/skills/modals/NewToolsModal";
import SharedDirModal from "./components/skills/modals/SharedDirModal";
import OverwriteModal from "./components/skills/modals/OverwriteModal";
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
import { useListLens } from "./hooks/useListLens";
import {
  useFocusReconciliation,
  useSurfaceState,
  type Area,
  type ExplorePreview,
} from "./hooks/useSurfaceState";
import {
  groupByRepoPreference,
  viewModePreference,
} from "./lib/preferences";
import { invokeTauri, isTauri } from "./lib/tauri";
import type { ManagedSkill } from "./components/skills/types";

// App is the binder: it owns only i18n, composes the surface module (what is
// on screen) and the per-world hooks (each returning that world's data +
// actions), and wires their interfaces together. State logic lives in
// src/hooks/, not here.
function App() {
  const { t } = useTranslation();
  const updateTitleId = useId();

  // Surface state first: every world asks it to open and close surfaces.
  const surface = useSurfaceState();

  // World hooks, wired in dependency order: reporter → sync → library →
  // settings/explore/addFlow. Hooks never import each other; every
  // cross-world need flows through the interfaces passed here.
  const reporter = useStatusReporter(t);
  const updates = useUpdateChecker({ reporter, surface });
  const sync = useSyncOrchestration({ t, reporter, surface });
  const library = useSkillLibrary({ t, reporter, sync, surface });
  // A focused or selected skill that leaves the catalog clears itself.
  useFocusReconciliation(surface, "skill", library.managedSkillIds);

  const { area, focused, focus, unfocus, open, openDialog, closeDialog, dismissDialog } =
    surface;
  const openExplorePreview = useCallback(
    (preview: ExplorePreview) => focus({ kind: "explore", ref: preview }),
    [focus],
  );

  const settings = useSettingsState({
    t,
    reporter,
    onManagedSkillsChanged: library.loadManagedSkills,
  });
  const explore = useExploreState({
    t,
    reporter,
    onOpenPreview: openExplorePreview,
  });
  const addFlow = useAddSkillFlow({ t, reporter, sync, library, surface });

  // My Skills' list lens: query, sort, Grouped/Flat, view mode.
  const lens = useListLens({
    items: library.managedSkills,
    labels: {
      local: t("localGroup"),
      ungrouped: t("ungrouped"),
      individual: t("individualSources"),
    },
    groupByRepoPreference,
    viewModePreference,
    selection: surface.selection.skills,
  });

  const {
    loading,
    loadingStartAt,
    actionMessage,
    cancelLoading,
    notify,
    notifyError,
    showActionErrors,
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

  const handleOpenSettings = useCallback(() => {
    open("settings");
  }, [open]);

  const handleCloseSettings = useCallback(() => {
    open("skills");
  }, [open]);

  // Opening the history marks it read.
  const handleOpenNotifications = useCallback(() => {
    markAllRead();
    openDialog({ kind: "notifications" });
  }, [markAllRead, openDialog]);

  const handleCloseNotifications = useCallback(() => {
    closeDialog("notifications");
  }, [closeDialog]);

  // Explore loads its lists whenever it becomes the area — from the tab, a
  // link or Back alike.
  const { loadFeaturedSkills, loadHiddenSkills } = explore;
  useEffect(() => {
    if (area !== "explore") return;
    void (async () => {
      await Promise.all([loadFeaturedSkills(), loadHiddenSkills()]);
    })();
    // Keyed on the area alone: `loadFeaturedSkills` changes identity once
    // its list lands, which must not trigger a second hidden-list fetch.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [area]);

  // What the detail shows: the focused Managed skill, or the Explore preview.
  const focusedSkill =
    focused?.kind === "skill"
      ? (library.managedSkills.find((skill) => skill.id === focused.id) ?? null)
      : null;
  const explorePreview = focused?.kind === "explore" ? focused.ref : null;
  const detailSubject = useMemo<DetailSubject | null>(
    () =>
      explorePreview
        ? { kind: "explore", preview: explorePreview }
        : focusedSkill
          ? { kind: "managed", skill: focusedSkill }
          : null,
    [explorePreview, focusedSkill],
  );

  const handleViewChange = useCallback(
    (next: Exclude<Area, "settings">) => {
      open(next);
    },
    [open],
  );

  const handleOpenDetail = useCallback(
    (skill: ManagedSkill) => {
      focus({ kind: "skill", id: skill.id });
    },
    [focus],
  );

  const { handleExploreInstall } = addFlow;
  const handleExploreInstallFromDetail = useCallback(() => {
    if (!explorePreview) return;
    handleExploreInstall(explorePreview.sourceUrl);
    unfocus();
  }, [explorePreview, handleExploreInstall, unfocus]);

  // "Sync all to the new tools" spans two worlds — enable the targets (sync)
  // and push every managed skill (library) — so the binder composes it.
  const {
    relevantNewlyInstalled,
    effectiveSyncTargetIds,
    enableTargetsFor,
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
    dismissDialog("newTools");
    // Zero work is reported, never silent: every newly detected tool sat
    // outside the operator's selection, so nothing was deployed.
    if (targets.length === 0) {
      notify("warning", t("noSyncTargets"));
      return;
    }
    enableTargetsFor(targets);
    void syncAllManagedToTools(targets);
  }, [
    dismissDialog,
    effectiveSyncTargetIds,
    enableTargetsFor,
    notify,
    relevantNewlyInstalled,
    t,
    syncAllManagedToTools,
  ]);

  // The two asks render from the surface stack and answer through it.
  const sharedDirAsk = surface.pendingAsk("sharedDir");
  const overwriteAsk = surface.pendingAsk("overwrite");

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
        loading={loading}
        activeArea={area}
        unreadNotifications={unreadCount}
        onOpenNotifications={handleOpenNotifications}
        onOpenSettings={handleOpenSettings}
        onViewChange={handleViewChange}
        t={t}
      />

      <main className="skills-main">
        {detailSubject ? (
          <SkillDetailView
            subject={detailSubject}
            onRepoint={library.handleRepointSkill}
            actionLoading={loading}
            onBack={unfocus}
            invokeTauri={invokeTauri}
            notify={notify}
            t={t}
            onInstall={
              detailSubject.kind === "explore"
                ? handleExploreInstallFromDetail
                : undefined
            }
          />
        ) : area === "skills" ? (
          <div className="dashboard-stack">
            <FilterBar
              lens={lens}
              loading={loading}
              onAddSkill={addFlow.handleOpenAdd}
              onRefresh={library.handleRefresh}
              autoSyncEnabled={sync.autoSyncEnabled}
              onAutoSyncChange={sync.handleAutoSyncToggle}
              onUnsyncAll={library.handleUnsyncAll}
              onConfigureTools={sync.handleOpenToolConfig}
              t={t}
            />
            <SkillsList
              plan={addFlow.plan}
              lens={lens}
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
        ) : area === "settings" ? (
          <SettingsPage
            isTauri={isTauri}
            storagePath={settings.storagePath}
            gitCacheCleanupDays={settings.gitCacheCleanupDays}
            gitCacheTtlSecs={settings.gitCacheTtlSecs}
            bounds={settings.bounds}
            themePreference={settings.themePreference}
            zoomLevel={settings.zoomLevel}
            onPickStoragePath={settings.handlePickStoragePath}
            onThemeChange={settings.handleThemeChange}
            onZoomLevelChange={settings.handleZoomLevelChange}
            onGitCacheCleanupDaysChange={settings.handleGitCacheCleanupDaysChange}
            onGitCacheTtlSecsChange={settings.handleGitCacheTtlSecsChange}
            onClearGitCacheNow={settings.handleClearGitCacheNow}
            onOpenLogFolder={settings.handleOpenLogFolder}
            githubTokenSet={settings.githubTokenSet}
            githubTokenDraft={settings.githubTokenDraft}
            githubTokenPending={settings.githubTokenPending}
            onGithubTokenDraftChange={settings.handleGithubTokenDraftChange}
            onGithubTokenSave={settings.handleGithubTokenSave}
            onGithubTokenRemove={settings.handleGithubTokenRemove}
            onBack={handleCloseSettings}
            t={t}
          />
        ) : area === "projects" ? (
          <ProjectsPage
            surface={surface}
            notify={notify}
            notifyError={notifyError}
            showActionErrors={showActionErrors}
            toolLabelById={sync.toolLabelById}
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
        open={surface.isOpen("notifications")}
        notifications={notifications}
        onRequestClose={handleCloseNotifications}
        onClear={clearNotifications}
        copyToClipboard={copyToClipboard}
        t={t}
      />

      <SharedDirModal
        pending={sharedDirAsk}
        loading={loading}
        onCancel={() => sharedDirAsk?.answer("declined")}
        t={t}
      />

      <OverwriteModal
        pending={overwriteAsk}
        t={t}
      />

      <ToolConfigModal
        open={surface.isOpen("toolConfig")}
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
        open={surface.isOpen("newTools") && Boolean(sync.newlyInstalledToolsText)}
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

      {library.pendingRepointSkill && library.repointSelection ? (
        <ChangeSourceModal
          key={`${library.pendingRepointSkill.id}:${library.repointSelection.preselect}`}
          skillName={library.pendingRepointSkill.name}
          preselect={library.repointSelection.preselect}
          loading={loading}
          onRequestClose={library.handleCloseRepoint}
          onPickFolder={library.pickRepointFolder}
          onConfirm={library.handleConfirmRepoint}
          t={t}
        />
      ) : null}

      <DeleteModal
        open={surface.isOpen("deleteSkill")}
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

      <Dialog
        open={surface.isOpen("update") && Boolean(updateAvailableVersion)}
        dismissible={!updateInstalling}
        onOpenChange={(next) => {
          if (!next) dismissUpdate();
        }}
      >
        <DialogContent
          size="sm"
          showCloseButton={!updateDone}
          aria-labelledby={updateTitleId}
        >
          <div className="update-modal">
            <div className="update-modal-body">
              <div className="update-modal-title" id={updateTitleId}>
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
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}

export default App;
