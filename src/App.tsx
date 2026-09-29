import { useCallback, useEffect, useLayoutEffect, useMemo, useRef } from "react";
import "./App.css";
import { useTranslation } from "react-i18next";
// The binder mounts the token-themed Toaster once; useStatusReporter makes
// every toast call and owns toast lifetime.
import { Toaster } from "@/components/ui/sonner";
import { mainLandmarkFocus } from "@/components/ui/focus-fallback";
import ExplorePage from "./components/skills/ExplorePage";
import FilterBar from "./components/skills/FilterBar";
import SkillDetailView, { type DetailSubject } from "./components/skills/SkillDetailView";
import AppSidebar from "./components/shell/AppSidebar";
import LibraryHeader from "./components/skills/LibraryHeader";
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
import UpdateDialog from "./components/shared/UpdateDialog";
import SettingsPage from "./components/skills/SettingsPage";
import ProjectsPage from "./components/projects/ProjectsPage";
import { useProjectState } from "./components/projects/useProjectState";
import { useAddSkillFlow } from "./hooks/useAddSkillFlow";
import { useExploreState } from "./hooks/useExploreState";
import { useSettingsState } from "./hooks/useSettingsState";
import { useSkillLibrary } from "./hooks/useSkillLibrary";
import { useSkillActions } from "./hooks/useSkillActions";
import { useStatusReporter } from "./hooks/useStatusReporter";
import { useSyncOrchestration } from "./hooks/useSyncOrchestration";
import { useToolCatalog } from "./hooks/useToolCatalog";
import { useAppUpdate } from "./hooks/useAppUpdate";
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
import { describeRefusal, resolveFocusTarget } from "./lib/actionLifecycle";
import { SKILL_FILTERS, libraryExceptions } from "./lib/skillPresentation";
import type { ManagedSkill } from "./components/skills/types";

// App is the binder: it owns only i18n, composes the surface module (what is
// on screen) and the per-world hooks (each returning that world's data +
// actions), and wires their interfaces together. State logic lives in
// src/hooks/, not here.
function App() {
  const { t } = useTranslation();

  // Surface state first: every world asks it to open and close surfaces.
  const surface = useSurfaceState();

  // World hooks, wired in dependency order: reporter → update → Tool
  // catalog → sync → library → projects → settings/explore/addFlow. Hooks
  // never import each other; every cross-world need flows through the
  // interfaces passed here.
  const reporter = useStatusReporter(t);
  const update = useAppUpdate({ reporter, surface });
  const catalog = useToolCatalog({ t });
  const sync = useSyncOrchestration({ t, reporter, surface, catalog });
  const library = useSkillLibrary({ t, reporter, surface });
  // A focused or selected skill that leaves the catalog clears itself.
  useFocusReconciliation(surface, "skill", library.managedSkillIds);
  // The project world lives as long as the app: its selected project, matrix
  // and the Tool catalog's project entries survive navigation. Its skill
  // rows are the library's catalog, passed to its view below.
  const projects = useProjectState({ settleCollisions: sync.settleProjectCollisions });

  const { area, focused, focus, unfocus, open, openDialog, closeDialog, dismissDialog } =
    surface;
  const openExplorePreview = useCallback(
    (preview: ExplorePreview) => focus({ kind: "explore", ref: preview }),
    [focus],
  );
  // Where the operator is now, for an intent that settles later (a queued
  // Explore preview opens only while Explore is still showing).
  const areaRef = useRef(area);
  useLayoutEffect(() => {
    areaRef.current = area;
  });
  const exploreIsActive = useCallback(() => areaRef.current === "explore", []);

  const settings = useSettingsState({
    t,
    reporter,
    onManagedSkillsChanged: library.loadManagedSkills,
  });
  const explore = useExploreState({
    t,
    reporter,
    onOpenPreview: openExplorePreview,
    isActive: exploreIsActive,
  });
  const reviewIsWanted = useCallback(() => areaRef.current === "skills", []);
  const addFlow = useAddSkillFlow({
    t,
    reporter,
    sync,
    catalog,
    library,
    surface,
    isReviewWanted: reviewIsWanted,
  });

  // My Skills' list lens: query, filter chips, sort, Grouped/Flat (single-
  // skill sources fold into "Individual sources"), view mode.
  const lens = useListLens({
    items: library.managedSkills,
    labels: {
      local: t("localGroup"),
      ungrouped: t("ungrouped"),
      individual: t("individualSources"),
    },
    groupByRepoPreference,
    viewModePreference,
    foldSingletons: true,
    filters: SKILL_FILTERS,
    selection: surface.selection.skills,
  });

  // The header's exception summary: the health fold over the whole library.
  const exceptions = useMemo(() => libraryExceptions(library.managedSkills), [library.managedSkills]);

  // The Synced strip's Tools: the effective target set, in registry order,
  // labelled by the Tool catalog (never a raw key).
  const { tools: catalogTools, toolLabelById, installedToolIds } = catalog;
  const { effectiveSyncTargetIds: stripTargetIds } = sync;
  const stripTools = useMemo(() => {
    const wanted = new Set(stripTargetIds);
    const known = catalogTools.filter((tool) => wanted.has(tool.id));
    const knownIds = new Set(known.map((tool) => tool.id));
    const unknown = stripTargetIds
      .filter((id) => !knownIds.has(id))
      .map((id) => ({ id, label: toolLabelById[id] ?? id }));
    return [...known.map((tool) => ({ id: tool.id, label: tool.label })), ...unknown];
  }, [catalogTools, stripTargetIds, toolLabelById]);
  const labelOfTool = useCallback((id: string) => toolLabelById[id] ?? id, [toolLabelById]);
  const isToolDetected = useCallback((id: string) => installedToolIds.includes(id), [installedToolIds]);
  const { toggle: toggleSelection } = surface;
  const toggleSkillSelection = useCallback((id: string) => toggleSelection("skills", id), [toggleSelection]);

  // The skill action door: every verb on a skill (card, detail, toolbar,
  // new-tools notice) goes through its one handle. Delete's focus rule reads
  // the list's display order (a grouped list walks its groups).
  const visibleOrder = useMemo(() => lens.displayed.map((skill) => skill.id), [lens.displayed]);
  const actions = useSkillActions({
    t,
    reporter,
    sync,
    catalog,
    surface,
    library,
    projects,
    order: visibleOrder,
  });

  const {
    loading,
    pending,
    refusal,
    actionMessage,
    cancelPending,
    runAction,
    applyOutcome,
    notify,
    notifyError,
    copyToClipboard,
    notifications,
    unreadCount,
    markAllRead,
    clearNotifications,
  } = reporter;
  // One mutation at a time (D19): while one runs, every verb is disabled
  // with the reason in words.
  const blockedReason = useMemo(
    () => (refusal ? describeRefusal(refusal, t) : null),
    [refusal, t],
  );

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

  // The Projects tab returns to the project the world still holds (its
  // matrix is loaded), as a real navigation; every other tab opens its area.
  const { selectedProjectId } = projects;
  const handleViewChange = useCallback(
    (next: Exclude<Area, "settings">) => {
      if (next === "projects" && selectedProjectId)
        focus({ kind: "project", id: selectedProjectId });
      else open(next);
    },
    [focus, open, selectedProjectId],
  );

  // Opening a skill from its row moves focus into what replaced the list
  // (the detail's first control until ticket 12's inspector), never <body>.
  const openedFromRow = useRef(false);
  const handleOpenDetail = useCallback(
    (skill: ManagedSkill) => {
      openedFromRow.current = true;
      focus({ kind: "skill", id: skill.id });
    },
    [focus],
  );

  // Leaving a skill — Back, Escape or browser history — returns focus to its
  // row (D21: the sheet closes → the focused row), when focus was left inside
  // the closed detail (now <body>).
  const focusedSkillId = focused?.kind === "skill" ? focused.id : null;
  const handleCloseDetail = unfocus;
  const lastFocusedSkill = useRef<string | null>(null);
  useEffect(() => {
    const left = lastFocusedSkill.current;
    lastFocusedSkill.current = focusedSkillId;
    if (focusedSkillId) {
      const fromRow = openedFromRow.current;
      openedFromRow.current = false;
      // From the row, or arriving by history with focus lost to <body>.
      if (!fromRow && document.activeElement && document.activeElement !== document.body) return;
      const main = document.querySelector("main");
      (main?.querySelector<HTMLElement>("button, a[href]") ?? mainLandmarkFocus())?.focus();
      return;
    }
    if (!left || (document.activeElement && document.activeElement !== document.body)) return;
    resolveFocusTarget({ kind: "row", skillId: left })?.focus();
  }, [focusedSkillId]);
  const layerOpen = surface.stack.length > 0;
  useEffect(() => {
    if (!focusedSkillId || layerOpen) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented) return;
      if (document.querySelector("[role='dialog'], [role='alertdialog'], [role='menu']")) return;
      handleCloseDetail();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [focusedSkillId, handleCloseDetail, layerOpen]);

  const { handleOpenAdd, setAddModalTab } = addFlow;
  const handleAddGit = useCallback(() => {
    setAddModalTab("git");
    handleOpenAdd();
  }, [handleOpenAdd, setAddModalTab]);
  const handleAddLocal = useCallback(() => {
    setAddModalTab("local");
    handleOpenAdd();
  }, [handleOpenAdd, setAddModalTab]);
  const handleOpenExplore = useCallback(() => handleViewChange("explore"), [handleViewChange]);
  const { setFilter: setLensFilter } = lens;
  const handleViewIssues = useCallback(() => setLensFilter("issues"), [setLensFilter]);
  const librarySize = library.managedSkills.length;
  // Reviewing discovered skills is the first task while the discovery
  // outweighs the library (first run); it then carries the one primary.
  const discoveryLeads = (addFlow.plan?.group_count ?? 0) >= Math.max(1, librarySize);

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
  const { autoSyncEnabled } = sync;
  const { run } = actions;
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
    // With auto-sync off the new Tools are only selected, never written to.
    if (autoSyncEnabled) void run("syncToTargets", "all", { tools: targets });
  }, [
    autoSyncEnabled,
    dismissDialog,
    effectiveSyncTargetIds,
    enableTargetsFor,
    notify,
    relevantNewlyInstalled,
    run,
    t,
  ]);

  // The two asks render from the surface stack and answer through it.
  const sharedDirAsk = surface.pendingAsk("sharedDir");
  const overwriteAsk = surface.pendingAsk("overwrite");

  return (
    <div className="flex h-full overflow-hidden bg-background text-foreground">
      {/* Toast lifetime is owned by useStatusReporter, per kind. Errors
          never auto-dismiss, so the visible stack is raised above sonner's
          default of 3: a fourth open error shows without hovering. */}
      <Toaster position="top-right" richColors visibleToasts={5} />
      <LoadingOverlay
        pending={pending}
        actionMessage={actionMessage}
        onCancel={cancelPending}
        t={t}
      />

      <AppSidebar
        activeArea={area}
        onViewChange={handleViewChange}
        onOpenSettings={handleOpenSettings}
        unreadNotifications={unreadCount}
        onOpenNotifications={handleOpenNotifications}
        themePreference={settings.themePreference}
        onThemeChange={settings.handleThemeChange}
        t={t}
      />

      <main className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
        {detailSubject ? (
          <SkillDetailView
            subject={detailSubject}
            actions={actions}
            blockedReason={blockedReason}
            onBack={detailSubject.kind === "managed" ? handleCloseDetail : unfocus}
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
          <>
            <LibraryHeader
              librarySize={librarySize}
              exceptions={exceptions}
              onViewIssues={handleViewIssues}
              blockedReason={blockedReason}
              onRefreshAll={() => void run("update", "all")}
              onConfigureTools={sync.handleOpenToolConfig}
              onAddSkill={handleOpenAdd}
              addIsPrimary={!discoveryLeads}
              t={t}
            />
            {librarySize > 0 ? <FilterBar lens={lens} t={t} /> : null}
            <SkillsList
              lens={lens}
              librarySize={librarySize}
              plan={addFlow.plan}
              stripTools={stripTools}
              labelOf={labelOfTool}
              isDetected={isToolDetected}
              selection={surface.selection.skills}
              onToggleSelect={toggleSkillSelection}
              pending={pending}
              blockedReason={blockedReason}
              actions={actions}
              onOpen={handleOpenDetail}
              onReviewImport={addFlow.handleReviewImport}
              discoveryLeads={discoveryLeads}
              onAddGit={handleAddGit}
              onAddLocal={handleAddLocal}
              onExplore={handleOpenExplore}
              t={t}
            />
          </>
        ) : area === "settings" ? (
          <SettingsPage
            isTauri={isTauri}
            update={update}
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
            autoSyncEnabled={sync.autoSyncEnabled}
            onAutoSyncChange={sync.handleAutoSyncToggle}
            autoSyncBlockedReason={blockedReason}
            onBack={handleCloseSettings}
            t={t}
          />
        ) : area === "projects" ? (
          <ProjectsPage
            state={projects}
            skills={library.managedSkills}
            catalog={catalog}
            surface={surface}
            pending={pending}
            runAction={runAction}
            applyOutcome={applyOutcome}
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
        installedTools={catalog.installedTools}
        toolStatus={catalog.toolStatus}
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
        toolStatus={catalog.toolStatus}
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
            if (library.invocationEditSkill) void run("editInvocation", [library.invocationEditSkill], { mode });
          }}
          t={t}
        />
      ) : null}

      {library.pendingRepointSkill && library.repointSelection ? (
        <ChangeSourceModal
          key={`${library.pendingRepointSkill.id}:${library.repointSelection.preselect}`}
          skillName={library.pendingRepointSkill.name}
          source={library.pendingRepointSkill}
          preselect={library.repointSelection.preselect}
          loading={loading}
          onRequestClose={library.handleCloseRepoint}
          onPickFolder={library.pickRepointFolder}
          onConfirm={(target) => {
            if (library.pendingRepointSkill) void run("repoint", [library.pendingRepointSkill], { target });
          }}
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
            void run("delete", [library.pendingDeleteSkill], { confirmed: true });
        }}
        fallbackFocus={actions.dialogFallbackFocus}
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

      <UpdateDialog open={surface.isOpen("update")} update={update} t={t} />
    </div>
  );
}

export default App;
