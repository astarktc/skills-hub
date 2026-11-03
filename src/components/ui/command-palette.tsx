import * as React from "react";
import { Autocomplete } from "@base-ui/react/autocomplete";
import { Search } from "lucide-react";
import { cn } from "@/lib/utils";
import { Dialog, DialogContent, type FocusFallback } from "./dialog";
import { dispatchRefusal } from "./held-refusal";
import { fieldFocusRing } from "./styles";

/**
 * CommandPalette — the ⌘K palette's shell (round-19 ticket 23): a search
 * input that filters a grouped list of commands, inside the one modal shell.
 *
 * Built on Base UI `Autocomplete` rendered `inline` inside our `Dialog`
 * (Base UI's own command-palette pattern), so it is one layer of the layer
 * stack like every dialog: focus moves to the input on open and is trapped,
 * the app behind is inert, Escape and an outside press dismiss it (top layer
 * only), and focus returns to the element that opened it — or, when that is
 * gone, to `fallbackFocus` — never to `<body>`.
 *
 * Semantics: the input is a `combobox` (`aria-autocomplete="list"`) that
 * controls a `listbox`; each command is an `option` inside a `group` named by
 * its label; DOM focus stays in the input and the highlighted option is its
 * `aria-activedescendant` (↑ / ↓ move it, Home / End stay with the text).
 * The first match is highlighted once a query is typed (never on open), so Enter runs it;
 * ↓ highlights the first option without typing.
 *
 * Filtering is the caller's (the palette's item fold): `groups` are what to
 * show for `query`; the primitive filters nothing itself.
 *
 * A **refused** command (`refusal`: something runs, or it would change
 * nothing) stays listed and reachable, `aria-disabled`, with its reason as
 * its visible hint. Activating it runs nothing and dispatches the held
 * refusal (`held-refusal.ts`), which the binder shows — as a held Button does.
 *
 * No `variant`, `tone` or `size`: there is one palette. No motion: it appears
 * and leaves instantly (DESIGN.md § Motion — it opens many times a day).
 */

export type CommandPaletteItem = {
  /** Stable id: the React key and what `onSelect` reports. */
  id: string;
  label: string;
  /** A quiet second fact on the right (a skill's source); the refusal replaces it. */
  hint?: string | null;
  /** Why the command cannot run now, or null when it can. */
  refusal?: string | null;
};

export type CommandPaletteGroup<T extends CommandPaletteItem = CommandPaletteItem> = {
  id: string;
  /** Names the group (`role="group"`) and is shown above it. */
  label: string;
  items: readonly T[];
};

type CommandPaletteProps<T extends CommandPaletteItem> = {
  open: boolean;
  /** Called with `false` when the operator dismisses the palette (Escape, outside press). */
  onOpenChange: (open: boolean) => void;
  /** After the palette has fully opened or closed; on close, focus return follows (callers that need it settled wait a frame or more). */
  onOpenChangeComplete?: (open: boolean) => void;
  /** The dialog's accessible name. */
  label: string;
  /** The input's accessible name. */
  inputLabel: string;
  placeholder: string;
  query: string;
  onQueryChange: (query: string) => void;
  /** What to show for `query`, in order; empty groups are skipped. */
  groups: readonly CommandPaletteGroup<T>[];
  /** Shown (and announced) when no group has an item. */
  emptyText: string;
  /** The foot's key hints (e.g. ↑↓ · Enter · Esc); decorative. */
  footer?: React.ReactNode;
  /** Run a command. Never called for a refused one. */
  onSelect: (item: T) => void;
  /** Where focus goes on close when the opener is gone (the Dialog's `fallbackFocus`). */
  fallbackFocus?: FocusFallback;
};

const itemClass = [
  // Row height (36 px, `h-row`: rows, headers, palette items); the highlighted
  // one carries the inset ring as well as the ground (the ground alone is
  // ~1.1:1, never the cue — DESIGN.md § DropdownMenu).
  "flex min-h-row cursor-default items-center gap-3 rounded-sm px-2 text-sm text-foreground select-none",
  "data-highlighted:bg-accent data-highlighted:text-accent-foreground",
  "data-highlighted:outline-2 data-highlighted:outline-solid data-highlighted:-outline-offset-2 data-highlighted:outline-ring",
  "aria-disabled:text-muted-foreground",
].join(" ");

function CommandPalette<T extends CommandPaletteItem>({
  open,
  onOpenChange,
  onOpenChangeComplete,
  label,
  inputLabel,
  placeholder,
  query,
  onQueryChange,
  groups,
  emptyText,
  footer,
  onSelect,
  fallbackFocus,
}: CommandPaletteProps<T>) {
  // Base UI's grouped `items` take mutable arrays.
  const shown = React.useMemo(
    () => groups.filter((group) => group.items.length > 0).map((group) => ({ ...group, items: [...group.items] })),
    [groups],
  );
  // Each item's place in the whole list, given explicitly: Base UI then
  // registers the item as it renders instead of guessing its index from the
  // DOM order, which it re-derives on every keystroke otherwise.
  const indexOf = React.useMemo(() => {
    const map = new Map<string, number>();
    for (const group of shown) for (const item of group.items) map.set(item.id, map.size);
    return map;
  }, [shown]);
  const inputRef = React.useRef<HTMLInputElement | null>(null);
  const footerId = React.useId();

  const activate = React.useCallback(
    (item: T, element: HTMLElement) => {
      if (item.refusal) {
        // One activation (a click, or Enter in the input) is one refusal.
        dispatchRefusal(element, item.refusal);
        return;
      }
      onSelect(item);
    },
    [onSelect],
  );

  return (
    <Dialog open={open} onOpenChange={onOpenChange} onOpenChangeComplete={onOpenChangeComplete}>
      <DialogContent
        placement="top"
        size="lg"
        showCloseButton={false}
        aria-label={label}
        data-command-palette=""
        initialFocus={inputRef}
        fallbackFocus={fallbackFocus}
      >
        <Autocomplete.Root
          open
          inline
          items={shown}
          filter={null}
          value={query}
          onValueChange={(next, details) => {
            // An item press would write its label into the input; the
            // palette closes on it instead.
            if (details.reason === "item-press") return;
            onQueryChange(next);
          }}
          itemToStringValue={(item: T) => item.label}
          // Nothing is highlighted on open: recents come first, so a
          // reflexive ⌘K → Enter must not repeat the last Update all. The
          // first match is highlighted once the operator types; ↓ reaches a
          // recent without typing.
          autoHighlight
          keepHighlight
        >
          <div className="flex shrink-0 items-center gap-2 border-b border-border px-3">
            <Search aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" />
            <Autocomplete.Input
              ref={inputRef}
              data-command-palette-input=""
              aria-label={inputLabel}
              aria-describedby={footer ? footerId : undefined}
              placeholder={placeholder}
              className={cn(
                "h-11 w-full min-w-0 rounded-sm bg-transparent text-sm text-foreground placeholder:text-faint",
                fieldFocusRing,
                // The input is the palette's one control and always focused:
                // its ring would only frame the popup's own edge.
                "focus-visible:outline-none",
              )}
            />
          </div>
          {/* Base UI's Empty is itself a polite `status`. */}
          <Autocomplete.Empty className="px-2 text-center text-sm text-muted-foreground not-empty:py-6">
            {emptyText}
          </Autocomplete.Empty>
          {/* The listbox is the scroller: it is focusable (`tabindex=-1`), so
              the scrolled region is the list the input drives (axe
              `scrollable-region-focusable`), never an anonymous wrapper. */}
          <Autocomplete.List className="min-h-0 flex-1 overflow-y-auto overscroll-contain p-1 outline-none empty:hidden">
            {(group: (typeof shown)[number]) => (
              <Autocomplete.Group key={group.id} items={group.items} className="not-last:mb-1">
                <Autocomplete.GroupLabel className="px-2 pt-2 pb-1 text-xs font-medium text-faint select-none">
                  {group.label}
                </Autocomplete.GroupLabel>
                <Autocomplete.Collection>
                  {(item: T) => {
                    // The label alone is the option's name; the hint or the
                    // refusal is its description, so a screen reader hears
                    // "Update all, Update all is running" as two things.
                    const labelId = `${footerId}-${item.id}-label`;
                    const detail = item.refusal ?? item.hint ?? null;
                    const detailId = detail ? `${footerId}-${item.id}-detail` : undefined;
                    return (
                      <Autocomplete.Item
                        key={item.id}
                        value={item}
                        index={indexOf.get(item.id)}
                        data-command-id={item.id}
                        aria-labelledby={labelId}
                        aria-describedby={detailId}
                        aria-disabled={item.refusal ? true : undefined}
                        onClick={(event) => activate(item, event.currentTarget)}
                        className={itemClass}
                      >
                        <span id={labelId} className="min-w-0 flex-1 truncate">
                          {item.label}
                        </span>
                        {detail ? (
                          <span id={detailId} className="max-w-[45%] shrink-0 truncate text-xs text-muted-foreground">
                            {detail}
                          </span>
                        ) : null}
                      </Autocomplete.Item>
                    );
                  }}
                </Autocomplete.Collection>
              </Autocomplete.Group>
            )}
          </Autocomplete.List>
          {footer ? (
            <div
              id={footerId}
              className="flex shrink-0 items-center gap-4 border-t border-border bg-card px-3 py-2 text-xs text-muted-foreground"
            >
              {footer}
            </div>
          ) : null}
        </Autocomplete.Root>
      </DialogContent>
    </Dialog>
  );
}

/** A key in the palette's foot (decorative: the keys are also in the shortcuts list). */
function CommandPaletteKey({ children }: { children: React.ReactNode }) {
  return (
    <kbd className="inline-flex h-5 min-w-5 items-center justify-center rounded-sm border border-border-strong px-1 font-mono text-xs text-foreground">
      {children}
    </kbd>
  );
}

export { CommandPalette, CommandPaletteKey };
