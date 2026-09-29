import * as React from "react";
import { ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";
import { disabledState } from "./styles";

/**
 * Field — label, control (Input / Textarea / Select), helper text and error,
 * wired so no call site has to: the label is associated with the control
 * (`htmlFor` ↔ `id`), helper and error are the control's
 * `aria-describedby`, and `invalid` sets `aria-invalid`. A placeholder is
 * never the label.
 *
 *   <Field invalid={Boolean(error)}>
 *     <FieldLabel>Repository URL</FieldLabel>
 *     <Input value={url} onChange={…} placeholder="https://github.com/…" />
 *     <FieldDescription>…</FieldDescription>
 *     <FieldError>{error}</FieldError>
 *   </Field>
 */

type FieldContextValue = {
  controlId: string;
  descriptionId: string | null;
  errorId: string | null;
  invalid: boolean;
  disabled: boolean;
  registerDescription: (present: boolean) => void;
  registerError: (present: boolean) => void;
};

const FieldContext = React.createContext<FieldContextValue | null>(null);

type FieldProps = Omit<React.ComponentProps<"div">, "className"> & {
  invalid?: boolean;
  disabled?: boolean;
  /** Layout only. */
  className?: string;
};

function Field({
  invalid = false,
  disabled = false,
  className,
  children,
  ...props
}: FieldProps) {
  const baseId = React.useId();
  const [hasDescription, setHasDescription] = React.useState(false);
  const [hasError, setHasError] = React.useState(false);
  const value = React.useMemo<FieldContextValue>(
    () => ({
      controlId: `${baseId}-control`,
      descriptionId: hasDescription ? `${baseId}-description` : null,
      errorId: hasError ? `${baseId}-error` : null,
      invalid,
      disabled,
      registerDescription: setHasDescription,
      registerError: setHasError,
    }),
    [baseId, disabled, hasDescription, hasError, invalid],
  );
  return (
    <FieldContext.Provider value={value}>
      <div
        data-slot="field"
        data-invalid={invalid || undefined}
        data-disabled={disabled || undefined}
        className={cn("flex flex-col gap-1.5", className)}
        {...props}
      >
        {children}
      </div>
    </FieldContext.Provider>
  );
}

/** A label: text-sm, medium weight, foreground. Standalone or inside Field. */
function Label({
  className,
  ...props
}: Omit<React.ComponentProps<"label">, "className"> & { className?: string }) {
  return (
    <label
      data-slot="label"
      className={cn(
        "flex items-center gap-2 text-sm font-medium text-foreground select-none",
        className,
      )}
      {...props}
    />
  );
}

function FieldLabel(props: React.ComponentProps<typeof Label>) {
  const field = React.useContext(FieldContext);
  return <Label htmlFor={field?.controlId} {...props} />;
}

function FieldDescription({
  className,
  ...props
}: Omit<React.ComponentProps<"p">, "className"> & { className?: string }) {
  const field = React.useContext(FieldContext);
  const register = field?.registerDescription;
  React.useLayoutEffect(() => {
    register?.(true);
    return () => register?.(false);
  }, [register]);
  return (
    <p
      data-slot="field-description"
      id={field?.descriptionId ?? undefined}
      className={cn("m-0 text-xs text-muted-foreground", className)}
      {...props}
    />
  );
}

/** Renders nothing without children; announced when it appears. */
function FieldError({
  className,
  children,
  ...props
}: Omit<React.ComponentProps<"p">, "className"> & { className?: string }) {
  const field = React.useContext(FieldContext);
  const register = field?.registerError;
  const present = children !== null && children !== undefined && children !== false && children !== "";
  React.useLayoutEffect(() => {
    register?.(present);
    return () => register?.(false);
  }, [present, register]);
  if (!present) return null;
  return (
    <p
      data-slot="field-error"
      role="alert"
      id={field?.errorId ?? undefined}
      className={cn("m-0 text-xs text-destructive", className)}
      {...props}
    >
      {children}
    </p>
  );
}

/** Wires a control to its Field; explicit props always win. */
function useFieldControlProps<
  P extends {
    id?: string;
    disabled?: boolean;
    "aria-describedby"?: string;
    "aria-invalid"?: React.AriaAttributes["aria-invalid"];
  },
>(props: P): P {
  const field = React.useContext(FieldContext);
  if (!field) return props;
  const describedBy =
    [props["aria-describedby"], field.descriptionId, field.invalid ? field.errorId : null]
      .filter(Boolean)
      .join(" ") || undefined;
  return {
    ...props,
    id: props.id ?? field.controlId,
    disabled: props.disabled ?? (field.disabled || undefined),
    "aria-describedby": describedBy,
    "aria-invalid": props["aria-invalid"] ?? (field.invalid || undefined),
  };
}

const controlBase = [
  "w-full min-w-0 rounded-md border border-input bg-transparent px-2.5 text-sm text-foreground",
  "placeholder:text-faint",
  "outline-none focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-ring",
  "aria-invalid:border-destructive",
  disabledState,
];

type ControlSize = "sm" | "md";
const controlHeight: Record<ControlSize, string> = { sm: "h-6 text-xs", md: "h-8" };

function Input({
  className,
  size = "md",
  ...props
}: Omit<React.ComponentProps<"input">, "className" | "size"> & {
  size?: ControlSize;
  /** Layout only (width, flex). */
  className?: string;
}) {
  const wired = useFieldControlProps(props);
  return (
    <input
      data-slot="input"
      className={cn(controlBase, controlHeight[size], className)}
      {...wired}
    />
  );
}

function Textarea({
  className,
  ...props
}: Omit<React.ComponentProps<"textarea">, "className"> & { className?: string }) {
  const wired = useFieldControlProps(props);
  return (
    <textarea
      data-slot="textarea"
      className={cn(controlBase, "min-h-16 py-2", className)}
      {...wired}
    />
  );
}

/** A native `<select>`: platform menus, full keyboard and screen-reader support. */
function Select({
  className,
  size = "md",
  ...props
}: Omit<React.ComponentProps<"select">, "className" | "size"> & {
  size?: ControlSize;
  /** Layout only (width, flex). */
  className?: string;
}) {
  const wired = useFieldControlProps(props);
  return (
    <span data-slot="select-wrapper" className={cn("relative inline-flex w-full", className)}>
      <select
        data-slot="select"
        className={cn(controlBase, controlHeight[size], "appearance-none pr-8")}
        {...wired}
      />
      <ChevronDown
        aria-hidden="true"
        className="pointer-events-none absolute top-1/2 right-2 size-4 -translate-y-1/2 text-muted-foreground"
      />
    </span>
  );
}

/** Groups related checkboxes or radios under a legend (the group's name). */
function FieldSet({
  className,
  ...props
}: Omit<React.ComponentProps<"fieldset">, "className"> & { className?: string }) {
  return (
    <fieldset
      data-slot="field-set"
      className={cn("m-0 flex min-w-0 flex-col gap-2 border-0 p-0", className)}
      {...props}
    />
  );
}

function FieldLegend({
  className,
  ...props
}: Omit<React.ComponentProps<"legend">, "className"> & { className?: string }) {
  return (
    <legend
      data-slot="field-legend"
      className={cn("mb-1 p-0 text-sm font-medium text-foreground", className)}
      {...props}
    />
  );
}

export {
  Field,
  FieldDescription,
  FieldError,
  FieldLabel,
  FieldLegend,
  FieldSet,
  Input,
  Label,
  Select,
  Textarea,
};
