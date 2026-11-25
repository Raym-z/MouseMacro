import { useEffect, useMemo, useState } from "react";
import {
  ArrowDown,
  ArrowUp,
  ChevronRight,
  Copy,
  MousePointerClick,
  Play,
  Plus,
  ScanLine,
  Sparkles,
  Square,
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
  MEDIA_KEYS,
  MOUSE_BUTTONS,
  WINDOW_ACTIONS,
  newMacroStep,
  newMapping,
  type Action,
  type CapturePayload,
  type MacroStep,
  type MacroStepType,
  type Mapping,
  type MouseButton,
  type ScrollDirection,
  type Trigger,
} from "../lib/types";
import {
  actionLabel,
  buttonLabel,
  keyLabel,
  macroStepLabel,
  triggerLabel,
  validateMapping,
} from "../lib/format";

interface MappingSheetProps {
  open: boolean;
  profileId: string;
  /** When set, the mapping belongs to a held-button layer. */
  layerId?: string;
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
    case "macro":
      return {
        type,
        steps: [newMacroStep("tapKeys")],
        repeat: 1,
        intervalMs: 120,
        toggle: false,
      };
    case "launch":
      return { type, target: "" };
    case "typeText":
      return { type, text: "" };
    case "media":
      return { type, key: "mediaplaypause" };
    case "window":
      return { type, action: "minimize" };
    case "toggleKeys":
      return { type, keys: [] };
    case "dragLock":
      return { type, button: "left" };
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
  { value: "macro", label: "Macro (action sequence)" },
  { value: "launch", label: "Launch a program or URL" },
  { value: "typeText", label: "Type text" },
  { value: "media", label: "Media key" },
  { value: "window", label: "Window action" },
  { value: "toggleKeys", label: "Toggle keys (sticky)" },
  { value: "dragLock", label: "Drag lock" },
  { value: "disable", label: "Disable the button" },
  { value: "none", label: "No action" },
];

const MACRO_STEP_OPTIONS: Array<{ value: MacroStepType; label: string }> = [
  { value: "tapKeys", label: "Tap keys" },
  { value: "holdKeys", label: "Hold keys" },
  { value: "releaseKeys", label: "Release keys" },
  { value: "wait", label: "Wait" },
  { value: "mouseClick", label: "Mouse click" },
  { value: "mouseHold", label: "Mouse hold" },
  { value: "mouseRelease", label: "Mouse release" },
  { value: "scroll", label: "Scroll" },
  { value: "typeText", label: "Type text" },
];

export function MappingSheet({
  open,
  profileId,
  layerId,
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
  const [newStepType, setNewStepType] = useState<MacroStepType>("tapKeys");
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
          } else if (
            action.type === "shortcut" ||
            action.type === "holdKeys" ||
            action.type === "toggleKeys"
          ) {
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
      const next = layerId
        ? await api.saveLayerMapping(profileId, layerId, draft)
        : await api.saveMapping(profileId, draft);
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

  const testAction = async () => {
    const draft: Mapping = {
      id: mapping?.id ?? "test",
      name: name.trim() ? name.trim() : null,
      button,
      trigger,
      action,
      enabled: true,
    };
    const validation = validateMapping(draft);
    if (validation) {
      setError(validation);
      return;
    }
    try {
      await api.testAction(action);
      setError(null);
      toast("Test action sent", "success");
    } catch (caught) {
      const message = errorMessage(caught);
      setError(message);
      toast(message, "error");
    }
  };

  const stopMacro = async () => {
    try {
      await api.stopMacros();
      toast("Macro stopped and holds released", "success");
    } catch (caught) {
      toast(errorMessage(caught), "error");
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

          {action.type === "macro" && (
            <div className="mt-3 space-y-3">
              <div className="grid gap-3 sm:grid-cols-3">
                <Field label="Repeat" hint="1–100 times">
                  <input
                    type="number"
                    min={1}
                    max={100}
                    value={action.repeat}
                    onChange={(event) =>
                      setAction({
                        ...action,
                        repeat: Math.max(
                          1,
                          Math.min(100, Number(event.target.value) || 1),
                        ),
                      })
                    }
                    className="focus-ring h-9 w-full rounded-[9px] border border-zinc-200 bg-white px-3 text-[12.5px] dark:border-white/10 dark:bg-white/[0.04]"
                  />
                </Field>
                <Field label="Interval" hint="ms between repeats">
                  <input
                    type="number"
                    min={0}
                    max={10000}
                    value={action.intervalMs}
                    onChange={(event) =>
                      setAction({
                        ...action,
                        intervalMs: Math.max(
                          0,
                          Math.min(10000, Number(event.target.value) || 0),
                        ),
                      })
                    }
                    className="focus-ring h-9 w-full rounded-[9px] border border-zinc-200 bg-white px-3 text-[12.5px] dark:border-white/10 dark:bg-white/[0.04]"
                  />
                </Field>
                <div className="flex items-end">
                  <div className="flex h-9 w-full items-center justify-between rounded-[9px] border border-zinc-200 px-3 dark:border-white/10">
                    <span className="text-[12px]">Toggle</span>
                    <Switch
                      size="sm"
                      checked={action.toggle}
                      onChange={(toggle) => setAction({ ...action, toggle })}
                      label="Press again to stop the macro"
                    />
                  </div>
                </div>
              </div>
              {action.toggle && (
                <p className="text-[11px] leading-relaxed text-zinc-400 dark:text-zinc-500">
                  Pressing the same button again stops the macro and releases
                  everything it holds. The panic hotkey always stops it too.
                </p>
              )}

              <div>
                <div className="mb-2 flex items-center justify-between gap-2">
                  <span className="text-[12px] font-medium">
                    Steps · {action.steps.length}
                  </span>
                  <div className="flex items-center gap-1.5">
                    <select
                      value={newStepType}
                      onChange={(event) =>
                        setNewStepType(event.target.value as MacroStepType)
                      }
                      className="focus-ring h-7 rounded-lg border border-zinc-200 bg-white px-2 text-[11.5px] dark:border-white/10 dark:bg-white/[0.04]"
                    >
                      {MACRO_STEP_OPTIONS.map((option) => (
                        <option key={option.value} value={option.value}>
                          {option.label}
                        </option>
                      ))}
                    </select>
                    <Button
                      size="sm"
                      variant="secondary"
                      icon={<Plus />}
                      onClick={() =>
                        setAction({
                          ...action,
                          steps: [...action.steps, newMacroStep(newStepType)],
                        })
                      }
                    >
                      Add step
                    </Button>
                  </div>
                </div>
                <div className="space-y-2">
                  {action.steps.length === 0 ? (
                    <div className="rounded-xl border border-dashed border-zinc-200 px-3 py-5 text-center text-[11.5px] text-zinc-400 dark:border-white/10">
                      Add the first step above. Steps run in order, top to bottom.
                    </div>
                  ) : (
                    action.steps.map((step, index) => (
                      <MacroStepRow
                        key={index}
                        step={step}
                        index={index}
                        total={action.steps.length}
                        onChange={(next) =>
                          setAction({
                            ...action,
                            steps: action.steps.map((candidate, position) =>
                              position === index ? next : candidate,
                            ),
                          })
                        }
                        onMove={(direction) => {
                          const target = index + direction;
                          if (target < 0 || target >= action.steps.length) return;
                          const steps = [...action.steps];
                          [steps[index], steps[target]] = [
                            steps[target],
                            steps[index],
                          ];
                          setAction({ ...action, steps });
                        }}
                        onDuplicate={() =>
                          setAction({
                            ...action,
                            steps: [
                              ...action.steps.slice(0, index + 1),
                              { ...step },
                              ...action.steps.slice(index + 1),
                            ],
                          })
                        }
                        onRemove={() =>
                          setAction({
                            ...action,
                            steps: action.steps.filter(
                              (_, position) => position !== index,
                            ),
                          })
                        }
                      />
                    ))
                  )}
                </div>
              </div>
            </div>
          )}

          {action.type === "launch" && (
            <div className="mt-3">
              <Field
                label="Target"
                hint="A program, file, folder or URL. Windows opens it with the default handler."
              >
                <input
                  value={action.target}
                  maxLength={1000}
                  onChange={(event) =>
                    setAction({ ...action, target: event.target.value })
                  }
                  placeholder="notepad.exe  ·  https://example.com  ·  C:\folder"
                  className="focus-ring h-9 w-full rounded-[9px] border border-zinc-200 bg-white px-3 font-mono text-[12px] placeholder:text-zinc-400 dark:border-white/10 dark:bg-white/[0.04] dark:placeholder:text-zinc-600"
                />
              </Field>
              <Callout tone="warning" className="mt-3">
                Launch actions run with your user account. Profiles imported from
                elsewhere ask for confirmation before any target is saved.
              </Callout>
            </div>
          )}

          {action.type === "typeText" && (
            <div className="mt-3">
              <Field
                label="Text"
                hint="Typed as Unicode, so non-Latin characters work. Add a wait or click step first if focus may change."
              >
                <textarea
                  value={action.text}
                  maxLength={2000}
                  rows={3}
                  onChange={(event) =>
                    setAction({ ...action, text: event.target.value })
                  }
                  placeholder="Text to type into the focused window"
                  className="focus-ring w-full resize-y rounded-[9px] border border-zinc-200 bg-white px-3 py-2 text-[12.5px] placeholder:text-zinc-400 dark:border-white/10 dark:bg-white/[0.04] dark:placeholder:text-zinc-600"
                />
              </Field>
            </div>
          )}

          {action.type === "media" && (
            <div className="mt-3">
              <Field label="Media key">
                <Select
                  value={action.key}
                  onChange={(event) =>
                    setAction({
                      ...action,
                      key: event.target.value,
                    })
                  }
                >
                  {MEDIA_KEYS.map((entry) => (
                    <option key={entry.value} value={entry.value}>
                      {entry.label}
                    </option>
                  ))}
                </Select>
              </Field>
            </div>
          )}

          {action.type === "window" && (
            <div className="mt-3">
              <Field label="Window action" hint="Applies to the window in front.">
                <Segmented
                  value={action.action}
                  onChange={(windowAction) =>
                    setAction({ ...action, action: windowAction })
                  }
                  options={WINDOW_ACTIONS}
                />
              </Field>
            </div>
          )}

          {action.type === "toggleKeys" && (
            <div className="mt-3">
              <KeySelector
                mode="combo"
                keys={action.keys}
                onChange={(keys) => setAction({ ...action, keys })}
                onRecord={() => {
                  setLiveModifiers([]);
                  setCaptureTarget("keys");
                }}
                recording={captureTarget === "keys"}
              />
              <p className="mt-2 text-[11px] leading-relaxed text-zinc-400 dark:text-zinc-500">
                The first press holds the keys down, the next press releases
                them. The panic hotkey releases everything.
              </p>
            </div>
          )}

          {action.type === "dragLock" && (
            <div className="mt-3">
              <Field label="Button to hold">
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
              <p className="mt-2 text-[11px] leading-relaxed text-zinc-400 dark:text-zinc-500">
                Press once to keep the button held (handy for dragging), press
                again to release it.
              </p>
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
          <Button
            size="sm"
            variant="ghost"
            icon={<Play />}
            onClick={testAction}
            disabled={saving}
            title="Run this action once. Hold actions are tapped for the test."
          >
            Test
          </Button>
          {action.type === "macro" && (
            <Button
              size="sm"
              variant="ghost"
              icon={<Square />}
              onClick={stopMacro}
              title="Stop every running macro and release the keys and buttons it holds."
            >
              Stop
            </Button>
          )}
        </span>
      </div>
    </Sheet>
  );
}

function MacroStepRow({
  step,
  index,
  total,
  onChange,
  onMove,
  onDuplicate,
  onRemove,
}: {
  step: MacroStep;
  index: number;
  total: number;
  onChange: (step: MacroStep) => void;
  onMove: (direction: 1 | -1) => void;
  onDuplicate: () => void;
  onRemove: () => void;
}) {
  const isKeyStep =
    step.type === "tapKeys" ||
    step.type === "holdKeys" ||
    step.type === "releaseKeys";

  return (
    <div className="rounded-xl border border-zinc-200/80 dark:border-white/[0.08]">
      <div className="flex items-center gap-2 border-b border-zinc-100 px-3 py-1.5 dark:border-white/[0.05]">
        <span className="grid size-5 shrink-0 place-items-center rounded-full bg-zinc-100 text-[10px] font-semibold text-zinc-500 dark:bg-white/[0.06] dark:text-zinc-400">
          {index + 1}
        </span>
        <span className="min-w-0 flex-1 truncate text-[11.5px] font-medium">
          {macroStepLabel(step)}
        </span>
        <Button
          size="icon"
          variant="ghost"
          onClick={() => onMove(-1)}
          disabled={index === 0}
          title="Move step up"
          aria-label="Move step up"
        >
          <ArrowUp className="size-3" />
        </Button>
        <Button
          size="icon"
          variant="ghost"
          onClick={() => onMove(1)}
          disabled={index === total - 1}
          title="Move step down"
          aria-label="Move step down"
        >
          <ArrowDown className="size-3" />
        </Button>
        <Button
          size="icon"
          variant="ghost"
          onClick={onDuplicate}
          title="Duplicate step"
          aria-label="Duplicate step"
        >
          <Copy className="size-3" />
        </Button>
        <Button
          size="icon"
          variant="ghost"
          className="hover:bg-rose-50 hover:text-rose-600 dark:hover:bg-rose-500/10 dark:hover:text-rose-400"
          onClick={onRemove}
          title="Remove step"
          aria-label="Remove step"
        >
          <Trash2 className="size-3" />
        </Button>
      </div>

      <div className="space-y-2 px-3 py-2.5">
        {isKeyStep && (
          <KeySelector
            mode="combo"
            keys={step.keys}
            onChange={(keys) => onChange({ ...step, keys })}
          />
        )}

        {step.type === "wait" && (
          <Field label="Wait" hint="milliseconds, 1–60000">
            <input
              type="number"
              min={1}
              max={60000}
              value={step.delayMs}
              onChange={(event) =>
                onChange({
                  ...step,
                  delayMs: Math.max(
                    1,
                    Math.min(60000, Number(event.target.value) || 1),
                  ),
                })
              }
              className="focus-ring h-8 w-32 rounded-[8px] border border-zinc-200 bg-white px-2.5 text-[12px] dark:border-white/10 dark:bg-white/[0.04]"
            />
          </Field>
        )}

        {(step.type === "mouseClick" ||
          step.type === "mouseHold" ||
          step.type === "mouseRelease") && (
          <Segmented
            value={step.button}
            onChange={(button) => onChange({ ...step, button })}
            options={MOUSE_BUTTONS.map((candidate) => ({
              value: candidate,
              label: buttonLabel(candidate),
            }))}
          />
        )}

        {step.type === "scroll" && (
          <div className="grid gap-2 sm:grid-cols-[1fr_110px]">
            <Segmented
              value={step.direction}
              onChange={(direction) => onChange({ ...step, direction })}
              options={(
                ["up", "down", "left", "right"] as ScrollDirection[]
              ).map((direction) => ({
                value: direction,
                label: direction.charAt(0).toUpperCase() + direction.slice(1),
              }))}
            />
            <input
              type="number"
              min={1}
              max={50}
              value={step.amount}
              onChange={(event) =>
                onChange({
                  ...step,
                  amount: Math.max(
                    1,
                    Math.min(50, Number(event.target.value) || 1),
                  ),
                })
              }
              className="focus-ring h-8 rounded-[8px] border border-zinc-200 bg-white px-2.5 text-[12px] dark:border-white/10 dark:bg-white/[0.04]"
            />
          </div>
        )}

        {step.type === "typeText" && (
          <textarea
            value={step.text}
            maxLength={2000}
            rows={2}
            onChange={(event) => onChange({ ...step, text: event.target.value })}
            placeholder="Text to type"
            className="focus-ring w-full resize-y rounded-[8px] border border-zinc-200 bg-white px-2.5 py-1.5 text-[12px] placeholder:text-zinc-400 dark:border-white/10 dark:bg-white/[0.04] dark:placeholder:text-zinc-600"
          />
        )}

        {step.type !== "wait" && (
          <div className="flex items-center gap-2">
            <span className="text-[11px] text-zinc-400 dark:text-zinc-500">
              Delay after
            </span>
            <input
              type="number"
              min={0}
              max={60000}
              value={step.delayMs}
              onChange={(event) =>
                onChange({
                  ...step,
                  delayMs: Math.max(
                    0,
                    Math.min(60000, Number(event.target.value) || 0),
                  ),
                })
              }
              className="focus-ring h-7 w-24 rounded-[7px] border border-zinc-200 bg-white px-2 text-[11.5px] dark:border-white/10 dark:bg-white/[0.04]"
            />
            <span className="text-[11px] text-zinc-400 dark:text-zinc-500">
              ms
            </span>
          </div>
        )}
      </div>
    </div>
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
  if (
    action.type === "macro" ||
    action.type === "launch" ||
    action.type === "typeText" ||
    action.type === "media" ||
    action.type === "window" ||
    action.type === "toggleKeys" ||
    action.type === "dragLock"
  ) {
    return <span>{actionLabel(action)}</span>;
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

