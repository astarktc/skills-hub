import * as React from "react";

/**
 * The Field wiring shared by `field.tsx` and the Field-aware controls in this
 * folder (Checkbox). Not a call-site API: call sites compose `<Field>`.
 */

export type FieldContextValue = {
  /** The id the label points at: the rendered control's id. */
  controlId: string;
  /** The id a control renders when it has no `id` prop of its own. */
  generatedControlId: string;
  registerControlId: (id: string | null) => void;
  descriptionId: string | null;
  errorId: string | null;
  invalid: boolean;
  disabled: boolean;
  registerDescription: (present: boolean) => void;
  registerError: (present: boolean) => void;
};

export const FieldContext = React.createContext<FieldContextValue | null>(null);

/**
 * Wires a control to its Field; explicit props always win. The id the
 * control renders is reported back, so the label follows an explicit `id`.
 */
export function useFieldControlProps<
  P extends {
    id?: string;
    disabled?: boolean;
    "aria-describedby"?: string;
    "aria-invalid"?: React.AriaAttributes["aria-invalid"];
  },
>(props: P): P {
  const field = React.useContext(FieldContext);
  const id = props.id ?? field?.generatedControlId;
  const register = field?.registerControlId;
  React.useLayoutEffect(() => {
    if (!register || id === undefined) return;
    register(id);
    return () => register(null);
  }, [id, register]);
  if (!field) return props;
  const describedBy =
    [props["aria-describedby"], field.descriptionId, field.invalid ? field.errorId : null]
      .filter(Boolean)
      .join(" ") || undefined;
  return {
    ...props,
    id,
    disabled: props.disabled ?? (field.disabled || undefined),
    "aria-describedby": describedBy,
    "aria-invalid": props["aria-invalid"] ?? (field.invalid || undefined),
  };
}
