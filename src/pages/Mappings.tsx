import { useEffect, useState } from "react";
import {
  ArrowRight,
  ChevronRight,
  Copy,
  MousePointerClick,
  Pencil,
  Plus,
  Search,
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
  actionLabel,
  buttonLabel,
  formatAge,
  shadowedMappingIds,
  sortMappings,
  triggerDescription,
} from "../lib/format";
import { cn } from "../lib/cn";

export function MappingsPage() {
  const { snapshot, runSnapshot, setView } = useApp();
  const [editorOpen, setEditorOpen] = useState(false);
  const [editing, setEditing] = useState<Mapping | null>(null);
  const [initialButton, setInitialButton] = useState<MouseButton | null>("x2");
  const [deleteTarget, setDeleteTarget] = useState<Mapping | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [bulkBusy, setBulkBusy] = useState(false);
  const [query, setQuery] = useState("");
  const [activity, setActivity] = useState<Record<string, number>>({});

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      try {
        const list = await api.getMappingActivity();
        if (!cancelled) {
          const next: Record<string, number> = {};
          for (const entry of list) next[entry.mappingId] = entry.ageMs;
          setActivity(next);
        }
      } catch {
        // Runtime activity is optional; editing must keep working without it.
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
  const { config } = snapshot;
  const profile =
    config.profiles.find((item) => item.id === config.activeProfileId) ??
    config.profiles[0];
  const mappings = profile?.mappings ?? [];
  const sorted = sortMappings(mappings);
  const shadowed = shadowedMappingIds(mappings);
  const needle = query.trim().toLowerCase();
  const visible = needle
    ? sorted.filter((mapping) =>
        [
          buttonLabel(mapping.button),
          triggerDescription(mapping),
          actionLabel(mapping.action),
          mapping.name ?? "",
        ]
          .join(" ")
          .toLowerCase()
          .includes(needle),
      )
    : sorted;
  const enabledCount = mappings.filter((mapping) => mapping.enabled).length;
  const shadowedCount = mappings.filter((mapping) => shadowed.has(mapping.id)).length;
  const gestureCount = mappings.filter(
    (mapping) =>
      mapping.trigger.type === "withWheel" ||
      mapping.trigger.type === "withButton",
  ).length;

  const openNew = (button: MouseButton | null = "x2") => {
    setInitialButton(button);
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

  const duplicateMapping = async (mapping: Mapping) => {
    setBusyId(mapping.id);
    await runSnapshot(
      () => api.duplicateMapping(profile.id, mapping.id),
      "Mapping duplicated",
    );
    setBusyId(null);
  };

  const setAllEnabled = async (enabled: boolean) => {
    if (mappings.length === 0) return;
    setBulkBusy(true);
    await runSnapshot(
      () =>
        api.setMappingsEnabled(
          profile.id,
          mappings.map((mapping) => mapping.id),
          enabled,
        ),
      enabled ? "All mappings enabled" : "All mappings disabled",
    );
    setBulkBusy(false);
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

      <div className="mb-3 flex flex-wrap items-center gap-2">
        <Badge tone="neutral">{profile?.name}</Badge>
        <Badge tone={enabledCount > 0 ? "success" : "outline"} dot>
          {enabledCount} enabled
        </Badge>
        <Badge tone="outline">{mappings.length} total</Badge>
        <Badge tone="outline">{gestureCount} gestures</Badge>
        {shadowedCount > 0 && (
          <Badge tone="warning" dot>
            {shadowedCount} shadowed
          </Badge>
        )}
        <div className="ml-auto flex flex-wrap items-center gap-2">
          <div className="relative">
            <Search className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-zinc-400" />
            <input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search mappings"
              className="focus-ring h-8 w-[190px] rounded-lg border border-zinc-200 bg-white pl-8 pr-2 text-[11.5px] placeholder:text-zinc-400 dark:border-white/10 dark:bg-white/[0.04] dark:placeholder:text-zinc-600"
            />
          </div>
          <Button
            size="sm"
            variant="secondary"
            disabled={bulkBusy || mappings.length === 0}
            onClick={() => setAllEnabled(false)}
          >
            Disable all
          </Button>
          <Button
            size="sm"
            variant="ghost"
            disabled={bulkBusy || mappings.length === 0}
            onClick={() => setAllEnabled(true)}
          >
            Enable all
          </Button>
        </div>
      </div>
      <div className="mb-5 flex items-center">
        <button
          onClick={() => setView("settings")}
          className="focus-ring flex items-center gap-1 rounded text-[11.5px] font-medium text-zinc-500 transition hover:text-zinc-900 dark:text-zinc-400 dark:hover:text-zinc-100"
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
          {visible.length === 0 && (
            <div className="surface rounded-2xl px-6 py-8 text-center">
              <p className="text-[12.5px] font-medium">
                No mappings match “{query}”.
              </p>
              <button
                onClick={() => setQuery("")}
                className="focus-ring mt-2 rounded text-[11.5px] font-medium text-zinc-500 underline-offset-2 hover:underline dark:text-zinc-400"
              >
                Clear search
              </button>
            </div>
          )}
          {[
            ...MOUSE_BUTTONS.map((button) => ({
              key: button as string,
              label: `${buttonLabel(button)} button`,
              button: button as MouseButton | null,
              items: visible.filter((mapping) => mapping.button === button),
            })),
            {
              key: "wheel",
              label: "Scroll wheel",
              button: null as MouseButton | null,
              items: visible.filter((mapping) => mapping.button === null),
            },
          ]
            .filter((group) => group.items.length > 0)
            .map((group) => (
              <section key={group.key}>
                <div className="mb-2 flex items-center gap-2 px-0.5">
                  <span className="text-[12px] font-semibold">
                    {group.label}
                  </span>
                  <span className="text-[11px] text-zinc-400 dark:text-zinc-500">
                    {group.items.length}
                  </span>
                  <button
                    onClick={() => openNew(group.button)}
                    className="focus-ring ml-auto flex items-center gap-1 rounded text-[11px] font-medium text-zinc-400 transition hover:text-zinc-800 dark:text-zinc-500 dark:hover:text-zinc-200"
                  >
                    <Plus className="size-3" /> Add
                  </button>
                </div>
                <div className="surface divide-y divide-zinc-100 overflow-hidden rounded-2xl dark:divide-white/[0.055]">
                  {group.items.map((mapping) => (
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
                          {shadowed.has(mapping.id) && (
                            <span title="An earlier enabled mapping uses the same button and trigger, so this one can never fire.">
                              <Badge tone="warning">Shadowed</Badge>
                            </span>
                          )}
                          {activity[mapping.id] !== undefined && (
                            <span className="text-[10.5px] text-zinc-400 dark:text-zinc-500">
                              fired {formatAge(activity[mapping.id])}
                            </span>
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
                          onClick={() => duplicateMapping(mapping)}
                          disabled={busyId === mapping.id}
                          title="Duplicate mapping"
                          aria-label="Duplicate mapping"
                        >
                          <Copy className="size-3.5" />
                        </Button>
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
            ))}

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
