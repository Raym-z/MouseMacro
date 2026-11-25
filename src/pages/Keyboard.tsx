import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import {
  Keyboard as KeyboardIcon,
  Pencil,
  Plus,
  ShieldAlert,
  Trash2,
} from "lucide-react";
import { useApp } from "../state/AppContext";
import { PageHeader } from "../components/PageHeader";
import { Badge } from "../components/Badge";
import { Button } from "../components/Button";
import { Callout } from "../components/Callout";
import { EmptyState } from "../components/EmptyState";
import { Field, Segmented } from "../components/Field";
import { KeySelector } from "../components/KeySelector";
import { KeyCombo } from "../components/Kbd";
import { ConfirmDialog } from "../components/Modal";
import { Select } from "../components/Select";
import { Sheet } from "../components/Sheet";
import { Switch } from "../components/Switch";
import { ActionFields } from "../components/MappingEditor";
import { useToast } from "../components/Toast";
import { useCaptureSession } from "../hooks/useCaptureSession";
import { api, errorMessage } from "../lib/api";
import { cn } from "../lib/cn";
import {
  actionLabel,
  keysLabel,
} from "../lib/format";
import {
  newKeyMapping,
  type Action,
  type CaptureControl,
  type CaptureEvent,
  type KeyMapping,
  type MacroStepType,
} from "../lib/types";

function behaviourLabel(mapping: KeyMapping): string {
  if (mapping.behaviour.mode === "remap") {
    return `Remap to ${keysLabel(mapping.behaviour.target)}`;
  }
  return actionLabel(mapping.behaviour.action);
}

export function KeyboardPage() {
  const { snapshot, runSnapshot, setView } = useApp();
  const [editorOpen, setEditorOpen] = useState(false);
  const [editing, setEditing] = useState<KeyMapping | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<KeyMapping | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [activity, setActivity] = useState<Record<string, number>>({});

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      try {
        const list = await api.getMappingActivity();
        if (cancelled) return;
        const next: Record<string, number> = {};
        for (const entry of list) next[entry.mappingId] = entry.ageMs;
        setActivity(next);
      } catch {
        // Activity is optional; the page must work without it.
      }
    };
    void load();
    const timer = window.setInterval(load, 2_000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, []);

  if (!snapshot) return null;
  const { config, status } = snapshot;
  const keyboard = config.settings.keyboard;
  const profile =
    config.profiles.find((item) => item.id === config.activeProfileId) ??
    config.profiles[0];
  const mappings = profile?.keyMappings ?? [];
  const engineOn = config.settings.enabled && !status.panicActive;

  const patch = (
    next: Partial<typeof keyboard>,
    message?: string,
  ) =>
    runSnapshot(
      () =>
        api.updateSettings({
          ...config.settings,
          keyboard: { ...keyboard, ...next },
        }),
      message,
    );

  const toggle = async (enabled: boolean) => {
    if (enabled && !keyboard.acknowledged) {
      setConfirming(true);
      return;
    }
    await patch(
      { enabled },
      enabled ? "Keyboard remapping on" : "Keyboard remapping off",
    );
  };

  const openNew = () => {
    setEditing(null);
    setEditorOpen(true);
  };

  const switchProfile = async (profileId: string) => {
    await runSnapshot(
      () => api.setActiveProfile(profileId),
      "Profile switched",
    );
  };

  const toggleMapping = async (mapping: KeyMapping, enabled: boolean) => {
    if (!profile) return;
    setBusyId(mapping.id);
    await runSnapshot(
      () => api.setKeyMappingEnabled(profile.id, mapping.id, enabled),
      enabled ? "Keyboard mapping enabled" : "Keyboard mapping disabled",
    );
    setBusyId(null);
  };

  const remove = async () => {
    if (!deleteTarget || !profile) return;
    const target = deleteTarget;
    setDeleting(true);
    const result = await runSnapshot(
      () => api.deleteKeyMapping(profile.id, target.id),
      "Keyboard mapping deleted",
    );
    setDeleting(false);
    if (result) setDeleteTarget(null);
  };

  return (
    <div className="animate-in">
      <PageHeader
        eyebrow="Input"
        title="Keyboard"
        description="Turn a key or key combination into another key, or run any MouseMacro action. Off by default: it installs an always-on keyboard hook."
        actions={
          <>
            <Badge
              tone={keyboard.enabled ? (engineOn ? "success" : "warning") : "outline"}
              dot
            >
              {keyboard.enabled
                ? engineOn
                  ? "Keyboard active"
                  : "Mappings paused"
                : "Off"}
            </Badge>
            <div className="flex items-center gap-2">
              <span className="hidden text-[11px] font-medium text-zinc-400 dark:text-zinc-500 sm:block">
                Profile
              </span>
              <Select
                value={config.activeProfileId}
                onChange={(event) => switchProfile(event.target.value)}
                className="w-[180px]"
                aria-label="Active profile"
              >
                {config.profiles.map((item) => (
                  <option key={item.id} value={item.id}>
                    {item.name}
                  </option>
                ))}
              </Select>
            </div>
            <Button
              variant="secondary"
              icon={<ShieldAlert />}
              onClick={() => setView("exceptions")}
            >
              Exceptions
            </Button>
            <Button variant="primary" icon={<Plus />} onClick={openNew}>
              Add mapping
            </Button>
          </>
        }
      />

      <section className="surface mb-4 rounded-2xl p-4">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div className="max-w-xl">
            <h2 className="text-[13px] font-semibold">Keyboard remapping</h2>
            <p className="mt-1 text-[11.5px] leading-relaxed text-secondary">
              While this is on, MouseMacro listens to every key press so it can
              match your sources. It stores no text and keeps no keystroke
              history. Unmatched keys always pass through untouched.
            </p>
          </div>
          <Switch checked={keyboard.enabled} onChange={toggle} />
        </div>

        {confirming && (
          <div className="mt-4 rounded-xl border border-amber-500/25 bg-amber-500/[0.06] p-4">
            <p className="text-[12.5px] font-medium">Before you turn this on</p>
            <ul className="mt-2 list-disc space-y-1 pl-5 text-[11.5px] leading-relaxed text-secondary">
              <li>
                A low-level keyboard hook sees every key. MouseMacro uses it only
                to match and remap the sources you define.
              </li>
              <li>
                Auto-repeat is swallowed for mapped keys, so holding one fires
                the action once.
              </li>
              <li>
                Games with anti-cheat and the secure desktop (Ctrl+Alt+Del) may
                ignore or block remapping.
              </li>
              <li>
                The emergency stop always wins and releases every held key.
              </li>
            </ul>
            <div className="mt-3 flex items-center gap-2">
              <Button
                size="sm"
                variant="primary"
                onClick={async () => {
                  setConfirming(false);
                  await patch(
                    { enabled: true, acknowledged: true },
                    "Keyboard remapping on",
                  );
                }}
              >
                I understand, turn it on
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => setConfirming(false)}
              >
                Cancel
              </Button>
            </div>
          </div>
        )}

        {keyboard.enabled && status.hookWarning && (
          <Callout tone="warning" className="mt-4">
            {status.hookWarning}
          </Callout>
        )}
      </section>

      <div className="mb-2 flex items-center gap-2 px-0.5">
        <span className="text-[12px] font-semibold">
          {profile?.name ?? "Profile"} mappings
        </span>
        <span className="text-[11px] text-zinc-400 dark:text-zinc-500">
          {mappings.length}
        </span>
        <button
          onClick={openNew}
          className="focus-ring ml-auto flex items-center gap-1 rounded text-[11px] font-medium text-zinc-400 transition hover:text-zinc-800 dark:text-zinc-500 dark:hover:text-zinc-200"
        >
          <Plus className="size-3" /> Add
        </button>
      </div>

      {mappings.length === 0 ? (
        <div className="surface overflow-hidden rounded-2xl">
          <EmptyState
            icon={KeyboardIcon}
            title="No keyboard mappings"
            description="Map Caps Lock to Ctrl, or turn F12 into a shortcut, macro or launch action."
            action={
              <Button variant="primary" icon={<Plus />} onClick={openNew}>
                Add your first mapping
              </Button>
            }
          />
        </div>
      ) : (
        <div className="surface divide-y divide-zinc-100 overflow-hidden rounded-2xl dark:divide-white/[0.055]">
          {mappings.map((mapping) => (
            <div
              key={mapping.id}
              className={cn(
                "group flex items-center gap-3 px-3.5 py-3 transition-colors hover:bg-zinc-50/70 dark:hover:bg-white/[0.02]",
                !mapping.enabled && "opacity-60",
              )}
            >
              <div className="grid min-w-24 shrink-0 place-items-start">
                <KeyCombo keys={mapping.source} max={4} />
              </div>
              <button
                className="focus-ring min-w-0 flex-1 rounded text-left"
                onClick={() => {
                  setEditing(mapping);
                  setEditorOpen(true);
                }}
              >
                <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
                  <span className="truncate text-[12.5px] font-medium">
                    {behaviourLabel(mapping)}
                  </span>
                  {mapping.name && (
                    <span className="truncate text-[12px] text-secondary">
                      · {mapping.name}
                    </span>
                  )}
                  {!mapping.enabled && <Badge tone="outline">Disabled</Badge>}
                  {activity[mapping.id] !== undefined && (
                    <span className="text-[10.5px] text-zinc-400 dark:text-zinc-500">
                      fired {Math.round(activity[mapping.id] / 1_000)}s ago
                    </span>
                  )}
                </div>
                <div className="mt-0.5 truncate text-[11.5px] text-secondary">
                  {mapping.behaviour.mode === "remap"
                    ? "Hold the source to hold the target"
                    : "Runs once on key down"}
                </div>
              </button>
              <div className="flex shrink-0 items-center gap-1">
                <Switch
                  size="sm"
                  checked={mapping.enabled}
                  disabled={busyId === mapping.id}
                  onChange={(enabled) => toggleMapping(mapping, enabled)}
                  label={`Enable keyboard mapping ${mapping.id}`}
                />
                <Button
                  size="icon"
                  variant="ghost"
                  onClick={() => {
                    setEditing(mapping);
                    setEditorOpen(true);
                  }}
                  title="Edit mapping"
                  aria-label="Edit mapping"
                >
                  <Pencil className="size-3.5" />
                </Button>
                <Button
                  size="icon"
                  variant="ghost"
                  onClick={() => setDeleteTarget(mapping)}
                  title="Delete mapping"
                  aria-label="Delete mapping"
                  className="hover:bg-rose-50 hover:text-rose-600 dark:hover:bg-rose-500/10 dark:hover:text-rose-400"
                >
                  <Trash2 className="size-3.5" />
                </Button>
              </div>
            </div>
          ))}
        </div>
      )}

      <ConfirmDialog
        open={deleteTarget !== null}
        onClose={() => setDeleteTarget(null)}
        onConfirm={remove}
        loading={deleting}
        title="Delete this keyboard mapping?"
        confirmLabel="Delete mapping"
        description={
          deleteTarget
            ? `${keysLabel(deleteTarget.source)} → ${behaviourLabel(deleteTarget)}. This cannot be undone.`
            : undefined
        }
      />

      {profile && (
        <KeyMappingSheet
          open={editorOpen}
          profileId={profile.id}
          existing={mappings}
          mapping={editing}
          onClose={() => setEditorOpen(false)}
        />
      )}
    </div>
  );
}

function KeyMappingSheet({
  open,
  profileId,
  existing,
  mapping,
  onClose,
}: {
  open: boolean;
  profileId: string;
  existing: KeyMapping[];
  mapping: KeyMapping | null;
  onClose: () => void;
}) {
  const { applySnapshot } = useApp();
  const { toast } = useToast();
  const [source, setSource] = useState<string[]>([]);
  const [mode, setMode] = useState<"remap" | "action">("remap");
  const [target, setTarget] = useState<string[]>([]);
  const [action, setAction] = useState<Action>({ type: "shortcut", keys: [] });
  const [name, setName] = useState("");
  const [enabled, setEnabled] = useState(true);
  const [captureTarget, setCaptureTarget] = useState<
    "source" | "target" | "keys" | null
  >(null);
  const [lastTarget, setLastTarget] = useState<
    "source" | "target" | "keys" | null
  >(null);
  const [newStepType, setNewStepType] = useState<MacroStepType>("tapKeys");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [appliedSeq, setAppliedSeq] = useState(0);
  const [manualPick, setManualPick] = useState(false);

  useEffect(() => {
    if (!open) {
      setCaptureTarget(null);
      return;
    }
    const draft = mapping ?? newKeyMapping();
    setSource(draft.source);
    setMode(draft.behaviour.mode);
    setTarget(
      draft.behaviour.mode === "remap" ? draft.behaviour.target : [],
    );
    setAction(
      draft.behaviour.mode === "action"
        ? draft.behaviour.action
        : { type: "shortcut", keys: [] },
    );
    setName(draft.name ?? "");
    setEnabled(draft.enabled);
    setError(null);
    setCaptureTarget(null);
    setNewStepType("tapKeys");
  }, [open, mapping]);

  const capture = useCaptureSession(captureTarget === null ? null : "keyboard", () =>
    setCaptureTarget(null),
  );

  const toggleCapture = (target: "source" | "target" | "keys") => {
    const next = captureTarget === target ? null : target;
    if (next !== null) {
      setAppliedSeq(
        captureTarget === null
          ? 0
          : capture.events.reduce((max, event) => Math.max(max, event.seq), 0),
      );
      setManualPick(false);
    }
    setLastTarget(target);
    setCaptureTarget(next);
  };

  const applyCaptured = (event: CaptureEvent) => {
    if (event.source !== "keyboard" || event.kind !== "down" || !event.token) {
      return;
    }
    const combo = [...event.modifiers, event.token];
    if (captureTarget === "source") {
      setSource(combo);
    } else if (captureTarget === "target") {
      setTarget(combo);
    } else if (captureTarget === "keys") {
      if (action.type === "key") {
        setAction({ type: "key", key: event.token });
      } else if (
        action.type === "shortcut" ||
        action.type === "holdKeys" ||
        action.type === "toggleKeys"
      ) {
        setAction({ ...action, keys: combo });
      }
    }
  };

  const pickCaptured = (event: CaptureEvent) => {
    setManualPick(true);
    setAppliedSeq(event.seq);
    applyCaptured(event);
  };

  const captureControl = (
    target: "source" | "target" | "keys",
  ): CaptureControl => ({
    recording: captureTarget === target,
    ready: capture.ready,
    events: capture.events,
    onToggle: () => toggleCapture(target),
    showLog: lastTarget === target,
    selectedSeq: appliedSeq,
    onPick: pickCaptured,
  });

  useEffect(() => {
    if (manualPick) return;
    const selected = [...capture.events]
      .reverse()
      .find((event) => event.decision === "selected" && event.kind === "down");
    if (!selected || selected.seq <= appliedSeq) return;
    setAppliedSeq(selected.seq);
    applyCaptured(selected);
  }, [capture.events, appliedSeq, manualPick, captureTarget, action]);

  useEffect(() => {
    if (!capture.error) return;
    setError(capture.error);
    setCaptureTarget(null);
  }, [capture.error]);

  const normalize = (keys: string[]) =>
    keys.map((key) => key.trim().toLowerCase()).filter(Boolean);

  const handleSave = async () => {
    const normalized = normalize(source);
    if (normalized.length === 0) {
      setError("Choose the key or combination to remap.");
      return;
    }
    const duplicate = existing.some(
      (candidate) =>
        candidate.id !== mapping?.id &&
        [...candidate.source].sort().join("+") ===
          [...normalized].sort().join("+"),
    );
    if (duplicate) {
      setError("Another keyboard mapping already uses this combination.");
      return;
    }
    if (mode === "remap" && target.length === 0) {
      setError("Choose at least one target key.");
      return;
    }
    const draft: KeyMapping = {
      id: mapping?.id ?? "",
      name: name.trim() ? name.trim() : null,
      source: normalized,
      behaviour:
        mode === "remap"
          ? { mode: "remap", target: normalize(target) }
          : { mode: "action", action },
      enabled,
    };
    setSaving(true);
    setError(null);
    try {
      const next = await api.saveKeyMapping(profileId, draft);
      applySnapshot(next);
      toast(mapping ? "Keyboard mapping updated" : "Keyboard mapping added", "success");
      onClose();
    } catch (caught) {
      const message = errorMessage(caught);
      setError(message);
      toast(message, "error");
    } finally {
      setSaving(false);
    }
  };

  return (
    <Sheet
      open={open}
      onClose={onClose}
      locked={captureTarget !== null}
      title={mapping ? "Edit keyboard mapping" : "New keyboard mapping"}
      subtitle="A key combination becomes another key, or runs an action."
      footer={
        <div className="flex items-center justify-between gap-2">
          <Button
            variant="ghost"
            onClick={onClose}
            disabled={saving}
          >
            Cancel
          </Button>
          <Button variant="primary" onClick={handleSave} disabled={saving}>
            {saving ? "Saving…" : mapping ? "Save changes" : "Add mapping"}
          </Button>
        </div>
      }
    >
      <div className="space-y-6">
        <EditorSection index="1" title="Source" hint="The combination you press.">
          <KeySelector
            mode="combo"
            keys={source}
            onChange={setSource}
            capture={captureControl("source")}
          />
          <p className="mt-2 text-[11px] leading-relaxed text-zinc-400 dark:text-zinc-500">
            One key plus optional modifiers. A bare modifier (Ctrl, Shift, Alt,
            Win) cannot be a source. Matching is exact: a plain F12 mapping does
            not fire when Ctrl is held.
          </p>
        </EditorSection>

        <EditorSection
          index="2"
          title="Behaviour"
          hint="Remap keeps hold semantics; an action fires once."
        >
          <Segmented
            value={mode}
            onChange={setMode}
            options={[
              { value: "remap", label: "Remap" },
              { value: "action", label: "Run an action" },
            ]}
          />
          {mode === "remap" ? (
            <div className="mt-3">
              <KeySelector
                mode="combo"
                keys={target}
                onChange={setTarget}
                capture={captureControl("target")}
              />
              <p className="mt-2 text-[11px] leading-relaxed text-zinc-400 dark:text-zinc-500">
                While the source is held, the target stays held. Releasing the
                source releases it. This is what makes Caps Lock → Ctrl and
                hold-to-walk work.
              </p>
            </div>
          ) : (
            <p className="mt-3 text-[11.5px] leading-relaxed text-secondary">
              The action fires once when the source goes down. Hold-style
              actions (Hold keys, Mouse hold) release when the source is
              released.
            </p>
          )}
        </EditorSection>

        {mode === "action" && (
          <ActionFields
            action={action}
            setAction={setAction}
            capture={captureControl("keys")}
            newStepType={newStepType}
            setNewStepType={setNewStepType}
          />
        )}

        <EditorSection index="4" title="Details" hint="Optional label and state.">
          <Field label="Label (optional)">
            <input
              value={name}
              maxLength={60}
              onChange={(event) => setName(event.target.value)}
              placeholder="Caps Lock to Ctrl"
              className="focus-ring h-9 w-full rounded-[9px] border border-zinc-200 bg-white px-3 text-[12.5px] placeholder:text-zinc-400 dark:border-white/10 dark:bg-white/[0.04] dark:placeholder:text-zinc-600"
            />
          </Field>
          <div className="mt-3 flex items-center justify-between rounded-[10px] border border-zinc-200 px-3 py-2.5 dark:border-white/10">
            <span className="text-[12px] font-medium">Enabled</span>
            <Switch checked={enabled} onChange={setEnabled} />
          </div>
        </EditorSection>

        {error && (
          <p className="rounded-xl border border-rose-500/25 bg-rose-500/[0.06] px-3 py-2 text-[11.5px] text-rose-600 dark:text-rose-400">
            {error}
          </p>
        )}
      </div>
    </Sheet>
  );
}

function EditorSection({
  index,
  title,
  hint,
  children,
}: {
  index: string;
  title: string;
  hint: string;
  children: ReactNode;
}) {
  return (
    <section>
      <div className="mb-2 flex items-baseline gap-2">
        <span className="grid size-5 place-items-center rounded-[6px] bg-zinc-900 text-[10px] font-semibold text-white dark:bg-white dark:text-zinc-900">
          {index}
        </span>
        <span className="text-[12.5px] font-semibold">{title}</span>
        <span className="text-[11px] text-zinc-400 dark:text-zinc-500">
          {hint}
        </span>
      </div>
      {children}
    </section>
  );
}
