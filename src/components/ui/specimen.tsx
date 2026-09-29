/**
 * The primitives specimen — every variant × tone × size of every primitive
 * on every surface token, so DESIGN.md's contrast table is measured on the
 * rendered page, not assumed. Dev-only: reached in fixture mode
 * (`npm run dev:fixture`) at `/?specimen`; the fixture branch in main.tsx is
 * a compile-time constant, so production never bundles this file.
 * Strings here are specimen labels, not product copy (never shipped).
 */
import { useState } from "react";
import { toast } from "sonner";
import { Badge, Chip } from "./badge";
import { Button } from "./button";
import { Checkbox } from "./checkbox";
import { ConfirmDialog } from "./confirm-dialog";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "./dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuShortcut,
  DropdownMenuTrigger,
} from "./dropdown-menu";
import {
  Field,
  FieldDescription,
  FieldError,
  FieldLabel,
  Input,
  Select,
} from "./field";
import { Popover, PopoverContent, PopoverTitle, PopoverTrigger } from "./popover";
import { Segmented, SegmentedItem } from "./segmented";
import { Toaster } from "./sonner";
import { Switch } from "./switch";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./tabs";
import { Tooltip, TooltipContent, TooltipTrigger } from "./tooltip";

const variants = ["solid", "outline", "ghost"] as const;
const tones = ["neutral", "accent", "ok", "stale", "danger"] as const;
const badgeVariants = ["soft", "outline", "solid"] as const;
const badgeTones = ["neutral", "accent", "ok", "stale", "failed"] as const;
const surfaces = ["bg-background", "bg-card", "bg-popover", "bg-raised"] as const;

function Row({ children }: { children: React.ReactNode }) {
  return <div className="flex flex-wrap items-center gap-2">{children}</div>;
}

function Surface({ surface }: { surface: (typeof surfaces)[number] }) {
  const [checked, setChecked] = useState(true);
  const [on, setOn] = useState(true);
  const [view, setView] = useState("grouped");
  const [chip, setChip] = useState(true);
  return (
    <section
      data-surface={surface}
      className={`${surface} flex flex-col gap-3 border-b border-border p-4`}
    >
      <h2 className="m-0 text-sm font-semibold text-foreground">
        {surface}{" "}
        <span className="font-normal text-muted-foreground">muted</span>{" "}
        <span className="font-normal text-faint">faint</span>{" "}
        <span className="text-ok">ok</span> <span className="text-stale">stale</span>{" "}
        <span className="text-failed">failed</span> <span className="text-ring">iris</span>
      </h2>
      {variants.map((variant) => (
        <Row key={variant}>
          {tones.map((tone) => (
            <Button key={tone} variant={variant} tone={tone} data-specimen={`button-${variant}-${tone}`}>
              {variant} {tone}
            </Button>
          ))}
          <Button variant={variant} size="sm">
            sm
          </Button>
          <Button variant={variant} size="lg">
            lg
          </Button>
          <Button variant={variant} disabled>
            disabled
          </Button>
          <Button variant={variant} pending>
            pending
          </Button>
        </Row>
      ))}
      {badgeVariants.map((variant) => (
        <Row key={variant}>
          {badgeTones.map((tone) => (
            <Badge key={tone} variant={variant} tone={tone}>
              {variant} {tone}
            </Badge>
          ))}
          <Badge variant={variant} size="md">
            md
          </Badge>
        </Row>
      ))}
      <Row>
        <Chip pressed={chip} onPressedChange={setChip}>
          Issues
        </Chip>
        <Chip>Edited</Chip>
        <label className="flex items-center gap-2 text-sm text-foreground">
          <Checkbox checked={checked} onCheckedChange={setChecked} /> checked
        </label>
        <label className="flex items-center gap-2 text-sm text-foreground">
          <Checkbox checked={false} /> unchecked
        </label>
        <label className="flex items-center gap-2 text-sm text-foreground">
          <Checkbox indeterminate /> mixed
        </label>
        <label className="flex items-center gap-2 text-sm text-foreground">
          <Switch checked={on} onCheckedChange={setOn} /> on
        </label>
        <label className="flex items-center gap-2 text-sm text-foreground">
          <Switch size="sm" checked={false} /> off
        </label>
        <Segmented aria-label="View" value={view} onValueChange={setView}>
          <SegmentedItem value="grouped">Grouped</SegmentedItem>
          <SegmentedItem value="flat">Flat</SegmentedItem>
        </Segmented>
      </Row>
      <Tabs defaultValue="a">
        <TabsList variant="line">
          <TabsTrigger value="a">Selected</TabsTrigger>
          <TabsTrigger value="b">Other</TabsTrigger>
        </TabsList>
      </Tabs>
      <Tabs defaultValue="a">
        <TabsList variant="segmented">
          <TabsTrigger value="a">Selected</TabsTrigger>
          <TabsTrigger value="b">Other</TabsTrigger>
        </TabsList>
        <TabsContent value="a" className="sr-only">
          panel
        </TabsContent>
      </Tabs>
      <div className="grid max-w-3xl grid-cols-3 gap-3">
        <Field>
          <FieldLabel>Label</FieldLabel>
          <Input placeholder="Placeholder text" />
          <FieldDescription>Helper text</FieldDescription>
        </Field>
        <Field invalid>
          <FieldLabel>Invalid</FieldLabel>
          <Input defaultValue="not a url" />
          <FieldError>Error text</FieldError>
        </Field>
        <Field>
          <FieldLabel>Select</FieldLabel>
          <Select defaultValue="a">
            <option value="a">Option</option>
          </Select>
        </Field>
      </div>
    </section>
  );
}

export default function Specimen() {
  const [dialog, setDialog] = useState(false);
  const [confirm, setConfirm] = useState(false);
  return (
    <div className="h-screen overflow-y-auto bg-background text-foreground">
      <Toaster position="top-right" richColors />
      <div className="flex flex-wrap items-center gap-2 p-4">
        <Button variant="solid" tone="accent" onClick={() => setDialog(true)}>
          Open dialog
        </Button>
        <Button onClick={() => setConfirm(true)}>Open confirm</Button>
        <Popover>
          <PopoverTrigger render={<Button />}>Popover</PopoverTrigger>
          <PopoverContent>
            <PopoverTitle>Popover title</PopoverTitle>
            <p className="m-0 text-sm text-muted-foreground">Muted body text</p>
          </PopoverContent>
        </Popover>
        <DropdownMenu>
          <DropdownMenuTrigger render={<Button />}>Menu</DropdownMenuTrigger>
          <DropdownMenuContent>
            <DropdownMenuGroup>
              <DropdownMenuLabel>Label</DropdownMenuLabel>
              <DropdownMenuItem>
                Edit <DropdownMenuShortcut>E</DropdownMenuShortcut>
              </DropdownMenuItem>
            </DropdownMenuGroup>
            <DropdownMenuSeparator />
            <DropdownMenuItem variant="destructive">Delete…</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
        <Tooltip>
          <TooltipTrigger render={<Button />}>Tooltip</TooltipTrigger>
          <TooltipContent>Tooltip text</TooltipContent>
        </Tooltip>
        <Button onClick={() => toast.success("Success toast", { description: "Description" })}>
          toast ok
        </Button>
        <Button onClick={() => toast.error("Error toast", { description: "Description" })}>
          toast error
        </Button>
        <Button onClick={() => toast.warning("Warning toast", { description: "Description" })}>
          toast warning
        </Button>
        <Button onClick={() => toast.info("Info toast", { description: "Description" })}>
          toast info
        </Button>
        <Button onClick={() => toast("Plain toast", { description: "Description" })}>
          toast plain
        </Button>
      </div>
      {surfaces.map((surface) => (
        <Surface key={surface} surface={surface} />
      ))}
      <Dialog open={dialog} onOpenChange={setDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Dialog title</DialogTitle>
          </DialogHeader>
          <DialogBody>
            <p className="m-0">Body text in muted-foreground.</p>
          </DialogBody>
          <DialogFooter>
            <Button onClick={() => setDialog(false)}>Cancel</Button>
            <Button variant="solid" tone="accent" onClick={() => setDialog(false)}>
              Save
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <ConfirmDialog
        open={confirm}
        title="Delete skill?"
        confirmLabel="Delete"
        cancelLabel="Cancel"
        tone="danger"
        onConfirm={() => setConfirm(false)}
        onCancel={() => setConfirm(false)}
      >
        The skill leaves every Tool.
      </ConfirmDialog>
    </div>
  );
}
