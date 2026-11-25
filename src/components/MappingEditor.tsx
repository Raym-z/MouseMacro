import { useEffect, useMemo, useState } from "react";
import {
  ChevronRight,
  MousePointerClick,
  ScanLine,
  Sparkles,
  Trash2,
} from "lucide-react";
import { Sheet } from "./Sheet";
import { Button } from "./Button";
import { Switch } from "./Switch";
import { Field, Segmented } from "./Field";
import { Select } from "./Select";
import { KeySelector } from "./KeySelector";
import { MouseButtonBadge } from "./MouseButtonBadge";
import { KeyCombo } from "./Kbd";
import { Callout } from "./Callout";
import { useCapture } from "../hooks/useCapture";
import { api, errorMessage } from "../lib/api";
import { cn } from "../lib/cn";
import { useApp } from "../state/AppContext";
import { useToast } from "./Toast";
import {
  MOUSE_BUTTONS,
  newMapping,
  type Action,
  type CapturePayload,
  type Mapping,
  type MouseButton,
  type ScrollDirection,
  type Trigger,
} from "../lib/types";
import {
  buttonLabel,
  keyLabel,
  triggerLabel,
  validateMapping,
} from "../lib/format";

interface MappingSheetProps {
  open: boolean;
  profileId: string;
  mapping: Mapping | null;
  initialButton?: MouseButton;
  onClose: () => void;
  onRequestDelete: (mapping: Mapping) => void;
}

function defaultAction(type: Action["type"]): Action {
  switch (type) {
    case "shortcut":
      return { type, keys: [] };
    case "key":
      return { type, key: "" };
    case "holdKeys":
      return { type, keys: [] };
    case "mouseClick":
      return { type, button: "left" };
    case "mouseHold":
      return { type, button: "left" };
    case "scroll":
      return { type, direction: "down", amount: 3 };
    case "disable":
    case "none":
      return { type };
  }
}

const TRIGGER_TYPES: Array<{ value: Trigger["type"]; label: string }> = [
  { value: "singleClick", label: "Single click" },
  { value: "doubleClick", label: "Double click" },
  { value: "longPress", label: "Long press" },
  { value: "release", label: "On button release" },
  { value: "withButton", label: "Button + another button" },
  { value: "withWheel", label: "Button + scroll wheel" },
];

const ACTION_TYPES: Array<{ value: Action["type"]; label: string }> = [
  { value: "shortcut", label: "Keyboard shortcut" },
  { value: "key", label: "Single keyboard key" },
  { value: "holdKeys", label: "Hold keyboard keys" },
  { value: "mouseClick", label: "Mouse click" },
  { value: "mouseHold", label: "Hold mouse button" },
  { value: "scroll", label: "Scroll" },
  { value: "disable", label: "Disable the button" },
  { value: "none", label: "No action" },
];

export function MappingSheet({
  open,
  profileId,
  mapping,
  initialButton,
  onClose,
  onRequestDelete,
}: MappingSheetProps) {
  const { applySnapshot } = useApp();
  const { toast } = useToast();
  const [button, setButton] = useState<MouseButton>("x2");
  const [trigger, setTrigger] = useState<Trigger>({ type: "singleClick" });
  const [action, setAction] = useState<Action>(defaultAction("shortcut"));
  const [name, setName] = useState("");
  const [enabled, setEnabled] = useState(true);
  const [captureTarget, setCaptureTarget] = useState<"button" | "keys" | null>(
    null,
  );
  const [liveModifiers, setLiveModifiers] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!open) {
      setCaptureTarget(null);
      return;
    }
    const source = mapping ?? newMapping(initialButton ?? "x2");
    setButton(source.button);
    setTrigger(source.trigger);
    setAction(source.action);
    setName(source.name ?? "");
    setEnabled(source.enabled);
    setError(null);
    setCaptureTarget(null);
    setLiveModifiers([]);
  }, [open, mapping, initialButton]);

  useEffect(() => {
    if (captureTarget === null) return;
    const timer = window.setTimeout(() => setCaptureTarget(null), 15_000);
    return () => window.clearTimeout(timer);
  }, [captureTarget]);

  // Mouse capture does not swallow keyboard input, so Escape can still stop it.
  useEffect(() => {
    if (captureTarget !== "button") return;
    const handler = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        setCaptureTarget(null);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [captureTarget]);

  const captureMode =
    captureTarget === "button"
      ? "mouse"
      : captureTarget === "keys"
        ? "keyboard"
        : "none";

  useCapture(
    captureMode,
    (payload: CapturePayload) => {
      if (captureTarget === "button") {
        if (payload.kind === "mouse" && payload.event === "down") {
          setButton(payload.button);
          setTrigger((current) =>
            current.type === "withButton" && current.button === payload.button
              ? { type: "singleClick" }
              : current,
          );
          setCaptureTarget(null);
          toast(`Captured ${buttonLabel(payload.button)} button`, "success");
        }
        return;
      }
      if (captureTarget === "keys") {
        if (payload.kind === "cancel") {
          setCaptureTarget(null);
          setLiveModifiers([]);
          return;
        }
        if (payload.kind === "modifiers") {
          setLiveModifiers(payload.keys);
          return;
        }
        if (payload.kind === "keys") {
          if (action.type === "key") {
            const primary = [...payload.keys]
              .reverse()
              .find((key) => !["ctrl", "shift", "alt", "win"].includes(key));
            if (primary) setAction({ type: "key", key: primary });
          } else if (action.type === "shortcut" || action.type === "holdKeys") {
            setAction({ ...action, keys: payload.keys });
          }
          setLiveModifiers([]);
          setCaptureTarget(null);
        }
      }
    },
    () => {
      if (captureTarget === "button") setCaptureTarget(null);
    },
    (message) => {
      if (captureTarget === "keys") {
        toast(
          `${message} Choose the keys manually instead — that path never needs the hook.`,
          "error",
        );
      }
    },
  );

  const needsKeys =
    action.type === "shortcut" || action.type === "holdKeys" || action.type === "key";
  const recordedKeys =
    action.type === "shortcut" || action.type === "holdKeys"
      ? action.keys
      : action.type === "key" && action.key
        ? [action.key]
        : [];

  const secondButton = useMemo(() => {
    return MOUSE_BUTTONS.find((candidate) => candidate !== button) ?? "left";
  }, [button]);

  const updateTriggerType = (type: Trigger["type"]) => {
    switch (type) {
      case "withButton":
        setTrigger({ type, button: secondButton });
        break;
      case "withWheel":
        setTrigger({ type, direction: "up" });
        break;
      default:
        setTrigger({ type });
    }
  };

  const selectButton = (candidate: MouseButton) => {
    setButton(candidate);
    if (trigger.type === "withButton" && trigger.button === candidate) {
      const other = MOUSE_BUTTONS.find((item) => item !== candidate);
      if (other) setTrigger({ type: "withButton", button: other });
    }
  };

  const handleSave = async () => {
    const draft: Mapping = {
      id: mapping?.id ?? "",
      name: name.trim() ? name.trim() : null,
      button,
      trigger,
      action,
      enabled,
    };
    const validation = validateMapping(draft);
    if (validation) {
      setError(validation);
      return;
    }
    setSaving(true);
    setError(null);
    try {
      const next = await api.saveMapping(profileId, draft);
      applySnapshot(next);
      toast(mapping ? "Mapping updated" : "Mapping added", "success");
      onClose();
    } catch (caught) {
      const message = errorMessage(caught);
      setError(message);
      toast(message, "error");
    } finally {
      setSaving(false);
    }
  };

  const showCaptureHelp = captureTarget !== null;

  return (
    <Sheet
      open={open}
      onClose={onClose}
      title={mapping ? "Edit mapping" : "New mapping"}
      subtitle="Choose the physical button, what it does, and the action it should trigger."
      footer={
        <>
          <div>
            {mapping ? (
              <Button
                variant="ghost"
                size="sm"
                icon={<Trash2 />}
                className="text-rose-600 hover:bg-rose-50 hover:text-rose-700 dark:text-rose-400 dark:hover:bg-rose-500/10"
                onClick={() => onRequestDelete(mapping)}
              >
                Delete
              </Button>
            ) : (
              <span className="text-[11px] text-zinc-400 dark:text-zinc-500">
                Changes apply instantly.
              </span>
            )}
          </div>
          <div className="flex items-center gap-2">
            <Button variant="ghost" onClick={onClose} disabled={saving}>
              Cancel
            </Button>
            <Button variant="primary" onClick={handleSave} loading={saving}>
              {mapping ? "Save changes" : "Add mapping"}
            </Button>
          </div>
        </>
      }
    >
      {showCaptureHelp && (
        <div className="mb-4 flex items-start gap-3 rounded-xl border border-indigo-500/20 bg-indigo-500/[0.06] px-3 py-2.5">
          <span className="animate-pulse-dot mt-1 size-1.5 rounded-full bg-indigo-500" />
          <div className="flex-1 text-[12px] leading-relaxed text-indigo-900 dark:text-indigo-200">
            <div className="font-semibold">
              {captureTarget === "button"
                ? "Click any mouse button"
                : "Listening for your shortcut"}
            </div>
            <div className="mt-0.5 opacity-80">
              {captureTarget === "button" ? (
                <>Press a physical mouse button. Press Esc to cancel.</>
              ) : liveModifiers.length > 0 ? (
                <>Held: {liveModifiers.map(keyLabel).join(" + ")} — now press the final key.</>
              ) : (
                <>
                  Press the keys you want to use. This uses the global hook; if
                  nothing is detected, use “Choose” in the action section instead.
                  Esc cancels.
                </>
              )}
            </div>
          </div>
          {captureTarget === "keys" && (
            <Button size="sm" variant="ghost" onClick={() => setCaptureTarget(null)}>
              Stop
            </Button>
          )}
        </div>
      )}

      <div className="space-y-6">
        <StepSection
          index="1"
          title="Physical button"
          hint="Which mouse button starts this mapping?"
        >
          <div className="flex flex-wrap items-center gap-1.5">
            {MOUSE_BUTTONS.map((candidate) => (
              <button
                key={candidate}
                type="button"
                onClick={() => selectButton(candidate)}
                className={cn(
                  "focus-ring flex h-9 items-center gap-2 rounded-[9px] border px-2.5 text-[12px] font-medium transition",
                  button === candidate
                    ? "border-zinc-900 bg-zinc-900 text-white shadow-sm dark:border-zinc-100 dark:bg-zinc-100 dark:text-zinc-900"
                    : "border-zinc-200 bg-white text-zinc-600 hover:border-zinc-300 dark:border-white/10 dark:bg-white/[0.03] dark:text-zinc-300",
                )}
              >
                <MouseButtonBadge
                  button={candidate}
                  hideLabel
                  className={cn(
                    "size-5 rounded-[5px] border-0 bg-transparent text-[8px] shadow-none dark:border-0 dark:bg-transparent",
                    button === candidate && "bg-white/15 text-white dark:bg-zinc-900/15 dark:text-zinc-900",
                  )}
                />
                {buttonLabel(candidate)}
              </button>
            ))}
            <Button
              size="sm"
              variant={captureTarget === "button" ? "subtle" : "secondary"}
              icon={<ScanLine />}
              onClick={() => setCaptureTarget("button")}
              className="ml-auto"
            >
              {captureTarget === "button" ? "Detecting…" : "Detect"}
            </Button>
          </div>
        </StepSection>

        <StepSection
          index="2"
          title="Trigger"
          hint="Gestures take priority over clicks and long presses."
        >
          <Field label="Trigger type">
            <Select
              value={trigger.type}
              onChange={(event) =>
                updateTriggerType(event.target.value as Trigger["type"])
              }
            >
              {TRIGGER_TYPES.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </Select>
          </Field>

          {trigger.type === "withButton" && (
            <div className="mt-3">
              <Field
                label="Second button"
                hint="Hold the first button, then press this one."
              >
                <div className="flex flex-wrap gap-1.5">
                  {MOUSE_BUTTONS.filter((candidate) => candidate !== button).map(
                    (candidate) => (
                      <Button
                        key={candidate}
                        size="sm"
                        variant={
                          trigger.button === candidate ? "primary" : "secondary"
                        }
                        onClick={() =>
                          setTrigger({ type: "withButton", button: candidate })
                        }
                      >
                        {buttonLabel(candidate)}
                      </Button>
                    ),
                  )}
                </div>
              </Field>
            </div>
          )}

          {trigger.type === "withWheel" && (
            <div className="mt-3">
              <Field label="Wheel direction">
                <Segmented
                  value={trigger.direction}
                  onChange={(direction) =>
                    setTrigger({ type: "withWheel", direction })
                  }
                  options={[
                    { value: "up", label: "Scroll up" },
                    { value: "down", label: "Scroll down" },
                    { value: "left", label: "Scroll left" },
                    { value: "right", label: "Scroll right" },
                  ]}
                />
              </Field>
            </div>
          )}

          {trigger.type === "longPress" && (
            <Callout tone="info" className="mt-3">
              The long-press duration is currently configured in Settings. The
              tap action is cancelled once it fires.
            </Callout>
          )}
        </StepSection>

        <StepSection
          index="3"
          title="Action"
          hint="What should happen when the trigger is detected?"
        >
          <Field label="Action type">
            <Select
              value={action.type}
              onChange={(event) =>
                setAction(defaultAction(event.target.value as Action["type"]))
              }
            >
              {ACTION_TYPES.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </Select>
          </Field>

          {needsKeys && (
            <div className="mt-3">
              <KeySelector
                mode={action.type === "key" ? "single" : "combo"}
                keys={recordedKeys}
                onChange={(keys) => {
                  if (action.type === "key") {
                    setAction({ type: "key", key: keys[0] ?? "" });
                  } else if (action.type === "shortcut" || action.type === "holdKeys") {
                    setAction({ ...action, keys });
                  }
                }}
                onRecord={() => {
                  setLiveModifiers([]);
                  setCaptureTarget("keys");
                }}
                recording={captureTarget === "keys"}
              />
              <p className="mt-2 text-[11px] leading-relaxed text-zinc-400 dark:text-zinc-500">
                {action.type === "holdKeys"
                  ? "Keys stay held until the trigger button is released. “Record” also works but needs the global keyboard hook, which some games and secure windows block — choosing keys manually always works."
                  : "Keys are pressed and released together. “Record” also works but needs the global keyboard hook, which some games and secure windows block — choosing keys manually always works."}
              </p>
            </div>
          )}

          {(action.type === "mouseClick" || action.type === "mouseHold") && (
            <div className="mt-3">
              <Field label="Mouse button">
                <Segmented
                  value={action.button}
                  onChange={(mouseButton) =>
                    setAction({ ...action, button: mouseButton })
                  }
                  options={MOUSE_BUTTONS.map((candidate) => ({
                    value: candidate,
                    label: buttonLabel(candidate),
                  }))}
                />
              </Field>
            </div>
          )}

          {action.type === "scroll" && (
            <div className="mt-3 grid gap-3 sm:grid-cols-[1fr_120px]">
              <Field label="Direction">
                <Segmented
                  value={action.direction}
                  onChange={(direction) =>
                    setAction({ ...action, direction })
                  }
                  options={(
                    ["up", "down", "left", "right"] as ScrollDirection[]
                  ).map((direction) => ({
                    value: direction,
                    label: direction.charAt(0).toUpperCase() + direction.slice(1),
                  }))}
                />
              </Field>
              <Field label="Amount" hint="1–50 notches">
                <input
                  type="number"
                  min={1}
                  max={50}
                  value={action.amount}
                  onChange={(event) =>
                    setAction({
                      ...action,
                      amount: Math.max(
                        1,
                        Math.min(50, Number(event.target.value) || 1),
                      ),
                    })
                  }
                  className="focus-ring h-9 w-full rounded-[9px] border border-zinc-200 bg-white px-3 text-[12.5px] dark:border-white/10 dark:bg-white/[0.04]"
                />
              </Field>
            </div>
          )}

          {action.type === "disable" && (
            <Callout tone="warning" className="mt-3">
              The button will be consumed and do nothing at all. Use “No action”
              for a mapping that is configured but intentionally silent.
            </Callout>
          )}
        </StepSection>

        <StepSection
          index="4"
          title="Details"
          hint="Optional label and mapping state."
        >
          <Field label="Label (optional)" hint="Shown instead of the action in lists.">
            <input
              value={name}
              maxLength={60}
              onChange={(event) => setName(event.target.value)}
              placeholder="e.g. Quick undo"
              className="focus-ring h-9 w-full rounded-[9px] border border-zinc-200 bg-white px-3 text-[12.5px] placeholder:text-zinc-400 dark:border-white/10 dark:bg-white/[0.04] dark:placeholder:text-zinc-600"
            />
          </Field>
          <div className="mt-4 flex items-center justify-between rounded-xl border border-zinc-200/80 px-3 py-2.5 dark:border-white/[0.07]">
            <div>
              <div className="text-[12.5px] font-medium">Mapping enabled</div>
              <div className="text-[11px] text-secondary">
                Disabled mappings stay saved but are ignored.
              </div>
            </div>
            <Switch checked={enabled} onChange={setEnabled} label="Mapping enabled" />
          </div>
        </StepSection>
      </div>

      {error && (
        <Callout tone="danger" className="mt-4">
          {error}
        </Callout>
      )}

      <div className="mt-5 flex items-start gap-2 text-[11px] leading-relaxed text-zinc-400 dark:text-zinc-500">
        <Sparkles className="mt-0.5 size-3 shrink-0" />
        Tip: use B4 or B5 for gestures. Hold the button, scroll or press another
        button, then release. The tap action is suppressed automatically.
      </div>

      <div className="mt-6 flex items-center justify-between rounded-xl bg-zinc-50 px-3 py-2 text-[11px] text-zinc-400 dark:bg-white/[0.025] dark:text-zinc-500">
        <span className="flex items-center gap-1.5">
          <MousePointerClick className="size-3" />
          Preview
        </span>
        <span className="flex min-w-0 items-center gap-2">
          <span className="truncate">
            {buttonLabel(button)} · {triggerLabel(trigger)}
          </span>
          <ChevronRight className="size-3 shrink-0" />
          <ActionPreview action={action} />
        </span>
      </div>
    </Sheet>
  );
}

function ActionPreview({ action }: { action: Action }) {
  if (action.type === "shortcut" || action.type === "holdKeys") {
    return action.keys.length > 0 ? (
      <KeyCombo keys={action.keys} max={3} />
    ) : (
      <span className="text-zinc-400">Not recorded</span>
    );
  }
  if (action.type === "key") {
    return <span>{action.key ? keyLabel(action.key) : "Not recorded"}</span>;
  }
  if (action.type === "mouseClick" || action.type === "mouseHold") {
    return <span>{buttonLabel(action.button)}</span>;
  }
  if (action.type === "scroll") {
    return (
      <span className="capitalize">
        Scroll {action.direction} ×{action.amount}
      </span>
    );
  }
  if (action.type === "disable") return <span>Disabled</span>;
  return <span>No action</span>;
}

function StepSection({
  index,
  title,
  hint,
  children,
}: {
  index: string;
  title: string;
  hint: string;
  children: React.ReactNode;
}) {
  return (
    <section>
      <div className="mb-2.5 flex items-start gap-2.5">
        <span className="mt-0.5 grid size-5 shrink-0 place-items-center rounded-full bg-zinc-100 text-[10px] font-semibold text-zinc-500 dark:bg-white/[0.06] dark:text-zinc-400">
          {index}
        </span>
        <div>
          <h3 className="text-[13px] font-semibold leading-5">{title}</h3>
          <p className="text-[11.5px] leading-relaxed text-secondary">{hint}</p>
        </div>
      </div>
      <div className="pl-[30px]">{children}</div>
    </section>
  );
}

