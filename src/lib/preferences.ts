/**
 * Every persisted view preference in the app, defined once. The literal
 * storage keys are a compatibility contract with existing installs — never
 * rename one.
 */
import {
  booleanPreference,
  stringPreference,
  unionPreference,
} from "./persistedPreference";

// Retired key: "skills-language" (the EN/ZH switcher, removed in 1.2.18 —
// EN only, BACKLOG #61). Existing installs may still hold "zh" there; nothing
// reads it any more. Never reuse the key for a different preference.

export const groupByRepoPreference = booleanPreference("skills-groupByRepo");

export const viewModePreference = unionPreference(
  "skills-viewMode",
  ["list", "auto-grid", "dense-grid"] as const,
  "list",
);

export const projectsGroupByRepoPreference = booleanPreference(
  "skills-projects-groupByRepo",
);

export const showHiddenPreference = booleanPreference("explore-showHidden");

export const themePreference = unionPreference(
  "skills-theme",
  ["system", "light", "dark"] as const,
  "system",
);

export const ignoredUpdateVersionPreference = stringPreference(
  "skills-ignored-update-version",
);
