import { useState } from "react";
import { Layers3, Pencil, Plus, Trash2 } from "lucide-react";
import { useApp } from "../state/AppContext";
import { PageHeader } from "../components/PageHeader";
import { Button } from "../components/Button";
import { Switch } from "../components/Switch";
import { Badge } from "../components/Badge";
import { Callout } from "../components/Callout";
import { EmptyState } from "../components/EmptyState";
import { Field, Segmented } from "../components/Field";
import { ConfirmDialog, Modal } from "../components/Modal";
import { ActionDisplay } from "../components/ActionDisplay";
import { MouseButtonBadge } from "../components/MouseButtonBadge";
import { MappingSheet } from "../components/MappingEditor";
import { api } from "../lib/api";
import {
  MOUSE_BUTTONS,
  newLayer,
  type Layer,
  type Mapping,
  type MouseButton,
} from "../lib/types";
import {
  buttonLabel,
  sortMappings,
  triggerDescription,
} from "../lib/format";

export function LayersPage() {
  const { snapshot, runSnapshot } = useApp();
  const [editorOpen, setEditorOpen] = useState(false);
  const [editingLayer, setEditingLayer] = useState<Layer | null>(null);
  const [draft, setDraft] = useState<Layer>(newLayer("x2"));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [deleteLayer, setDeleteLayer] = useState<Layer | null>(null);
  const [deletingLayer, setDeletingLayer] = useState(false);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [busyMappingId, setBusyMappingId] = useState<string | null>(null);
  const [mappingOpen, setMappingOpen] = useState(false);
  const [mappingLayerId, setMappingLayerId] = useState<string | null>(null);
  const [editingMapping, setEditingMapping] = useState<Mapping | null>(null);
  const [deleteMappingTarget, setDeleteMappingTarget] = useState<{
    layerId: string;
    mapping: Mapping;
  } | null>(null);
  const [deletingMapping, setDeletingMapping] = useState(false);

  if (!snapshot) return null;
  const { config } = snapshot;
  const profile =
    config.profiles.find((item) => item.id === config.activeProfileId) ??
    config.profiles[0];
  const layers = profile?.layers ?? [];

  const openNewLayer = () => {
    // Prefer side buttons: a layer on Left would swallow normal clicking.
    const candidates: MouseButton[] = ["x1", "x2", "middle", "left", "right"];
    const freeButton =
      candidates.find(
        (button) => !layers.some((layer) => layer.activator === button),
      ) ?? "x2";
    const layer = newLayer(freeButton);
    layer.name = `${buttonLabel(freeButton)} layer`;
    setDraft(layer);
    setEditingLayer(null);
    setError(null);
    setEditorOpen(true);
  };

  const openEditLayer = (layer: Layer) => {
    setDraft({ ...layer, mappings: layer.mappings.map((m) => ({ ...m })) });
    setEditingLayer(layer);
    setError(null);
    setEditorOpen(true);
  };

  const saveLayer = async () => {
    if (!draft.name.trim()) {
      setError("Give the layer a name.");
      return;
    }
    setSaving(true);
    setError(null);
    const result = await runSnapshot(
      () => api.saveLayer(profile.id, draft),
      editingLayer ? "Layer updated" : "Layer added",
    );
    setSaving(false);
    if (result) setEditorOpen(false);
  };

  const removeLayer = async () => {
    if (!deleteLayer) return;
    setDeletingLayer(true);
    const result = await runSnapshot(
      () => api.deleteLayer(profile.id, deleteLayer.id),
      "Layer deleted",
    );
    setDeletingLayer(false);
    if (result) setDeleteLayer(null);
  };

  const toggleLayer = async (layer: Layer, enabled: boolean) => {
    setBusyId(layer.id);
    await runSnapshot(
      () => api.setLayerEnabled(profile.id, layer.id, enabled),
      enabled ? "Layer enabled" : "Layer disabled",
    );
    setBusyId(null);
  };

  const toggleMapping = async (
    layerId: string,
    mapping: Mapping,
    enabled: boolean,
  ) => {
    setBusyMappingId(mapping.id);
    await runSnapshot(
      () =>
        api.setLayerMappingEnabled(
          profile.id,
          layerId,
          mapping.id,
          enabled,
        ),
      enabled ? "Mapping enabled" : "Mapping disabled",
    );
    setBusyMappingId(null);
  };

  const openNewMapping = (layer: Layer) => {
    setMappingLayerId(layer.id);
    setEditingMapping(null);
    setMappingOpen(true);
  };

  const openEditMapping = (layerId: string, mapping: Mapping) => {
    setMappingLayerId(layerId);
    setEditingMapping(mapping);
    setMappingOpen(true);
  };

  const removeMapping = async () => {
    if (!deleteMappingTarget) return;
    setDeletingMapping(true);
    const result = await runSnapshot(
      () =>
        api.deleteLayerMapping(
          profile.id,
          deleteMappingTarget.layerId,
          deleteMappingTarget.mapping.id,
        ),
      "Mapping deleted",
    );
    setDeletingMapping(false);
    if (result) setDeleteMappingTarget(null);
  };

  return (
    <div className="animate-in">
      <PageHeader
        eyebrow="Held buttons"
        title="Layers"
        description="Hold a button to switch what the other buttons do. Release it and the profile returns, with no extra taps or stuck keys."
        actions={
          <>
            <Badge tone="neutral">{profile?.name}</Badge>
            <Button variant="primary" icon={<Plus />} onClick={openNewLayer}>
              Add layer
            </Button>
          </>
        }
      />

      {layers.length === 0 ? (
        <div className="surface rounded-2xl">
          <EmptyState
            icon={Layers3}
            title="No layers in this profile"
            description="A layer is a second mapping set that activates only while one button is held. For example: hold B5 and click Left to go Back, without changing what a B5 tap does."
            action={
              <Button variant="primary" icon={<Plus />} onClick={openNewLayer}>
                Add your first layer
              </Button>
            }
          />
        </div>
      ) : (
        <div className="space-y-4">
          {layers.map((layer) => {
            const mappings = sortMappings(layer.mappings);
            return (
              <section
                key={layer.id}
                className="surface overflow-hidden rounded-2xl"
              >
                <div className="flex items-center gap-3 border-b border-zinc-100 px-4 py-3.5 dark:border-white/[0.055]">
                  <MouseButtonBadge
                    button={layer.activator}
                    muted={!layer.enabled}
                  />
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <h3 className="truncate text-[13.5px] font-semibold">
                        {layer.name}
                      </h3>
                      <Badge tone={layer.enabled ? "accent" : "outline"}>
                        hold {buttonLabel(layer.activator)}
                      </Badge>
                      {!layer.enabled && <Badge tone="outline">Disabled</Badge>}
                      <span className="text-[11px] text-zinc-400 dark:text-zinc-500">
                        {layer.mappings.length}{" "}
                        {layer.mappings.length === 1 ? "mapping" : "mappings"}
                      </span>
                    </div>
                    <p className="mt-0.5 text-[11.5px] text-secondary">
                      While {buttonLabel(layer.activator)} is held, unlisted
                      buttons keep their normal profile mapping.
                    </p>
                  </div>
                  <div className="flex shrink-0 items-center gap-1">
                    <Switch
                      size="sm"
                      checked={layer.enabled}
                      disabled={busyId === layer.id}
                      onChange={(enabled) => toggleLayer(layer, enabled)}
                      label={`Enable layer ${layer.name}`}
                    />
                    <Button
                      size="icon"
                      variant="ghost"
                      onClick={() => openEditLayer(layer)}
                      title="Edit layer"
                      aria-label="Edit layer"
                    >
                      <Pencil className="size-3.5" />
                    </Button>
                    <Button
                      size="icon"
                      variant="ghost"
                      className="hover:bg-rose-50 hover:text-rose-600 dark:hover:bg-rose-500/10 dark:hover:text-rose-400"
                      onClick={() => setDeleteLayer(layer)}
                      title="Delete layer"
                      aria-label="Delete layer"
                    >
                      <Trash2 className="size-3.5" />
                    </Button>
                  </div>
                </div>

                {mappings.length === 0 ? (
                  <div className="flex items-center justify-between gap-4 px-4 py-4">
                    <p className="text-[11.5px] text-secondary">
                      No mappings yet. The layer currently changes nothing.
                    </p>
                    <Button
                      size="sm"
                      variant="secondary"
                      icon={<Plus />}
                      onClick={() => openNewMapping(layer)}
                    >
                      Add mapping
                    </Button>
                  </div>
                ) : (
                  <>
                    <div className="divide-y divide-zinc-100 dark:divide-white/[0.055]">
                      {mappings.map((mapping) => (
                        <div
                          key={mapping.id}
                          className="flex items-center gap-3 px-4 py-2.5"
                        >
                          <MouseButtonBadge
                            button={mapping.button}
                            muted={!mapping.enabled}
                          />
                          <button
                            onClick={() => openEditMapping(layer.id, mapping)}
                            className="focus-ring min-w-0 flex-1 rounded text-left"
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
                              disabled={busyMappingId === mapping.id}
                              onChange={(enabled) =>
                                toggleMapping(layer.id, mapping, enabled)
                              }
                              label={`Enable mapping ${mapping.id}`}
                            />
                            <Button
                              size="icon"
                              variant="ghost"
                              onClick={() => openEditMapping(layer.id, mapping)}
                              title="Edit mapping"
                              aria-label="Edit mapping"
                            >
                              <Pencil className="size-3.5" />
                            </Button>
                            <Button
                              size="icon"
                              variant="ghost"
                              className="hover:bg-rose-50 hover:text-rose-600 dark:hover:bg-rose-500/10 dark:hover:text-rose-400"
                              onClick={() =>
                                setDeleteMappingTarget({
                                  layerId: layer.id,
                                  mapping,
                                })
                              }
                              title="Delete mapping"
                              aria-label="Delete mapping"
                            >
                              <Trash2 className="size-3.5" />
                            </Button>
                          </div>
                        </div>
                      ))}
                    </div>
                    <div className="flex justify-end border-t border-zinc-100 px-4 py-2.5 dark:border-white/[0.055]">
                      <Button
                        size="sm"
                        variant="ghost"
                        icon={<Plus />}
                        onClick={() => openNewMapping(layer)}
                      >
                        Add mapping
                      </Button>
                    </div>
                  </>
                )}
              </section>
            );
          })}
        </div>
      )}

      <Callout tone="info" className="mt-4">
        The activator's own tap, double click and long press still come from the
        profile. A layer overrides only the buttons it defines; everything else
        falls through. Use the same mapping editor for layers — gestures,
        shortcuts and held actions all work.
      </Callout>

      <Modal
        open={editorOpen}
        onClose={() => setEditorOpen(false)}
        title={editingLayer ? "Edit layer" : "New layer"}
        description="The layer is active only while its activator button is physically held."
        size="md"
        footer={
          <>
            <Button variant="ghost" onClick={() => setEditorOpen(false)}>
              Cancel
            </Button>
            <Button variant="primary" onClick={saveLayer} loading={saving}>
              {editingLayer ? "Save layer" : "Add layer"}
            </Button>
          </>
        }
      >
        <div className="space-y-4">
          <Field label="Layer name">
            <input
              value={draft.name}
              maxLength={60}
              onChange={(event) =>
                setDraft({ ...draft, name: event.target.value })
              }
              placeholder="e.g. Navigation"
              className="focus-ring h-9 w-full rounded-[9px] border border-zinc-200 bg-white px-3 text-[12.5px] placeholder:text-zinc-400 dark:border-white/10 dark:bg-white/[0.04] dark:placeholder:text-zinc-600"
            />
          </Field>

          <Field
            label="Activator button"
            hint="Hold this button to activate the layer. Each layer needs a different activator."
          >
            <Segmented
              value={draft.activator}
              onChange={(activator: MouseButton) =>
                setDraft({ ...draft, activator })
              }
              options={MOUSE_BUTTONS.map((button) => ({
                value: button,
                label: buttonLabel(button),
                title: layers.some(
                  (layer) =>
                    layer.activator === button &&
                    layer.id !== editingLayer?.id,
                )
                  ? "Already used by another layer"
                  : undefined,
              }))}
            />
          </Field>

          <div className="flex items-center justify-between rounded-xl border border-zinc-200/80 px-3 py-2.5 dark:border-white/[0.07]">
            <div>
              <div className="text-[12.5px] font-medium">Layer enabled</div>
              <div className="text-[11px] text-secondary">
                Disabled layers stay saved but never activate.
              </div>
            </div>
            <Switch
              checked={draft.enabled}
              onChange={(enabled) => setDraft({ ...draft, enabled })}
              label="Layer enabled"
            />
          </div>

          {error && <Callout tone="danger">{error}</Callout>}
        </div>
      </Modal>

      <MappingSheet
        key={`${editingMapping?.id ?? "new"}-${mappingLayerId ?? "none"}`}
        open={mappingOpen}
        profileId={profile?.id ?? ""}
        layerId={mappingLayerId ?? undefined}
        mapping={editingMapping}
        initialButton="left"
        onClose={() => setMappingOpen(false)}
        onRequestDelete={(mapping) => {
          setMappingOpen(false);
          if (mappingLayerId) {
            setDeleteMappingTarget({
              layerId: mappingLayerId,
              mapping,
            });
          }
        }}
      />

      <ConfirmDialog
        open={deleteLayer !== null}
        onClose={() => setDeleteLayer(null)}
        onConfirm={removeLayer}
        loading={deletingLayer}
        title={`Delete “${deleteLayer?.name ?? ""}”?`}
        description="Its mappings will be removed. Profile mappings are not affected."
        confirmLabel="Delete layer"
      />

      <ConfirmDialog
        open={deleteMappingTarget !== null}
        onClose={() => setDeleteMappingTarget(null)}
        onConfirm={removeMapping}
        loading={deletingMapping}
        title="Delete this layer mapping?"
        description={
          deleteMappingTarget
            ? `${triggerDescription(deleteMappingTarget.mapping)} → ${
                deleteMappingTarget.mapping.name || "its action"
              }.`
            : undefined
        }
        confirmLabel="Delete mapping"
      />
    </div>
  );
}
