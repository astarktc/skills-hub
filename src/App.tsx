import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { flushSync } from "react-dom";
import { useTranslation } from "react-i18next";
// The binder mounts the token-themed Toaster once; useStatusReporter makes
// every toast call and owns toast lifetime.
import { Toaster } from "@/components/ui/sonner";
import { mainLandmarkFocus } from "@/components/ui/focus-fallback";
import ExplorePage from "./components/skills/ExplorePage";
import FilterBar from "./components/skills/FilterBar";
import SkillInspector, { type InspectorPlacement } from "./components/skills/inspector/SkillInspector";
import { useInspectorTabOrder } from "./components/skills/inspector/useInspectorTabOrder";
import AppSidebar from "./components/shell/AppSidebar";
import HeaderProgress from "./components/shell/HeaderProgress";
import CommandPalette from "./components/shell/CommandPalette";
import LibraryHeader from "./components/skills/LibraryHeader";
import SkillsList from "./components/skills/SkillsList";
import NotificationsModal from "./components/shared/NotificationsModal";
import AddSkillDialog from "./components/skills/add/AddSkillDialog";
import DeleteModal from "./components/skills/modals/DeleteModal";
import CountConfirmModal from "./components/skills/modals/CountConfirmModal";
import BulkRail from "./components/skills/BulkRail";
import { useSkillListKeymap } from "./components/skills/useSkillListKeymap";
import RepointModal from "./components/skills/modals/RepointModal";
import DetachModal from "./components/skills/modals/DetachModal";
import AssignProjectModal from "./components/skills/modals/AssignProjectModal";
import InvocationModeModal from "./components/skills/modals/InvocationModeModal";
import ImportReviewDialog from "./components/skills/import/ImportReviewDialog";
import NewToolsModal from "./components/skills/modals/NewToolsModal";
import SharedDirModal from "./components/skills/modals/SharedDirModal";
import OverwriteModal from "./components/skills/modals/OverwriteModal";
import UpdateDialog from "./components/shared/UpdateDialog";
import SettingsDialog from "./components/settings/SettingsDialog";
import ShortcutsDialog from "./components/settings/ShortcutsDialog";
import DeployHarnessAsk from "./components/settings/DeployHarnessAsk";
import MoveLibraryAsk from "./components/settings/MoveLibraryAsk";
import ProjectsPage from "./components/projects/ProjectsPage";
import { useProjectState } from "./components/projects/useProjectState";
import { useAddSkillFlow } from "./hooks/useAddSkillFlow";
import { useExploreState } from "./hooks/useExploreState";
import { useSettingsState } from "./hooks/useSettingsState";
import { useSkillLibrary } from "./hooks/useSkillLibrary";
import { useSkillActions } from "./hooks/useSkillActions";
import { useStatusReporter } from "./hooks/useStatusReporter";
import { useHeldRefusal } from "./hooks/useHeldRefusal";
import { useSyncOrchestration } from "./hooks/useSyncOrchestration";
import { useToolCatalog } from "./hooks/useToolCatalog";
import { useHarnessSelection } from "./hooks/useHarnessSelection";
import { useNewHarnessesDeploy } from "./hooks/useNewHarnessesDeploy";
import { useRedeploy } from "./hooks/useRedeploy";
import RedeployOfferDialog from "./components/settings/RedeployOfferDialog";
import { useAppUpdate } from "./hooks/useAppUpdate";
import { useListLens } from "./hooks/useListLens";
import { SHELL_WIDE_QUERY, useMediaQuery } from "./hooks/useMediaQuery";
import {
  useFocusReconciliation,
  useSurfaceState,
  type Area,
  type ExplorePreview,
} from "./hooks/useSurfaceState";
import {
  collapsedSourcesPreference,
  groupByRepoPreference,
  viewModePreference,
} from "./lib/preferences";
import { isTauri } from "./lib/tauri";
import { revealPath } from "./lib/platform";
import type { AssignmentsOf } from "./lib/bulkCounts";
import {
  describeRefusal,
  dialogFallback,
  resolveFirst,
  resolveFocusTarget,
} from "./lib/actionLifecycle";
import { SKILL_FILTERS, libraryExceptions, problemRank } from "./lib/skillPresentation";
import { openKeyboardShortcuts, openSettingsPane } from "./lib/commandPalette";
import type { SettingsPane, ShortcutPane } from "./lib/settingsPanes";
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
  // The reporter reads the surface for the Notifications panel: a toast
  // never covers it, and "View report" opens it on a run.
  const reporter = useStatusReporter(t, surface);
  // A held control's activation (click, Space, Enter) says why it is
  // refused: the primitives' event becomes the reporter's notice (ticket 27).
  useHeldRefusal(reporter.refuseHeld);
  const update = useAppUpdate({ reporter, surface });
  const catalog = useToolCatalog({ t });
  const sync = useSyncOrchestration({ t, reporter, surface, catalog });
  const library = useSkillLibrary({ t, reporter, surface });
  // A focused or selected skill that leaves the catalog clears itself.
  useFocusReconciliation(surface, "skill", library.managedSkillIds);
  // The project world lives as long as the app: its selected project, matrix
  // and the Tool catalog's project entries survive navigation. Its skill
  // rows are the library's catalog, passed to its view below.
  // The library's rows carry each skill's project deployments (D41): a
  // project mutation reloads the catalog so the two worlds agree (31 audit F1).
  const projects = useProjectState({
    settleCollisions: sync.settleProjectCollisions,
    onDeploymentsChanged: library.loadManagedSkills,
  });

  const { area, focused, focus, unfocus, open, closeDialog, dismissDialog, openDialog, updateDialog } =
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

  const settings = useSettingsState({ t, reporter, surface, library, projects });
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

  // My Skills' list lens: query, filter chips, sort (Problems first reads the
  // health fold's rank), Grouped/Flat (single-skill sources fold into
  // "Individual sources"), view mode.
  const lens = useListLens({
    items: library.managedSkills,
    labels: {
      local: t("localGroup"),
      managed: t("managedGroup"),
      ungrouped: t("ungrouped"),
      individual: t("individualSources"),
    },
    groupByRepoPreference,
    viewModePreference,
    collapsePreference: collapsedSourcesPreference,
    foldSingletons: true,
    problemRank,
    filters: SKILL_FILTERS,
    selection: surface.selection.skills,
  });

  // The header's exception summary: the health fold over the whole library.
  const exceptions = useMemo(() => libraryExceptions(library.managedSkills), [library.managedSkills]);

  // The Synced strip's Tools: the effective target set, in registry order,
  // labelled by the Tool catalog (never a raw key).
  const { tools: catalogTools, toolLabelById, installedToolIds, toolStatus, isFolderDetected } = catalog;
  const { effectiveSyncTargetIds: stripTargetIds } = sync;
  const stripTools = useMemo(() => {
    const wanted = new Set(stripTargetIds);
    const known = catalogTools.filter((tool) => wanted.has(tool.id));
    const knownIds = new Set(known.map((tool) => tool.id));
    const unknown = stripTargetIds
      .filter((id) => !knownIds.has(id))
      .map((id) => ({ id, label: toolLabelById[id] ?? id, folder: id }));
    return [...known.map((tool) => ({ id: tool.id, label: tool.label, folder: tool.folder })), ...unknown];
  }, [catalogTools, stripTargetIds, toolLabelById]);
  const labelOfTool = useCallback((id: string) => toolLabelById[id] ?? id, [toolLabelById]);
  const toolInfos = useMemo(() => toolStatus?.tools ?? [], [toolStatus]);
  const isToolDetected = useCallback((id: string) => installedToolIds.includes(id), [installedToolIds]);

  // The skill action door: every verb on a skill (card, detail, toolbar,
  // new-tools notice) goes through its one handle. Delete's focus rule reads
  // the list's display order (a grouped list walks its groups; a collapsed
  // group's rows are not on screen, so neither focus nor ⌘A lands there).
  const visibleOrder = useMemo(() => lens.onScreen.map((skill) => skill.id), [lens.onScreen]);
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

  // Settings → Harnesses' switch (D48): save, then — auto-deploy on — ask once.
  const harnessSelection = useHarnessSelection({
    sync,
    installed: installedToolIds,
    catalog: toolInfos,
    skills: library.managedSkills,
    labelOf: labelOfTool,
    surface,
    actions,
    reporter,
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
    openReport,
    clearNotifications,
  } = reporter;
  // One mutation at a time (D19): while one runs, every verb is disabled
  // with the reason in words.
  const blockedReason = useMemo(
    () => (refusal ? describeRefusal(refusal, t) : null),
    [refusal, t],
  );

  // Settings is a dialog (D37): every way in takes `openSettingsPane`; a
  // pane change is dialog state, never the hash.
  const surfaceDoor = useMemo(() => ({ openDialog }), [openDialog]);
  const handleOpenSettings = useCallback(() => openSettingsPane(surfaceDoor, null), [surfaceDoor]);
  const handleOpenHarnesses = useCallback(() => openSettingsPane(surfaceDoor, "harnesses"), [surfaceDoor]);
  const handleSettingsPane = useCallback(
    (pane: SettingsPane) => updateDialog({ kind: "settings", pane }),
    [updateDialog],
  );
  const handleCloseSettings = useCallback(() => closeDialog("settings"), [closeDialog]);
  const handleOpenShortcuts = useCallback(() => openKeyboardShortcuts(surfaceDoor), [surfaceDoor]);
  const handleShortcutsPane = useCallback(
    (pane: ShortcutPane) => updateDialog({ kind: "shortcuts", pane }),
    [updateDialog],
  );
  const handleCloseShortcuts = useCallback(() => closeDialog("shortcuts"), [closeDialog]);

  // Opening the history marks it read (the reporter's `openReport`); what
  // was listed while it stood open counts as seen when it closes.
  const handleOpenNotifications = useCallback(() => openReport(), [openReport]);

  const handleCloseNotifications = useCallback(() => {
    markAllRead();
    closeDialog("notifications");
  }, [closeDialog, markAllRead]);

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

  // What the inspector shows: the focused Managed skill (a selection beside
  // the list, D3); an Explore preview takes the same slot beside Explore's rows.
  const focusedSkill =
    focused?.kind === "skill"
      ? (library.managedSkills.find((skill) => skill.id === focused.id) ?? null)
      : null;
  const explorePreview = focused?.kind === "explore" ? focused.ref : null;

  // The Projects tab returns to the project the world still holds (its
  // matrix is loaded), as a real navigation; every other tab opens its area.
  const { selectedProjectId } = projects;
  const handleViewChange = useCallback(
    (next: Area) => {
      if (next === "projects" && selectedProjectId)
        focus({ kind: "project", id: selectedProjectId });
      else open(next);
    },
    [focus, open, selectedProjectId],
  );

  // The inspector (D3, D23): beside the list at and above the shell
  // breakpoint, a sheet over it below. Opening from a row leaves focus on the
  // row (Tab enters the inspector after it); arriving without a row (a deep
  // link, history) — or a resize that reparents a focused inspector — puts
  // focus on the inspector's heading; closing returns it to the row.
  const wide = useMediaQuery(SHELL_WIDE_QUERY);
  const wantedPlacement: InspectorPlacement = wide ? "side" : "sheet";
  // A presentation change never reorders layers (I2): while any dialog is
  // open above the inspector, it keeps its place, so the sheet never
  // registers above a child it opened. The change lands once the last layer
  // has closed, after that layer's own focus return (a microtask of the
  // commit that closed it). Progress is the header's, never a layer (D19).
  const layerAboveInspector = surface.stack.length > 0;
  const [placement, setPlacement] = useState<InspectorPlacement>(wantedPlacement);
  //
  // Focus inside the inspector crosses the reparent in one task (14 audit
  // N2): the old placement unmounts and the new one mounts synchronously, and
  // its heading takes focus before the task ends, so no frame — and no
  // sample — ever finds it on <body> between the two. The sheet's own initial
  // focus (the same heading) and the aside's hand-over below then agree.
  const reparentWithFocus = useCallback((next: InspectorPlacement, skillId: string | null) => {
    const active = document.activeElement;
    const held = active instanceof Element && active.closest("[data-inspector]") !== null;
    flushSync(() => setPlacement(next));
    if (!held || !skillId) return;
    const inside = document.activeElement?.closest("[data-inspector]");
    if (!inside) {
      document.querySelector<HTMLElement>(`[data-inspector-heading="${CSS.escape(skillId)}"]`)?.focus();
    }
  }, []);
  const placementSubject = focused?.kind === "skill" ? focused.id : null;
  useEffect(() => {
    if (layerAboveInspector || placement === wantedPlacement) return;
    const timer = window.setTimeout(() => reparentWithFocus(wantedPlacement, placementSubject), 0);
    return () => window.clearTimeout(timer);
  }, [layerAboveInspector, placement, wantedPlacement, reparentWithFocus, placementSubject]);
  const focusedSkillId = focused?.kind === "skill" ? focused.id : null;
  const handleCloseDetail = unfocus;
  const openedFromRow = useRef(false);
  const handleOpenDetail = useCallback(
    (skill: ManagedSkill) => {
      openedFromRow.current = true;
      focus({ kind: "skill", id: skill.id });
    },
    [focus],
  );
  const focusInspectorHeading = useCallback((skillId: string) => {
    const heading = document.querySelector<HTMLElement>(`[data-inspector-heading="${CSS.escape(skillId)}"]`);
    heading?.focus();
    return heading !== null;
  }, []);

  // Files expanded to the full width, per skill: another skill opens collapsed.
  const [expandedFilesOf, setExpandedFilesOf] = useState<string | null>(null);
  const filesExpanded = focusedSkillId !== null && expandedFilesOf === focusedSkillId;
  const filesCover = filesExpanded && placement === "side";
  const setFilesExpanded = useCallback(
    (expanded: boolean) => setExpandedFilesOf(expanded ? focusedSkillId : null),
    [focusedSkillId],
  );
  // Escape's first step while Files is expanded (beside the list and in the
  // sheet alike): collapse, and focus its toggle once it is back in place.
  const collapseFiles = useCallback(() => {
    flushSync(() => setExpandedFilesOf(null));
    document.querySelector<HTMLElement>("[data-files-toggle]")?.focus();
  }, []);

  // The operator's last key or pointer press: a focus placement the app
  // makes on its own never overrides one made after it.
  const lastInteraction = useRef(0);
  useEffect(() => {
    const mark = () => {
      lastInteraction.current += 1;
    };
    window.addEventListener("keydown", mark, true);
    window.addEventListener("pointerdown", mark, true);
    return () => {
      window.removeEventListener("keydown", mark, true);
      window.removeEventListener("pointerdown", mark, true);
    };
  }, []);

  // Keyed on the skill the inspector shows (a deep link's skill appears once
  // the catalog lands), not on the address alone.
  const shownSkillId = focusedSkill?.id ?? null;
  const lastFocusedSkill = useRef<string | null>(null);
  useEffect(() => {
    const left = lastFocusedSkill.current;
    lastFocusedSkill.current = shownSkillId;
    // One placement, once this commit is painted: a layer that closed in the
    // same commit (the Notifications panel's Show, which returns focus
    // nowhere) has released the page by then. Nothing places it again: no
    // layer competes for focus (the progress is the header's, D19). Focus
    // the operator placed since — a key or a pointer — is never overridden,
    // and a later subject, placement or area disposes it.
    const areaAtStart = areaRef.current;
    const since = lastInteraction.current;
    const isLost = (active: Element | null) =>
      !active || active === document.body || active.tagName === "MAIN";
    const whenPainted = (place: () => void, override: boolean) =>
      window.requestAnimationFrame(() => {
        if (areaRef.current !== areaAtStart) return;
        const active = document.activeElement;
        const untouched = lastInteraction.current === since;
        const elsewhere = !active?.closest("[data-inspector], [role='dialog'], [role='alertdialog']");
        if (isLost(active) || (override && untouched && elsewhere)) place();
      });
    let frame = 0;
    if (shownSkillId) {
      const fromRow = openedFromRow.current;
      openedFromRow.current = false;
      // The sheet moves focus in itself (its heading); beside the list, a
      // row keeps it, and an arrival with focus nowhere takes the heading.
      if (placement === "side" && !fromRow) {
        frame = whenPainted(() => {
          if (!focusInspectorHeading(shownSkillId)) mainLandmarkFocus()?.focus();
        }, true);
      }
    } else if (left) {
      frame = whenPainted(() => resolveFocusTarget({ kind: "row", skillId: left })?.focus(), false);
    }
    return () => window.cancelAnimationFrame(frame);
  }, [focusInspectorHeading, shownSkillId, placement]);

  // A resize across the breakpoint reparents the inspector (aside ↔ sheet):
  // focus that was inside it goes to the new place's heading (D23).
  const focusWasInInspector = useRef(false);
  useEffect(() => {
    const onFocusIn = (event: FocusEvent) => {
      focusWasInInspector.current =
        event.target instanceof Element && event.target.closest("[data-inspector]") !== null;
    };
    document.addEventListener("focusin", onFocusIn);
    return () => document.removeEventListener("focusin", onFocusIn);
  }, []);
  const lastPlacement = useRef(placement);
  useEffect(() => {
    if (lastPlacement.current === placement) return;
    lastPlacement.current = placement;
    if (!focusedSkillId || !focusWasInInspector.current) return;
    // The sheet focuses its heading as it opens; the aside needs a hand
    // (cancelled if the subject or placement changes first).
    if (placement === "side") {
      const id = focusedSkillId;
      const timer = window.setTimeout(() => focusInspectorHeading(id), 0);
      return () => window.clearTimeout(timer);
    }
  }, [focusInspectorHeading, focusedSkillId, placement]);
  // The sheet closing returns to the focused row — unless it closed because
  // the window widened and the inspector moved beside the list. A row a
  // Delete removed hands over to that Delete's own target (D21: the next
  // row, the previous one, the heading).
  const { dialogFallbackFocus } = actions;
  const sheetFinalFocus = useCallback((): HTMLElement | boolean => {
    if (window.matchMedia?.(SHELL_WIDE_QUERY).matches) return false;
    const left = lastFocusedSkill.current ?? focusedSkillId;
    return (left ? resolveFocusTarget({ kind: "row", skillId: left }) : null) ?? dialogFallbackFocus() ?? true;
  }, [dialogFallbackFocus, focusedSkillId]);

  useInspectorTabOrder(focusedSkillId, placement === "side" && !filesExpanded);

  // A skill dialog whose opener is gone when it closes (Re-point, Detach and
  // Edit remove the control that opened them) returns focus by D21's chain:
  // the focused row, the inspector's heading, the surface heading — the
  // first the closing layer finds usable (inside the sheet the row stays
  // inert, so the sheet's heading).
  const skillDialogFallback = useCallback(
    (usable: (element: Element) => boolean) =>
      resolveFirst(dialogFallback({ focusedSkillId, surface: "skills" }), null, usable),
    [focusedSkillId],
  );

  // My Skills' one keymap (j/k · x · ⌘A · / · Esc). Escape clears the
  // selection first; then, beside the list, it collapses expanded Files and
  // then closes the inspector (the sheet's Escape is the Dialog's own). While
  // expanded Files cover the list it stays on for Escape alone: the covered
  // rows are inert, so their shortcuts are refused.
  const layerOpen = surface.stack.length > 0;
  const escapeStep = useCallback(() => {
    if (!focusedSkillId || placement !== "side") return;
    if (filesExpanded) collapseFiles();
    else handleCloseDetail();
  }, [collapseFiles, filesExpanded, focusedSkillId, handleCloseDetail, placement]);
  const { selectFromRow, selectGroup, clear: clearSkillSelection } = useSkillListKeymap({
    enabled: area === "skills" && !explorePreview && !layerOpen,
    listCovered: filesCover,
    order: visibleOrder,
    surface,
    focusedSkillId,
    onEscape: escapeStep,
  });

  // The selection the bulk rail acts on (it may hold rows a filter hides).
  const selectedSkills = useMemo(
    () => library.managedSkills.filter((skill) => surface.selection.skills.has(skill.id)),
    [library.managedSkills, surface.selection.skills],
  );

  // Where each skill is assigned, for the counts a wide verb states (the
  // project world's held views; unknown until a read landed).
  const { placementOf, placementViews, loadPlacementViews, projects: projectList } = projects;
  const assignmentsOf = useMemo<AssignmentsOf | null>(
    () =>
      placementViews.loaded
        ? (skillId: string) =>
            placementOf(skillId).rows?.map((row) => ({
              projectId: row.project.id,
              projectName: row.project.name,
              assignments: row.assignments,
            })) ?? null
        : null,
    [placementOf, placementViews.loaded],
  );
  const projectLabelById = useMemo(
    () => Object.fromEntries(projectList.map((project) => [project.id, project.name])),
    [projectList],
  );
  const assignPlacements = useMemo(
    () => library.assignProjectSkills.map((skill) => placementOf(skill.id)),
    [library.assignProjectSkills, placementOf],
  );
  const handleReveal = useCallback(
    (path: string) => {
      void revealPath(path).catch(notifyError);
    },
    [notifyError],
  );

  // Project Tool labels for the inspector's Projects section and the picker.
  const { projectToolStatus, loadProjectToolStatus } = catalog;
  useEffect(() => {
    if (!focusedSkillId || projectToolStatus || !isTauri) return;
    void loadProjectToolStatus().catch(() => undefined);
  }, [focusedSkillId, loadProjectToolStatus, projectToolStatus]);

  // Add's intents (ticket 16): every way in opens the one stepped surface.
  const { open: openAdd } = addFlow;
  const handleOpenAdd = useCallback(() => openAdd(), [openAdd]);
  const handleAddGit = useCallback(() => openAdd({ kind: "git" }), [openAdd]);
  const handleAddLocal = useCallback(() => openAdd({ kind: "local" }), [openAdd]);
  // Explore's "install this": Add opens on the repository, listed, the named skill checked.
  const handleExploreInstall = useCallback(
    (sourceUrl: string, skillName?: string) =>
      openAdd({ source: { kind: "git", url: sourceUrl }, preselect: skillName }),
    [openAdd],
  );
  const handleOpenExplore = useCallback(() => handleViewChange("explore"), [handleViewChange]);
  const { setFilter: setLensFilter } = lens;
  const handleViewIssues = useCallback(() => setLensFilter("issues"), [setLensFilter]);
  // The empty Notifications panel's pointer (#74): My Skills, under the Issues lens.
  const handleViewIssuesFromPanel = useCallback(() => {
    handleViewChange("skills");
    setLensFilter("issues");
  }, [handleViewChange, setLensFilter]);
  const librarySize = library.managedSkills.length;
  // Reviewing discovered skills is the first task while the discovery
  // outweighs the library (first run); it then carries the one primary.
  const discoveryLeads = (addFlow.plan?.group_count ?? 0) >= Math.max(1, librarySize);

  // An installed Explore candidate is its Managed skill: the inspector, never a second Install.
  const handleOpenInstalledFromExplore = useCallback(
    (skillId: string) => focus({ kind: "skill", id: skillId }),
    [focus],
  );

  const { effectiveSyncTargetIds } = sync;
  const { run } = actions;
  // "Deploy all skills" to the new harnesses spans two worlds — enable the
  // targets (sync) and push every managed skill (library) — so the binder
  // composes it. An explicit Deploy always deploys, whatever Auto-deploy says (D34).
  const handleSyncAllNewTools = useNewHarnessesDeploy({ sync, surface, reporter, actions, t });
  // The redeploy (round 20 #108): the launch check's offer, the run and the
  // Harnesses pane's "Move N Deployments" line.
  const redeploy = useRedeploy({ t, reporter, surface, library, toolLabelById });
  const { pendingCount: redeployPending, run: runRedeploy } = redeploy;
  const redeployLine = useMemo(
    () => ({ pendingCount: redeployPending, onRun: () => void runRedeploy() }),
    [redeployPending, runRedeploy],
  );

  // The two asks render from the surface stack and answer through it.
  const sharedDirAsk = surface.pendingAsk("sharedDir");
  const overwriteAsk = surface.pendingAsk("overwrite");
  const deployHarnessAsk = surface.pendingAsk("deployHarness");
  const moveLibraryAsk = surface.pendingAsk("moveLibrary");

  // A dialog whose own action runs shows the header progress at its foot:
  // the page behind a modal is inert, so its Cancel must be the dialog's.
  const dialogProgress = (
    <HeaderProgress pending={pending} message={actionMessage} onCancel={cancelPending} placement="dialog" t={t} />
  );
  // While one of those dialogs is open its foot is the progress; the page
  // copy under the scrim yields, so a run never shows twice (14 audit N5).
  const progressInDialog =
    addFlow.add.open ||
    addFlow.importReview.open ||
    surface.isOpen("notifications") ||
    surface.isOpen("settings") ||
    (library.pendingRepointSkill !== null && library.repointSelection !== null) ||
    (area === "projects" &&
      (surface.isOpen("projectAdd") ||
        surface.isOpen("projectEdit") ||
        surface.isOpen("projectToolConfig") ||
        surface.isOpen("projectRemove")));
  // The panel's catalog: a row's skill actions exist only while its skill does.
  const skillNames = useMemo(
    () => Object.fromEntries(library.managedSkills.map((skill) => [skill.id, skill.name])),
    [library.managedSkills],
  );
  const reportRunId = surface.layer("notifications")?.runId ?? null;

  return (
    <div className="flex h-full overflow-hidden bg-background text-foreground">
      {/* Toast lifetime is owned by useStatusReporter, per kind. Errors
          never auto-dismiss, so the visible stack is raised above sonner's
          default of 3: a fourth open error shows without hovering.
          Placement (BACKLOG #80): the bottom-right corner, lifted 6 rem
          above the window's foot — clear of the header's verbs (the next
          action after a notice), the filter bar, the bulk rail and a
          dialog's or the inspector's foot; it passes over list content,
          which scrolls. Pointer events and hover-pause stay sonner's. */}
      <Toaster position="bottom-right" offset={{ bottom: "6rem" }} richColors visibleToasts={3} />

      <AppSidebar
        activeArea={area}
        onViewChange={handleViewChange}
        onOpenSettings={handleOpenSettings}
        onOpenShortcuts={handleOpenShortcuts}
        unreadNotifications={unreadCount}
        onOpenNotifications={handleOpenNotifications}
        themePreference={settings.themePreference}
        onThemeChange={settings.handleThemeChange}
        update={update}
        t={t}
      />

      <main className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
        {/* The header progress (D19) of an area without a header slot for
            it (My Skills has one): a strip above the area, never a layer. */}
        {area !== "skills" ? (
          <HeaderProgress
            pending={pending}
            message={actionMessage}
            onCancel={cancelPending}
            placement="bar"
            yielded={progressInDialog}
            t={t}
          />
        ) : null}
        {area === "skills" ? (
          <div className="relative flex min-h-0 flex-1 flex-col" data-skill-surface="">
            {/* Header and filters span My Skills; the inspector sits beside
                the list below them (its right third). Expanded Files covers
                the whole area in place: what it covers stays mounted — the
                list's scroll, the focused row — and inert. */}
            <div className="flex shrink-0 flex-col" inert={filesCover}>
              <LibraryHeader
                librarySize={librarySize}
                exceptions={exceptions}
                onViewIssues={handleViewIssues}
                blockedReason={blockedReason}
                pending={pending}
                actionMessage={actionMessage}
                onCancel={cancelPending}
                progressYielded={progressInDialog}
                onRefreshAll={() => void run("update", "all", { confirmed: true })}
                onConfigureTools={handleOpenHarnesses}
                onAddSkill={handleOpenAdd}
                addIsPrimary={!discoveryLeads}
                t={t}
              />
              {librarySize > 0 ? <FilterBar lens={lens} t={t} /> : null}
            </div>
            <div className="flex min-h-0 flex-1">
              <div className="flex min-h-0 min-w-0 flex-1 flex-col" inert={filesCover}>
                {/* The rail sits at the foot (`order-last`) but first in the
                    DOM: the selection's verbs are one Tab from the filter bar
                    and a short Shift+Tab from the rows, before the list. */}
                {selectedSkills.length > 0 ? (
                  <BulkRail
                    selected={selectedSkills}
                    actions={actions}
                    blockedReason={blockedReason}
                    onClear={clearSkillSelection}
                    t={t}
                  />
                ) : null}
                <SkillsList
                  lens={lens}
                  librarySize={librarySize}
                  plan={addFlow.plan}
                  landing={addFlow.importLanding}
                  stripTools={stripTools}
                  labelOf={labelOfTool}
                  isDetected={isToolDetected}
                  isFolderDetected={isFolderDetected}
                  selection={surface.selection.skills}
                  focusedId={focusedSkillId}
                  onToggleSelect={selectFromRow}
                  onSelectGroup={selectGroup}
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
              </div>
              {focusedSkill ? (
                <SkillInspector
                  key={`${focusedSkill.id}:${placement}`}
                  skill={focusedSkill}
                  placement={placement}
                  stripTools={stripTools}
                  labelOf={labelOfTool}
                  isDetected={isToolDetected}
                  isFolderDetected={isFolderDetected}
                  actions={actions}
                  blockedReason={blockedReason}
                  projectPlacement={projects.placementOf(focusedSkill.id)}
                  loadPlacementViews={projects.loadPlacementViews}
                  projectCount={projects.projects.length}
                  copyToClipboard={copyToClipboard}
                  filesExpanded={filesExpanded}
                  onFilesExpandedChange={setFilesExpanded}
                  onCollapseFiles={collapseFiles}
                  onClose={handleCloseDetail}
                  sheetFinalFocus={sheetFinalFocus}
                  t={t}
                />
              ) : null}
            </div>
          </div>
        ) : area === "projects" ? (
          <ProjectsPage
            state={projects}
            skills={library.managedSkills}
            catalog={catalog}
            actions={actions}
            surface={surface}
            pending={pending}
            dialogProgress={dialogProgress}
            runAction={runAction}
            applyOutcome={applyOutcome}
            notify={notify}
            notifyError={notifyError}
          />
        ) : (
          <ExplorePage
            featuredSkills={explore.featuredSkills}
            featuredLoading={explore.featuredLoading}
            query={explore.exploreFilter}
            onQueryChange={explore.handleExploreFilterChange}
            searchResults={explore.searchResults}
            searchLoading={explore.searchLoading}
            managedSkills={library.managedSkills}
            hiddenSkills={explore.hiddenSkills}
            onHideSkill={explore.handleHideSkill}
            onUnhideSkill={explore.handleUnhideSkill}
            onPreview={explore.handleOpenExploreDetail}
            onInstall={handleExploreInstall}
            onOpenInstalled={handleOpenInstalledFromExplore}
            onOpenAdd={handleOpenAdd}
            preview={explorePreview}
            placement={placement}
            onClosePreview={unfocus}
            scopeTools={addFlow.add.summary.tools}
            deploys={addFlow.add.summary.deploy}
            t={t}
          />
        )}
      </main>

      <AddSkillDialog
        add={addFlow.add}
        busy={loading}
        blockedReason={blockedReason}
        progress={dialogProgress}
        t={t}
      />

      <ImportReviewDialog
        review={addFlow.importReview}
        busy={loading}
        blockedReason={blockedReason}
        progress={dialogProgress}
        t={t}
      />

      <NotificationsModal
        open={surface.isOpen("notifications")}
        runId={reportRunId}
        notifications={notifications}
        skillNames={skillNames}
        projectLabelById={projectLabelById}
        actions={actions}
        surface={surface}
        blockedReason={blockedReason}
        skillsWithIssues={exceptions.skillsWithIssues}
        onViewIssues={handleViewIssuesFromPanel}
        onRequestClose={handleCloseNotifications}
        onClear={clearNotifications}
        copyToClipboard={copyToClipboard}
        progress={dialogProgress}
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

      <SettingsDialog
        layer={surface.layer("settings")}
        onPaneChange={handleSettingsPane}
        onClose={handleCloseSettings}
        settings={settings}
        update={update}
        deploy={{
          autoDeployEnabled: sync.autoSyncEnabled,
          onAutoDeployChange: sync.handleAutoSyncToggle,
          selected: effectiveSyncTargetIds,
          scanSelectedOnly: sync.scanSelectedToolsOnly,
          onSelectedChange: harnessSelection.select,
          onScanSelectedOnlyChange: (scanSelectedOnly) => void sync.saveToolConfig({ scanSelectedOnly }),
        }}
        toolStatus={catalog.toolStatus}
        labelOf={labelOfTool}
        blockedReason={blockedReason}
        redeploy={redeployLine}
        progress={dialogProgress}
        isTauri={isTauri}
        t={t}
      />

      <RedeployOfferDialog
        layer={surface.layer("redeployOffer")}
        labelOf={labelOfTool}
        onAccept={() => void redeploy.acceptOffer()}
        onDecline={redeploy.declineOffer}
        t={t}
      />

      <DeployHarnessAsk pending={deployHarnessAsk} labelOf={labelOfTool} t={t} />
      <MoveLibraryAsk pending={moveLibraryAsk} t={t} />

      <ShortcutsDialog
        layer={surface.layer("shortcuts")}
        onPaneChange={handleShortcutsPane}
        onClose={handleCloseShortcuts}
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

      <InvocationModeModal
        skill={library.invocationEditSkill}
        loading={loading}
        onRequestClose={library.closeInvocationEdit}
        fallbackFocus={skillDialogFallback}
        onConfirm={(mode) => {
          if (library.invocationEditSkill) void run("editInvocation", [library.invocationEditSkill], { mode });
        }}
        t={t}
      />

      <RepointModal
        request={
          library.pendingRepointSkill && library.repointSelection
            ? { skill: library.pendingRepointSkill, preselect: library.repointSelection.preselect }
            : null
        }
        loading={loading}
        previewRepoint={library.previewRepoint}
        onRequestClose={library.handleCloseRepoint}
        onPickFolder={library.pickRepointFolder}
        fallbackFocus={skillDialogFallback}
        progress={dialogProgress}
        onConfirm={async (target) =>
          library.pendingRepointSkill ? run("repoint", [library.pendingRepointSkill], { target }) : null
        }
        t={t}
      />

      <DetachModal
        skill={library.pendingDetachSkill}
        loading={loading}
        onRequestClose={library.handleCloseDetach}
        fallbackFocus={skillDialogFallback}
        onConfirm={() => {
          if (library.pendingDetachSkill) void run("detach", [library.pendingDetachSkill], { confirmed: true });
        }}
        t={t}
      />

      <AssignProjectModal
        skills={library.assignProjectSkills}
        loading={loading}
        placements={assignPlacements}
        loadPlacementViews={loadPlacementViews}
        labelOf={labelOfTool}
        onRequestClose={library.handleCloseAssignProject}
        onConfirm={(projectId) => {
          const skills = library.assignProjectSkills;
          if (skills.length === 0) return;
          void (async () => {
            await run("assignToProject", skills, { projectId });
            dismissDialog("assignProject");
          })();
        }}
        t={t}
      />

      <CountConfirmModal
        request={library.countConfirm}
        pending={loading}
        blockedReason={blockedReason}
        assignmentsOf={assignmentsOf}
        loadPlacementViews={loadPlacementViews}
        toolChoices={catalog.installedTools}
        defaultTools={effectiveSyncTargetIds}
        onRequestClose={library.handleCloseCountConfirm}
        onConfirm={(tools) => {
          const request = library.countConfirm;
          if (!request) return;
          if (request.verb === "syncToTargets")
            void run("syncToTargets", request.skills, { tools: tools ?? [], confirmed: true });
          else void run(request.verb, request.skills, { confirmed: true });
        }}
        t={t}
      />

      <DeleteModal
        request={library.pendingDelete}
        pending={loading}
        progress={actionMessage}
        assignmentsOf={assignmentsOf}
        loadPlacementViews={loadPlacementViews}
        labelOf={labelOfTool}
        toolLabelById={toolLabelById}
        projectLabelById={projectLabelById}
        onRequestClose={library.handleCloseDelete}
        onConfirm={(skillIds) => void run("delete", skillIds, { confirmed: true })}
        onRetry={(skillIds) => void run("delete", skillIds, { confirmed: true })}
        onReveal={handleReveal}
        fallbackFocus={actions.dialogFallbackFocus}
        t={t}
      />

      <UpdateDialog open={surface.isOpen("update")} update={update} t={t} />

      {/* ⌘K (ticket 23): skills, places and the door's verbs on the showing surface's selection. */}
      <CommandPalette
        surface={surface}
        skills={library.managedSkills}
        actions={actions}
        blockedReason={blockedReason}
        onOpenArea={handleViewChange}
        onAddSkill={handleOpenAdd}
        onConfigureHarnesses={handleOpenHarnesses}
        t={t}
      />
    </div>
  );
}

export default App;
