import { useState } from "react";
import {
  Check,
  Copy,
  Download,
  FolderInput,
  Layers,
  Pencil,
  Plus,
  Trash2,
  Upload,
} from "lucide-react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useApp } from "../state/AppContext";
import { useToast } from "../components/Toast";
import { PageHeader } from "../components/PageHeader";
import { Button } from "../components/Button";
import { Badge } from "../components/Badge";
import { Callout } from "../components/Callout";
import { Modal, ConfirmDialog } from "../components/Modal";
import { api, errorMessage } from "../lib/api";
import type { Profile } from "../lib/types";
import { cn } from "../lib/cn";

export function ProfilesPage() {
  const { snapshot, runSnapshot, run } = useApp();
  const { toast } = useToast();
  const [createOpen, setCreateOpen] = useState(false);
  const [newName, setNewName] = useState("");
  const [creating, setCreating] = useState(false);
  const [renameTarget, setRenameTarget] = useState<Profile | null>(null);
  const [renameName, setRenameName] = useState("");
  const [renaming, setRenaming] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<Profile | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [pendingImport, setPendingImport] = useState<{
    path: string;
    launches: string[];
  } | null>(null);
  const [importing, setImporting] = useState(false);

  if (!snapshot) return null;
  const { config } = snapshot;
  const active = config.profiles.find((p) => p.id === config.activeProfileId);

  const createProfile = async () => {
    if (!newName.trim()) {
      toast("Enter a profile name.", "error");
      return;
    }
    setCreating(true);
    const result = await runSnapshot(
      () => api.createProfile(newName.trim()),
      "Profile created",
    );
    setCreating(false);
    if (result) {
      setCreateOpen(false);
      setNewName("");
    }
  };

  const renameProfile = async () => {
    if (!renameTarget) return;
    if (!renameName.trim()) {
      toast("Enter a profile name.", "error");
      return;
    }
    setRenaming(true);
    const result = await runSnapshot(
      () => api.renameProfile(renameTarget.id, renameName.trim()),
      "Profile renamed",
    );
    setRenaming(false);
    if (result) setRenameTarget(null);
  };

  const deleteProfile = async () => {
    if (!deleteTarget) return;
    setDeleting(true);
    const result = await runSnapshot(
      () => api.deleteProfile(deleteTarget.id),
      "Profile deleted",
    );
    setDeleting(false);
    if (result) setDeleteTarget(null);
  };

  const duplicateProfile = async (profile: Profile) => {
    setBusyId(profile.id);
    await runSnapshot(
      () => api.duplicateProfile(profile.id),
      `“${profile.name}” duplicated`,
    );
    setBusyId(null);
  };

  const switchProfile = async (profile: Profile) => {
    if (profile.id === config.activeProfileId) return;
    setBusyId(profile.id);
    await runSnapshot(
      () => api.setActiveProfile(profile.id),
      `Switched to “${profile.name}”`,
    );
    setBusyId(null);
  };

  const exportProfile = async (profile: Profile) => {
    try {
      const path = await save({
        title: "Export MouseMacro profile",
        defaultPath: `${profile.name.replace(/[<>:"/\\|?*]/g, "-")}.mousemacro.json`,
        filters: [{ name: "MouseMacro profile", extensions: ["json"] }],
      });
      if (!path) return;
      await run(
        () => api.exportProfile(profile.id, path),
        `Exported “${profile.name}”`,
      );
    } catch (error) {
      toast(errorMessage(error), "error");
    }
  };

  const importProfile = async () => {
    try {
      const path = await open({
        title: "Import MouseMacro profile",
        multiple: false,
        directory: false,
        filters: [{ name: "MouseMacro profile", extensions: ["json"] }],
      });
      if (typeof path !== "string") return;
      const launches = await api.inspectProfileImport(path);
      if (launches.length > 0) {
        setPendingImport({ path, launches });
        return;
      }
      await runSnapshot(
        () => api.importProfile(path, false),
        "Profile imported and activated",
      );
    } catch (error) {
      toast(errorMessage(error), "error");
    }
  };

  const confirmImport = async () => {
    if (!pendingImport) return;
    setImporting(true);
    const result = await runSnapshot(
      () => api.importProfile(pendingImport.path, true),
      "Profile imported and activated",
    );
    setImporting(false);
    if (result) setPendingImport(null);
  };

  return (
    <div className="animate-in">
      <PageHeader
        eyebrow="Workspaces"
        title="Profiles"
        description="Switch whole mapping sets for work, play or design. The active profile is applied instantly."
        actions={
          <>
            <Button variant="secondary" icon={<Upload />} onClick={importProfile}>
              Import JSON
            </Button>
            <Button
              variant="primary"
              icon={<Plus />}
              onClick={() => {
                setNewName("");
                setCreateOpen(true);
              }}
            >
              New profile
            </Button>
          </>
        }
      />

      <div className="mb-4 flex flex-wrap items-center gap-2">
        <Badge tone="neutral" dot>
          Active: {active?.name ?? "—"}
        </Badge>
        <Badge tone="outline">{config.profiles.length} profiles</Badge>
        <span className="ml-auto text-[11.5px] text-zinc-400 dark:text-zinc-500">
          Profiles store mappings only · global timing lives in Settings
        </span>
      </div>

      <div className="grid gap-3 lg:grid-cols-2">
        {config.profiles.map((profile) => {
          const isActive = profile.id === config.activeProfileId;
          const enabled = profile.mappings.filter((m) => m.enabled).length;
          const gestures = profile.mappings.filter(
            (m) =>
              m.trigger.type === "withWheel" || m.trigger.type === "withButton",
          ).length;
          return (
            <div
              key={profile.id}
              className={cn(
                "surface relative flex flex-col rounded-2xl p-4 transition-shadow",
                isActive && "ring-1 ring-zinc-900/10 dark:ring-white/25",
                busyId === profile.id && "opacity-70",
              )}
            >
              <div className="flex items-start gap-3">
                <div
                  className={cn(
                    "grid size-9 shrink-0 place-items-center rounded-[10px] border",
                    isActive
                      ? "border-zinc-900 bg-zinc-900 text-white dark:border-white dark:bg-white dark:text-zinc-900"
                      : "border-zinc-200 bg-zinc-50 text-zinc-400 dark:border-white/[0.07] dark:bg-white/[0.03] dark:text-zinc-500",
                  )}
                >
                  <Layers className="size-4" strokeWidth={1.7} />
                </div>
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <h3 className="truncate text-[14px] font-semibold tracking-[-0.01em]">
                      {profile.name}
                    </h3>
                    {isActive && (
                      <Badge tone="success" dot>
                        Active
                      </Badge>
                    )}
                  </div>
                  <p className="mt-0.5 line-clamp-2 text-[11.5px] leading-relaxed text-secondary">
                    {profile.description || "No description"}
                  </p>
                </div>
              </div>

              <div className="mt-3.5 grid grid-cols-3 gap-2 rounded-xl bg-zinc-50/80 px-3 py-2 dark:bg-white/[0.025]">
                <div>
                  <div className="text-[14px] font-semibold">
                    {profile.mappings.length}
                  </div>
                  <div className="text-[10px] text-zinc-400 dark:text-zinc-500">
                    Mappings
                  </div>
                </div>
                <div>
                  <div className="text-[14px] font-semibold">{enabled}</div>
                  <div className="text-[10px] text-zinc-400 dark:text-zinc-500">
                    Enabled
                  </div>
                </div>
                <div>
                  <div className="text-[14px] font-semibold">{gestures}</div>
                  <div className="text-[10px] text-zinc-400 dark:text-zinc-500">
                    Gestures
                  </div>
                </div>
              </div>

              <div className="mt-3.5 flex items-center gap-1">
                {isActive ? (
                  <span className="flex h-8 items-center gap-1.5 rounded-lg bg-zinc-100 px-2.5 text-[12px] font-medium text-zinc-500 dark:bg-white/[0.06] dark:text-zinc-400">
                    <Check className="size-3.5" /> In use
                  </span>
                ) : (
                  <Button
                    size="sm"
                    variant="primary"
                    onClick={() => switchProfile(profile)}
                    disabled={busyId !== null}
                  >
                    Use profile
                  </Button>
                )}
                <div className="ml-auto flex items-center gap-0.5">
                  <IconAction
                    label="Duplicate profile"
                    onClick={() => duplicateProfile(profile)}
                    icon={<Copy className="size-3.5" />}
                  />
                  <IconAction
                    label="Export profile as JSON"
                    onClick={() => exportProfile(profile)}
                    icon={<Download className="size-3.5" />}
                  />
                  <IconAction
                    label="Rename profile"
                    onClick={() => {
                      setRenameTarget(profile);
                      setRenameName(profile.name);
                    }}
                    icon={<Pencil className="size-3.5" />}
                  />
                  <IconAction
                    label="Delete profile"
                    danger
                    disabled={config.profiles.length <= 1}
                    onClick={() => setDeleteTarget(profile)}
                    icon={<Trash2 className="size-3.5" />}
                  />
                </div>
              </div>
            </div>
          );
        })}

        <button
          onClick={() => {
            setNewName("");
            setCreateOpen(true);
          }}
          className="group flex min-h-[190px] flex-col items-center justify-center rounded-2xl border border-dashed border-zinc-200 px-6 text-center transition hover:border-zinc-300 hover:bg-white/50 dark:border-white/[0.09] dark:hover:border-white/20 dark:hover:bg-white/[0.02]"
        >
          <div className="grid size-10 place-items-center rounded-xl border border-zinc-200 bg-white text-zinc-400 transition group-hover:text-zinc-700 dark:border-white/10 dark:bg-white/[0.04] dark:text-zinc-500 dark:group-hover:text-zinc-200">
            <Plus className="size-5" />
          </div>
          <div className="mt-3 text-[13px] font-semibold">Create a profile</div>
          <p className="mt-1 max-w-[220px] text-[11.5px] leading-relaxed text-secondary">
            A clean mapping set for a different kind of work.
          </p>
        </button>
      </div>

      <Callout tone="info" className="mt-5">
        <span className="flex items-start gap-2">
          <FolderInput className="mt-0.5 size-3.5 shrink-0" />
          Import and export use readable JSON files. Imported profiles are added
          to your library and activated, never merged into existing ones.
        </span>
      </Callout>

      <Modal
        open={createOpen}
        onClose={() => setCreateOpen(false)}
        title="New profile"
        description="Give the profile a short, recognizable name."
        size="sm"
        footer={
          <>
            <Button variant="ghost" onClick={() => setCreateOpen(false)}>
              Cancel
            </Button>
            <Button variant="primary" onClick={createProfile} loading={creating}>
              Create profile
            </Button>
          </>
        }
      >
        <NameInput
          value={newName}
          onChange={setNewName}
          placeholder="e.g. Video editing"
          onSubmit={createProfile}
          autoFocus
        />
      </Modal>

      <Modal
        open={renameTarget !== null}
        onClose={() => setRenameTarget(null)}
        title="Rename profile"
        size="sm"
        footer={
          <>
            <Button variant="ghost" onClick={() => setRenameTarget(null)}>
              Cancel
            </Button>
            <Button variant="primary" onClick={renameProfile} loading={renaming}>
              Save name
            </Button>
          </>
        }
      >
        <NameInput
          value={renameName}
          onChange={setRenameName}
          placeholder="Profile name"
          onSubmit={renameProfile}
          autoFocus
        />
      </Modal>

      <Modal
        open={pendingImport !== null}
        onClose={() => setPendingImport(null)}
        title="This profile can launch programs"
        description="Imported profiles can open programs, files or URLs. Review every target before allowing the import."
        footer={
          <>
            <Button variant="ghost" onClick={() => setPendingImport(null)}>
              Cancel
            </Button>
            <Button
              variant="primary"
              onClick={confirmImport}
              loading={importing}
            >
              Import anyway
            </Button>
          </>
        }
      >
        <div className="space-y-2">
          {pendingImport?.launches.map((target, index) => (
            <div
              key={`${target}-${index}`}
              className="rounded-lg border border-zinc-200/80 bg-zinc-50 px-3 py-2 font-mono text-[11.5px] break-all dark:border-white/[0.07] dark:bg-white/[0.03]"
            >
              {target}
            </div>
          ))}
        </div>
        <Callout tone="warning" className="mt-3">
          Only import profiles from sources you trust. A launch target runs with
          your user account when its mapping fires.
        </Callout>
      </Modal>

      <ConfirmDialog
        open={deleteTarget !== null}
        onClose={() => setDeleteTarget(null)}
        onConfirm={deleteProfile}
        loading={deleting}
        title={`Delete “${deleteTarget?.name ?? ""}”?`}
        description="All mappings in this profile will be removed. This cannot be undone."
        confirmLabel="Delete profile"
      />
    </div>
  );
}

function IconAction({
  icon,
  label,
  onClick,
  danger,
  disabled,
}: {
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
  danger?: boolean;
  disabled?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      title={label}
      aria-label={label}
      disabled={disabled}
      className={cn(
        "focus-ring grid size-7 place-items-center rounded-lg text-zinc-400 transition disabled:opacity-30",
        danger
          ? "hover:bg-rose-50 hover:text-rose-600 dark:hover:bg-rose-500/10 dark:hover:text-rose-400"
          : "hover:bg-zinc-100 hover:text-zinc-700 dark:hover:bg-white/[0.07] dark:hover:text-zinc-200",
      )}
    >
      {icon}
    </button>
  );
}

function NameInput({
  value,
  onChange,
  placeholder,
  onSubmit,
  autoFocus,
}: {
  value: string;
  onChange: (value: string) => void;
  placeholder: string;
  onSubmit: () => void;
  autoFocus?: boolean;
}) {
  return (
    <input
      value={value}
      autoFocus={autoFocus}
      maxLength={60}
      placeholder={placeholder}
      onChange={(event) => onChange(event.target.value)}
      onKeyDown={(event) => {
        if (event.key === "Enter") onSubmit();
      }}
      className="focus-ring h-10 w-full rounded-[9px] border border-zinc-200 bg-white px-3 text-[13px] placeholder:text-zinc-400 dark:border-white/10 dark:bg-white/[0.04] dark:placeholder:text-zinc-600"
    />
  );
}
