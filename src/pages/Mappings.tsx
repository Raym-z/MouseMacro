import { useState } from "react";
import {
  ArrowRight,
  ChevronRight,
  MousePointerClick,
  Pencil,
  Plus,
  Trash2,
} from "lucide-react";
import { useApp } from "../state/AppContext";
import { PageHeader } from "../components/PageHeader";
import { Button } from "../components/Button";
import { Switch } from "../components/Switch";
import { Select } from "../components/Select";
import { Badge } from "../components/Badge";
import { Callout } from "../components/Callout";
import { EmptyState } from "../components/EmptyState";
import { ConfirmDialog } from "../components/Modal";
import { ActionDisplay } from "../components/ActionDisplay";
import { MouseButtonBadge } from "../components/MouseButtonBadge";
import { MappingSheet } from "../components/MappingEditor";
import { api } from "../lib/api";
import type { Mapping, MouseButton } from "../lib/types";
import { MOUSE_BUTTONS } from "../lib/types";
import {
  buttonLabel,
  sortMappings,
  triggerDescription,
} from "../lib/format";
import { cn } from "../lib/cn";

export function MappingsPage() {
  const { snapshot, runSnapshot, setView } = useApp();
  const [editorOpen, setEditorOpen] = useState(false);
  const [editing, setEditing] = useState<Mapping | null>(null);
  const [initialButton, setInitialButton] = useState<MouseButton>("x2");
  const [deleteTarget, setDeleteTarget] = useState<Mapping | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [busyId, setBusyId] = useState<string | null>(null);

  if (!snapshot) return null;
  const { config } = snapshot;
  const profile =
    config.profiles.find((item) => item.id === config.activeProfileId) ??
    config.profiles[0];
  const mappings = profile?.mappings ?? [];
  const sorted = sortMappings(mappings);
  const enabledCount = mappings.filter((mapping) => mapping.enabled).length;
  const gestureCount = mappings.filter(
    (mapping) =>
      mapping.trigger.type === "withWheel" ||
      mapping.trigger.type === "withButton",
  ).length;

  const openNew = (button?: MouseButton) => {
    setInitialButton(button ?? "x2");
    setEditing(null);
    setEditorOpen(true);
  };

  const openEdit = (mapping: Mapping) => {
    setEditing(mapping);
    setEditorOpen(true);
  };

  const switchProfile = async (profileId: string) => {
    await runSnapshot(
      () => api.setActiveProfile(profileId),
      "Profile switched",
    );
  };

  const toggleMapping = async (mapping: Mapping, enabled: boolean) => {
    setBusyId(mapping.id);
    await runSnapshot(
      () => api.setMappingEnabled(profile.id, mapping.id, enabled),
      enabled ? "Mapping enabled" : "Mapping disabled",
    );
    setBusyId(null);
  };

  const deleteMapping = async () => {
    if (!deleteTarget) return;
    setDeleting(true);
    const result = await runSnapshot(
      () => api.deleteMapping(profile.id, deleteTarget.id),
      "Mapping deleted",
    );
    setDeleting(false);
    if (result) setDeleteTarget(null);
  };

  const enableEngine = async () => {
    await runSnapshot(() => api.setEnabled(true), "Mouse mappings enabled");
  };

  return (
    <div className="animate-in">
      <PageHeader
        eyebrow="Rules"
        title="Button mappings"
        description="Assign shortcuts, clicks and gestures to any physical mouse button. Gestures always win over taps."
        actions={
          <>
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
              variant="primary"
              icon={<Plus />}
              onClick={() => openNew()}
            >
              Add mapping
            </Button>
          </>
        }
      />

      {!config.settings.enabled && (
        <Callout
          tone="info"
          className="mb-4"
          action={
            <Button size="sm" variant="primary" onClick={enableEngine}>
              Enable mappings
            </Button>
          }
        >
          The engine is paused, so these mappings are saved but not applied.
        </Callout>
      )}

      <div className="mb-5 flex flex-wrap items-center gap-2">
        <Badge tone="neutral">{profile?.name}</Badge>
        <Badge tone={enabledCount > 0 ? "success" : "outline"} dot>
          {enabledCount} enabled
        </Badge>
        <Badge tone="outline">{mappings.length} total</Badge>
        <Badge tone="outline">{gestureCount} gestures</Badge>
        <button
          onClick={() => setView("settings")}
          className="focus-ring ml-auto flex items-center gap-1 rounded text-[11.5px] font-medium text-zinc-500 transition hover:text-zinc-900 dark:text-zinc-400 dark:hover:text-zinc-100"
        >
          Priority & timing <ArrowRight className="size-3" />
        </button>
      </div>

      {mappings.length === 0 ? (
        <div className="surface rounded-2xl">
          <EmptyState
            icon={MousePointerClick}
            title="No mappings in this profile"
            description="Start with B5. Map a single click to a shortcut, then add B5 + scroll gestures for zooming."
            action={
              <Button variant="primary" icon={<Plus />} onClick={() => openNew()}>
                Add your first mapping
              </Button>
            }
          />
        </div>
      ) : (
        <div className="space-y-5">
          {MOUSE_BUTTONS.map((button) => {
            const group = sorted.filter((mapping) => mapping.button === button);
            if (group.length === 0) return null;
            return (
              <section key={button}>
                <div className="mb-2 flex items-center gap-2 px-0.5">
                  <span className="text-[12px] font-semibold">
                    {buttonLabel(button)} button
                  </span>
                  <span className="text-[11px] text-zinc-400 dark:text-zinc-500">
                    {group.length}
                  </span>
                  <button
                    onClick={() => openNew(button)}
                    className="focus-ring ml-auto flex items-center gap-1 rounded text-[11px] font-medium text-zinc-400 transition hover:text-zinc-800 dark:text-zinc-500 dark:hover:text-zinc-200"
                  >
                    <Plus className="size-3" /> Add
                  </button>
                </div>
                <div className="surface divide-y divide-zinc-100 overflow-hidden rounded-2xl dark:divide-white/[0.055]">
                  {group.map((mapping) => (
                    <div
                      key={mapping.id}
                      className={cn(
                        "group flex items-center gap-3 px-3.5 py-3 transition-colors hover:bg-zinc-50/70 dark:hover:bg-white/[0.02]",
                        !mapping.enabled && "opacity-60",
                      )}
                    >
                      <MouseButtonBadge
                        button={mapping.button}
                        muted={!mapping.enabled}
                      />
                      <button
                        className="focus-ring min-w-0 flex-1 rounded text-left"
                        onClick={() => openEdit(mapping)}
                      >
                        <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
                          <ActionDisplay action={mapping.action} />
                          {mapping.name && (
                            <span className="truncate text-[12px] text-secondary">
                              · {mapping.name}
                            </span>
                          )}
                          {!mapping.enabled && (
                            <Badge tone="outline">Disabled</Badge>
                          )}
                        </div>
                        <div className="mt-0.5 truncate text-[11.5px] text-secondary">
                          {triggerDescription(mapping)}
                        </div>
                      </button>
                      <div className="flex shrink-0 items-center gap-1">
                        <Switch
                          size="sm"
                          checked={mapping.enabled}
                          disabled={busyId === mapping.id}
                          onChange={(enabled) =>
                            toggleMapping(mapping, enabled)
                          }
                          label={`Enable mapping ${mapping.id}`}
                        />
                        <Button
                          size="icon"
                          variant="ghost"
                          onClick={() => openEdit(mapping)}
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
              </section>
            );
          })}

          <div className="flex items-center justify-between rounded-xl border border-dashed border-zinc-200 px-4 py-3 dark:border-white/[0.08]">
            <div className="flex items-center gap-2 text-[11.5px] text-secondary">
              <ChevronRight className="size-3.5" />
              Mappings apply immediately while the engine is running.
            </div>
            <Button size="sm" variant="ghost" icon={<Plus />} onClick={() => openNew()}>
              Add mapping
            </Button>
          </div>
        </div>
      )}

      <MappingSheet
        key={`${editing?.id ?? "new"}-${initialButton}`}
        open={editorOpen}
        profileId={profile?.id ?? ""}
        mapping={editing}
        initialButton={initialButton}
        onClose={() => setEditorOpen(false)}
        onRequestDelete={(mapping) => {
          setEditorOpen(false);
          setDeleteTarget(mapping);
        }}
      />

      <ConfirmDialog
        open={deleteTarget !== null}
        onClose={() => setDeleteTarget(null)}
        onConfirm={deleteMapping}
        loading={deleting}
        title="Delete this mapping?"
        confirmLabel="Delete mapping"
        description={
          deleteTarget
            ? `${triggerDescription(deleteTarget)} → ${
                deleteTarget.name || "its action"
              }. This cannot be undone.`
            : undefined
        }
      />
    </div>
  );
}
