import { useEffect, useMemo, useState } from "react";
import {
  Gamepad2,
  Layers,
  Monitor,
  Pencil,
  Plus,
  RefreshCw,
  Search,
  ShieldAlert,
  ShieldCheck,
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
import { Field, Segmented } from "../components/Field";
import { ConfirmDialog, Modal } from "../components/Modal";
import { api } from "../lib/api";
import type {
  ExceptionMode,
  ExceptionRule,
  InstalledApp,
  ScrollSettings,
} from "../lib/types";
import { cn } from "../lib/cn";

function emptyRule(): ExceptionRule {
  return {
    id: "",
    process: "",
    mode: "disable",
    profileId: null,
    enabled: true,
    scroll: null,
    keyboardDisabled: false,
  };
}

const MODE_LABEL: Record<ExceptionMode, string> = {
  always: "Always on",
  disable: "Mappings off",
  profile: "Use profile",
};

export function ExceptionsPage() {
  const { snapshot, runSnapshot, run, applySnapshot } = useApp();
  const [editorOpen, setEditorOpen] = useState(false);
  const [editing, setEditing] = useState<ExceptionRule | null>(null);
  const [draft, setDraft] = useState<ExceptionRule>(emptyRule());
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<ExceptionRule | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [apps, setApps] = useState<InstalledApp[]>([]);
  const [loadingApps, setLoadingApps] = useState(false);
  const [search, setSearch] = useState("");
  const [gameBusy, setGameBusy] = useState<string | null>(null);
  const [gamesOpen, setGamesOpen] = useState(true);
  const [appFilter, setAppFilter] = useState<"all" | "games" | "running">("all");

  const scanApps = async () => {
    setLoadingApps(true);
    const result = await run(() => api.scanInstalledApps());
    if (result) {
      setApps(result.apps);
      applySnapshot(result.snapshot);
    }
    setLoadingApps(false);
  };

  useEffect(() => {
    void scanApps();
    // Scan once when the page opens; the refresh button re-runs it.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!editorOpen) return;
    setSearch("");
    setAppFilter("all");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editorOpen]);

  const filteredApps = useMemo(() => {
    const query = search.trim().toLowerCase();
    return apps
      .filter((app) => {
        if (appFilter === "games" && !app.game) return false;
        if (appFilter === "running" && !app.running) return false;
        if (!query) return true;
        return (
          app.name.toLowerCase().includes(query) ||
          app.process.toLowerCase().includes(query)
        );
      })
      .slice(0, 60);
  }, [apps, search, appFilter]);

  if (!snapshot) return null;
  const { config, status } = snapshot;
  const rules = config.exceptions;
  const foreground = status.foregroundApp;
  const activeRule = rules.find(
    (rule) => foreground && rule.process === foreground && rule.enabled,
  );
  const foregroundIsGuardedGame =
    !activeRule &&
    foreground !== null &&
    config.settings.excludeGames &&
    config.knownGames.some(
      (game) => game.toLowerCase() === foreground.toLowerCase(),
    );
  const detectedGames = apps.filter((app) => app.game);
  const explicitRule = (process: string) =>
    rules.find((rule) => rule.process === process);
  const gameIsExcluded = (process: string) => {
    const rule = explicitRule(process);
    if (rule) return rule.enabled && rule.mode === "disable";
    return config.settings.excludeGames;
  };
  const excludedGames = detectedGames.filter((app) =>
    gameIsExcluded(app.process),
  ).length;
  const selectedIsGame =
    draft.process.length > 0 &&
    apps.some((app) => app.process === draft.process && app.game);

  const toggleGame = async (app: InstalledApp, excluded: boolean) => {
    const rule = explicitRule(app.process);
    setGameBusy(app.process);
    if (excluded) {
      if (!rule) {
        await runSnapshot(
          () =>
            api.saveException({
              ...emptyRule(),
              process: app.process,
              mode: "disable",
            }),
          `${app.name} excluded`,
        );
      } else if (rule.mode === "disable") {
        if (!rule.enabled) {
          await runSnapshot(
            () => api.setExceptionEnabled(rule.id, true),
            `${app.name} excluded`,
          );
        }
      } else if (config.settings.excludeGames) {
        // Default exclusion already covers this game; drop the allow rule.
        await runSnapshot(
          () => api.deleteException(rule.id),
          `${app.name} excluded`,
        );
      } else {
        await runSnapshot(
          () =>
            api.saveException({
              ...rule,
              mode: "disable",
              profileId: null,
              enabled: true,
            }),
          `${app.name} excluded`,
        );
      }
    } else if (!rule) {
      if (config.settings.excludeGames) {
        await runSnapshot(
          () =>
            api.saveException({
              ...emptyRule(),
              process: app.process,
              mode: "always",
            }),
          `${app.name} allowed`,
        );
      }
    } else if (rule.mode === "always") {
      if (!rule.enabled) {
        await runSnapshot(
          () => api.setExceptionEnabled(rule.id, true),
          `${app.name} allowed`,
        );
      }
    } else if (config.settings.excludeGames) {
      await runSnapshot(
        () =>
          api.saveException({
            ...rule,
            mode: "always",
            profileId: null,
            enabled: true,
          }),
        `${app.name} allowed`,
      );
    } else if (rule.mode === "disable") {
      await runSnapshot(
        () => api.deleteException(rule.id),
        `${app.name} allowed`,
      );
    } else if (!rule.enabled) {
      await runSnapshot(
        () => api.setExceptionEnabled(rule.id, true),
        `${app.name} allowed`,
      );
    }
    setGameBusy(null);
  };

  const toggleGameGuard = async (excludeGames: boolean) => {
    await runSnapshot(
      () =>
        api.updateSettings({
          ...config.settings,
          excludeGames,
        }),
      excludeGames
        ? "Games are paused by default"
        : "Games use normal mappings",
    );
  };

  const openNew = () => {
    setEditing(null);
    setDraft(emptyRule());
    setError(null);
    setEditorOpen(true);
  };

  const openEdit = (rule: ExceptionRule) => {
    setEditing(rule);
    setDraft({ ...rule });
    setError(null);
    setEditorOpen(true);
  };

  const saveRule = async () => {
    if (!draft.process.trim()) {
      setError("Choose an application or type its executable name.");
      return;
    }
    if (draft.mode === "profile" && !draft.profileId) {
      setError("Choose the profile this app should use.");
      return;
    }

    if (draft.mode === "always") {
      if (editing) {
        await removeRule(editing, "Rule removed — default behaviour applies");
      } else if (selectedIsGame && config.settings.excludeGames) {
        // An explicit rule is the only way to allow a game while the game
        // guard is on; without it the game is excluded by default.
        setSaving(true);
        const result = await runSnapshot(
          () => api.saveException(draft),
          "Game allowed — mappings can run in this game",
        );
        setSaving(false);
        if (result) setEditorOpen(false);
      } else {
        setEditorOpen(false);
      }
      return;
    }

    setSaving(true);
    setError(null);
    const result = await runSnapshot(
      () => api.saveException(draft),
      editing ? "Application rule updated" : "Application rule added",
    );
    setSaving(false);
    if (result) setEditorOpen(false);
  };

  const removeRule = async (rule: ExceptionRule, message = "Application rule deleted") => {
    setDeleting(true);
    const result = await runSnapshot(
      () => api.deleteException(rule.id),
      message,
    );
    setDeleting(false);
    if (result) {
      setDeleteTarget(null);
      setEditorOpen(false);
    }
  };

  const toggleRule = async (rule: ExceptionRule, enabled: boolean) => {
    setBusyId(rule.id);
    await runSnapshot(
      () => api.setExceptionEnabled(rule.id, enabled),
      enabled ? "Rule enabled" : "Rule disabled",
    );
    setBusyId(null);
  };

  const patchScroll = (next: Partial<ScrollSettings>) => {
    if (!draft.scroll) return;
    setDraft({ ...draft, scroll: { ...draft.scroll, ...next } });
  };

  return (
    <div className="animate-in">
      <PageHeader
        eyebrow="Context"
        title="Exceptions"
        description="Decide how MouseMacro behaves in each application. Rules follow the foreground executable name."
        actions={
          <Button variant="primary" icon={<Plus />} onClick={openNew}>
            Add application
          </Button>
        }
      />

      <div className="mb-4 flex flex-wrap items-center gap-2">
        <Badge tone={foreground ? "neutral" : "outline"} dot>
          Foreground: {foreground ?? "no window"}
        </Badge>
        {activeRule ? (
          <Badge
            tone={activeRule.mode === "disable" ? "warning" : "accent"}
            dot
          >
            {MODE_LABEL[activeRule.mode]}
            {activeRule.mode === "profile" && (
              <>
                {" · "}
                {config.profiles.find((p) => p.id === activeRule.profileId)
                  ?.name ?? "missing profile"}
              </>
            )}
          </Badge>
        ) : foregroundIsGuardedGame ? (
          <Badge tone="warning" dot>
            Detected game · mappings paused
          </Badge>
        ) : (
          <Badge tone="success" dot>
            Normal mappings
          </Badge>
        )}
        <button
          onClick={() => void scanApps()}
          className="focus-ring ml-auto flex items-center gap-1 rounded text-[11.5px] font-medium text-zinc-400 transition hover:text-zinc-800 dark:hover:text-zinc-200"
        >
          <RefreshCw className={cn("size-3", loadingApps && "animate-spin")} />
          {loadingApps ? "Scanning installed apps…" : "Rescan installed apps"}
        </button>
      </div>

      <section className="surface mb-5 rounded-2xl">
        <div className="flex items-start gap-3 p-4">
          <div className="grid size-9 shrink-0 place-items-center rounded-xl border border-amber-500/20 bg-amber-500/[0.08] text-amber-500">
            <Gamepad2 className="size-4" />
          </div>
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2">
              <h2 className="text-[13px] font-semibold">
                Pause mappings in games
              </h2>
              <Badge
                tone={
                  detectedGames.length === 0
                    ? "outline"
                    : config.settings.excludeGames
                      ? "success"
                      : "warning"
                }
                dot
              >
                {detectedGames.length === 0
                  ? "No games detected yet"
                  : config.settings.excludeGames
                    ? `${excludedGames} of ${detectedGames.length} excluded`
                    : "Games allowed"}
              </Badge>
            </div>
            <p className="mt-1 text-[11.5px] leading-relaxed text-secondary">
              Detected games are excluded by default. MouseMacro still needs
              to be running, but mappings and injected input stay off while a
              game is in front. This is the safest setting for anti-cheat.
            </p>
          </div>
          <Switch
            checked={config.settings.excludeGames}
            onChange={(enabled) => void toggleGameGuard(enabled)}
            label="Pause mappings in detected games"
          />
        </div>

        {detectedGames.length > 0 && (
          <div className="border-t border-zinc-100 px-4 py-3 dark:border-white/[0.055]">
            <button
              onClick={() => setGamesOpen((open) => !open)}
              className="focus-ring flex w-full items-center gap-2 rounded text-left"
            >
              <span className="text-[11.5px] font-semibold">
                Detected games
              </span>
              <span className="text-[11px] text-zinc-400">
                {detectedGames.length}
              </span>
              <span className="ml-auto text-[11px] font-medium text-zinc-400">
                {gamesOpen ? "Hide" : "Show"} list
              </span>
            </button>
            {gamesOpen && (
              <div className="mt-2 max-h-[min(45vh,28rem)] overflow-y-auto rounded-xl border border-zinc-200/80 dark:border-white/[0.07]">
                {detectedGames.map((app) => {
                  const excluded = gameIsExcluded(app.process);
                  const rule = explicitRule(app.process);
                  return (
                    <div
                      key={app.process}
                      className="flex items-center gap-3 border-b border-zinc-100 px-3 py-2 last:border-0 dark:border-white/[0.05]"
                    >
                      <Gamepad2 className="size-3.5 shrink-0 text-zinc-400" />
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-2">
                          <span className="truncate text-[11.5px] font-medium">
                            {app.name}
                          </span>
                          {app.running && (
                            <Badge tone="accent" dot>
                              running
                            </Badge>
                          )}
                        </div>
                        <span className="block truncate font-mono text-[10.5px] text-zinc-400">
                          {app.process}
                          {rule && (
                            <span className="ml-2 font-sans text-zinc-400">
                              · manual rule
                            </span>
                          )}
                        </span>
                      </div>
                      <span
                        className={cn(
                          "text-[10.5px] font-medium",
                          excluded ? "text-rose-500" : "text-emerald-500",
                        )}
                      >
                        {excluded ? "Excluded" : "Allowed"}
                      </span>
                      <Switch
                        size="sm"
                        checked={excluded}
                        disabled={gameBusy === app.process}
                        onChange={(next) => void toggleGame(app, next)}
                        label={`Pause mappings in ${app.name}`}
                      />
                    </div>
                  );
                })}
              </div>
            )}
            <Callout tone="danger" className="mt-3">
              We do not recommend allowing MouseMacro to run on top of a game.
              Anti-cheat systems can treat injected input as cheating and ban
              the account, even when every action is legitimate.
            </Callout>
          </div>
        )}

        {detectedGames.length === 0 && !loadingApps && (
          <div className="border-t border-zinc-100 px-4 py-3 text-[11.5px] text-secondary dark:border-white/[0.055]">
            No installed games were found. Use{" "}
            <span className="font-medium">Rescan installed apps</span>, or add
            a game manually below.
          </div>
        )}
      </section>

      {rules.length === 0 ? (
        <div className="surface rounded-2xl">
          <EmptyState
            icon={ShieldAlert}
            title="No application rules"
            description="Mappings currently apply everywhere. Add an app to turn them off in a game, or switch to a different profile in an editor."
            action={
              <Button variant="primary" icon={<Plus />} onClick={openNew}>
                Add an application
              </Button>
            }
          />
        </div>
      ) : (
        <div className="surface divide-y divide-zinc-100 overflow-hidden rounded-2xl dark:divide-white/[0.055]">
          {rules.map((rule) => {
            const isCurrent = foreground === rule.process;
            const profile = config.profiles.find(
              (item) => item.id === rule.profileId,
            );
            return (
              <div
                key={rule.id}
                className={cn(
                  "flex items-center gap-3 px-4 py-3.5",
                  isCurrent && "bg-zinc-50/80 dark:bg-white/[0.03]",
                  !rule.enabled && "opacity-55",
                )}
              >
                <div
                  className={cn(
                    "grid size-8 shrink-0 place-items-center rounded-lg border",
                    rule.mode === "disable"
                      ? "border-rose-500/20 bg-rose-500/[0.07] text-rose-500"
                      : rule.mode === "always"
                        ? "border-emerald-500/20 bg-emerald-500/[0.07] text-emerald-500"
                        : "border-indigo-500/20 bg-indigo-500/[0.07] text-indigo-500",
                  )}
                >
                  {rule.mode === "disable" ? (
                    <ShieldAlert className="size-4" />
                  ) : rule.mode === "always" ? (
                    <ShieldCheck className="size-4" />
                  ) : (
                    <Layers className="size-4" />
                  )}
                </div>
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <span className="truncate font-mono text-[12.5px] font-medium">
                      {rule.process}
                    </span>
                    {isCurrent && (
                      <Badge tone="accent" dot>
                        in focus
                      </Badge>
                    )}
                  </div>
                  <div className="mt-0.5 text-[11.5px] text-secondary">
                    {rule.mode === "disable"
                      ? "Physical input passes through untouched."
                      : rule.mode === "always"
                        ? "Mappings run normally here, even in detected games."
                        : `Use “${profile?.name ?? "missing profile"}” while this app is active.`}
                  </div>
                </div>
                <div className="flex shrink-0 items-center gap-1">
                  <Switch
                    size="sm"
                    checked={rule.enabled}
                    disabled={busyId === rule.id}
                    onChange={(enabled) => toggleRule(rule, enabled)}
                    label={`Enable rule for ${rule.process}`}
                  />
                  <Button
                    size="icon"
                    variant="ghost"
                    onClick={() => openEdit(rule)}
                    title="Edit rule"
                    aria-label="Edit rule"
                  >
                    <Pencil className="size-3.5" />
                  </Button>
                  <Button
                    size="icon"
                    variant="ghost"
                    className="hover:bg-rose-50 hover:text-rose-600 dark:hover:bg-rose-500/10 dark:hover:text-rose-400"
                    onClick={() => setDeleteTarget(rule)}
                    title="Delete rule"
                    aria-label="Delete rule"
                  >
                    <Trash2 className="size-3.5" />
                  </Button>
                </div>
              </div>
            );
          })}
        </div>
      )}

      <Callout tone="info" className="mt-4">
        Rules match the executable name, so a rule for <code>game.exe</code>{" "}
        applies wherever that build runs. Elevated applications, and the secure
        desktop (UAC, lock screen), need MouseMacro to be elevated as well.
      </Callout>

      <Modal
        open={editorOpen}
        onClose={() => setEditorOpen(false)}
        title={editing ? "Edit application rule" : "Add application rule"}
        description="Pick the app, then choose what MouseMacro should do while it is in front."
        size="lg"
        footer={
          <>
            <Button variant="ghost" onClick={() => setEditorOpen(false)}>
              Cancel
            </Button>
            <Button variant="primary" onClick={saveRule} loading={saving}>
              {draft.mode === "always" && editing
                ? "Remove rule"
                : editing
                  ? "Save rule"
                  : "Add rule"}
            </Button>
          </>
        }
      >
        <div className="space-y-4">
          <Field
            label="Application"
            hint="Executable name only, for example chrome.exe or game.exe."
          >
            <div className="flex gap-2">
              <input
                value={draft.process}
                onChange={(event) =>
                  setDraft({ ...draft, process: event.target.value })
                }
                placeholder="application.exe"
                className="focus-ring h-9 min-w-0 flex-1 rounded-[9px] border border-zinc-200 bg-white px-3 font-mono text-[12px] placeholder:text-zinc-400 dark:border-white/10 dark:bg-white/[0.04] dark:placeholder:text-zinc-600"
              />
              {foreground && (
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() =>
                    setDraft({ ...draft, process: foreground })
                  }
                >
                  Use current
                </Button>
              )}
            </div>
          </Field>

          <Field label="Behaviour">
            <Segmented
              value={draft.mode}
              onChange={(mode) =>
                setDraft({
                  ...draft,
                  mode,
                  profileId:
                    mode === "profile"
                      ? draft.profileId ?? config.profiles[0]?.id ?? null
                      : null,
                })
              }
              options={[
                { value: "always", label: "Always on" },
                { value: "disable", label: "Mappings off" },
                { value: "profile", label: "Use profile" },
              ]}
            />
          </Field>

          {selectedIsGame && draft.mode !== "disable" && (
            <Callout tone="danger">
              This looks like a game. We do not recommend allowing MouseMacro
              to run on top of games: anti-cheat systems can treat injected
              input as cheating and ban the account.
            </Callout>
          )}

          {draft.mode === "profile" && (
            <Field label="Profile">
              <Select
                value={draft.profileId ?? ""}
                onChange={(event) =>
                  setDraft({ ...draft, profileId: event.target.value || null })
                }
              >
                {config.profiles.map((profile) => (
                  <option key={profile.id} value={profile.id}>
                    {profile.name}
                  </option>
                ))}
              </Select>
            </Field>
          )}

          <div className="rounded-xl border border-zinc-200/80 px-3 py-2.5 dark:border-white/[0.07]">
            <div className="flex items-center justify-between gap-3">
              <div className="min-w-0">
                <div className="text-[12.5px] font-medium">Scroll override</div>
                <div className="text-[11px] leading-relaxed text-secondary">
                  {draft.scroll
                    ? "This app uses its own wheel direction and speed."
                    : draft.mode === "disable"
                      ? "Mappings off leaves the wheel untouched."
                      : "Inherits the global Scroll settings."}
                </div>
              </div>
              <Switch
                checked={draft.scroll !== null}
                onChange={(enabled) =>
                  setDraft({
                    ...draft,
                    scroll: enabled ? { ...config.settings.scroll } : null,
                  })
                }
                label="Override scroll for this app"
              />
            </div>

            {draft.scroll && (
              <div className="mt-3 space-y-2 border-t border-zinc-100 pt-3 dark:border-white/[0.05]">
                <CompactToggle
                  label="Invert vertical"
                  checked={draft.scroll.invertVertical}
                  onChange={(invertVertical) => patchScroll({ invertVertical })}
                />
                <CompactToggle
                  label="Invert horizontal"
                  checked={draft.scroll.invertHorizontal}
                  onChange={(invertHorizontal) =>
                    patchScroll({ invertHorizontal })
                  }
                />
                <CompactToggle
                  label="Swap axes"
                  checked={draft.scroll.swapAxes}
                  onChange={(swapAxes) => patchScroll({ swapAxes })}
                />
                <CompactToggle
                  label="Acceleration"
                  checked={draft.scroll.acceleration}
                  onChange={(acceleration) => patchScroll({ acceleration })}
                />
                <div className="flex items-center justify-between gap-3 pt-1">
                  <span className="text-[11.5px]">Speed</span>
                  <Segmented
                    size="sm"
                    value={String(draft.scroll.speed)}
                    onChange={(value) => patchScroll({ speed: Number(value) })}
                    options={[1, 2, 3, 4, 5].map((speed) => ({
                      value: String(speed),
                      label: `×${speed}`,
                    }))}
                  />
                </div>
              </div>
            )}
          </div>

          {draft.mode === "always" && (
            <Callout tone="info">
              <span className="flex items-start gap-2">
                <Monitor className="mt-0.5 size-3.5 shrink-0" />
                This app uses the normal global behaviour. Saving removes an
                existing rule.
              </span>
            </Callout>
          )}

          <div className="rounded-xl border border-zinc-200/80 px-3 py-2.5 dark:border-white/[0.07]">
            <div className="flex items-center justify-between gap-3">
              <div className="min-w-0">
                <div className="text-[12.5px] font-medium">
                  Keyboard remapping off
                </div>
                <div className="text-[11px] leading-relaxed text-secondary">
                  {draft.mode === "disable"
                    ? "Mappings off already passes every key through."
                    : "Keys pass through untouched in this app. Mouse mappings still apply."}
                </div>
              </div>
              <Switch
                checked={draft.keyboardDisabled}
                onChange={(keyboardDisabled) =>
                  setDraft({ ...draft, keyboardDisabled })
                }
                label="Disable keyboard remapping in this app"
              />
            </div>
          </div>

          <div className="flex items-center justify-between rounded-xl border border-zinc-200/80 px-3 py-2.5 dark:border-white/[0.07]">
            <div>
              <div className="text-[12.5px] font-medium">Rule enabled</div>
              <div className="text-[11px] text-secondary">
                Disabled rules stay saved but are ignored.
              </div>
            </div>
            <Switch
              checked={draft.enabled}
              onChange={(enabled) => setDraft({ ...draft, enabled })}
              label="Rule enabled"
            />
          </div>

          <div>
            <div className="mb-2 flex flex-wrap items-center gap-2">
              <span className="text-[12.5px] font-medium">
                Installed applications
              </span>
              <span className="text-[11px] text-zinc-400">
                {apps.length} found
              </span>
              <div className="relative ml-auto">
                <Search className="pointer-events-none absolute left-2 top-1/2 size-3 -translate-y-1/2 text-zinc-400" />
                <input
                  value={search}
                  onChange={(event) => setSearch(event.target.value)}
                  placeholder="Search apps and games"
                  className="focus-ring h-7 w-48 rounded-lg border border-zinc-200 bg-white pl-7 pr-2 text-[11.5px] dark:border-white/10 dark:bg-white/[0.04]"
                />
              </div>
            </div>
            <div className="mb-2">
              <Segmented
                size="sm"
                value={appFilter}
                onChange={(value) =>
                  setAppFilter(value as "all" | "games" | "running")
                }
                options={[
                  { value: "all", label: "All" },
                  {
                    value: "games",
                    label: `Games (${detectedGames.length})`,
                  },
                  { value: "running", label: "Running" },
                ]}
              />
            </div>
            <div className="max-h-[min(55vh,32rem)] overflow-y-auto rounded-xl border border-zinc-200/80 dark:border-white/[0.07]">
              {loadingApps && apps.length === 0 ? (
                <div className="px-3 py-4 text-center text-[11.5px] text-zinc-400">
                  Scanning installed applications…
                </div>
              ) : filteredApps.length === 0 ? (
                <div className="px-3 py-4 text-center text-[11.5px] text-zinc-400">
                  No matching application. Type the executable name above.
                </div>
              ) : (
                filteredApps.map((app) => {
                  const hasRule = rules.some(
                    (rule) => rule.process === app.process,
                  );
                  return (
                    <button
                      key={app.process}
                      onClick={() =>
                        setDraft({ ...draft, process: app.process })
                      }
                      className={cn(
                        "flex w-full items-center gap-2 border-b border-zinc-100 px-3 py-2 text-left last:border-0 hover:bg-zinc-50 dark:border-white/[0.05] dark:hover:bg-white/[0.04]",
                        draft.process === app.process &&
                          "bg-indigo-500/[0.06] dark:bg-indigo-500/10",
                      )}
                    >
                      {app.game ? (
                        <Gamepad2 className="size-3.5 shrink-0 text-amber-500" />
                      ) : (
                        <Monitor className="size-3.5 shrink-0 text-zinc-400" />
                      )}
                      <span className="min-w-0 flex-1">
                        <span className="block truncate text-[11.5px] font-medium">
                          {app.name}
                        </span>
                        <span className="block truncate font-mono text-[10.5px] text-zinc-400">
                          {app.process}
                          {app.source === "steam" && " · Steam library"}
                          {app.source === "epic" && " · Epic library"}
                          {app.source === "riot" && " · Riot library"}
                        </span>
                      </span>
                      {app.running && (
                        <Badge tone="accent" dot>
                          running
                        </Badge>
                      )}
                      {app.game && <Badge tone="warning">game</Badge>}
                      {hasRule && <Badge tone="outline">rule</Badge>}
                      {draft.process === app.process && (
                        <Badge tone="accent">selected</Badge>
                      )}
                    </button>
                  );
                })
              )}
            </div>
            <div className="mt-1.5 text-[10.5px] leading-relaxed text-zinc-400">
              Installed apps come from the Start menu, Steam and Epic
              libraries, plus apps that are currently running.
            </div>
          </div>

          {error && (
            <Callout tone="danger">{error}</Callout>
          )}
        </div>
      </Modal>

      <ConfirmDialog
        open={deleteTarget !== null}
        onClose={() => setDeleteTarget(null)}
        onConfirm={() => {
          if (deleteTarget) void removeRule(deleteTarget);
        }}
        loading={deleting}
        title={`Delete the rule for ${deleteTarget?.process ?? "this app"}?`}
        description="The app will use the normal global behaviour again."
        confirmLabel="Delete rule"
      />
    </div>
  );
}

function CompactToggle({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <div className="flex items-center justify-between gap-3">
      <span className="text-[11.5px]">{label}</span>
      <Switch size="sm" checked={checked} onChange={onChange} label={label} />
    </div>
  );
}
