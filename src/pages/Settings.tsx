import { useState } from "react";
import {
  Activity,
  ArrowDown,
  ArrowUp,
  ChevronRight,
  FolderOpen,
  Gauge,
  Lock,
  Monitor,
  Moon,
  Palette,
  RefreshCw,
  RotateCcw,
  Settings2,
  ShieldAlert,
  ShieldCheck,
  SlidersHorizontal,
  Sun,
  Timer,
  Zap,
} from "lucide-react";
import { useApp } from "../state/AppContext";
import { useToast } from "../components/Toast";
import { PageHeader } from "../components/PageHeader";
import { Button } from "../components/Button";
import { Switch } from "../components/Switch";
import { Select } from "../components/Select";
import { Badge } from "../components/Badge";
import { Callout } from "../components/Callout";
import { ConfirmDialog } from "../components/Modal";
import { InputDiagnostics } from "../components/InputDiagnostics";
import { api, errorMessage } from "../lib/api";
import {
  RULE_CATEGORY_HINT,
  RULE_CATEGORY_LABEL,
} from "../lib/format";
import type { RuleCategory, Settings } from "../lib/types";
import { cn } from "../lib/cn";

const LONG_PRESS_OPTIONS = [250, 300, 350, 400, 450, 500, 600, 700, 800];
const DOUBLE_CLICK_OPTIONS = [150, 200, 250, 280, 320, 350, 400, 450, 500];
const RELEASE_CATEGORIES: RuleCategory[] = [
  "doubleClick",
  "release",
  "singleClick",
];

export function SettingsPage() {
  const { snapshot, runSnapshot, run } = useApp();
  const { toast } = useToast();
  const [saving, setSaving] = useState(false);
  const [resetOpen, setResetOpen] = useState(false);
  const [resetting, setResetting] = useState(false);
  const [restarting, setRestarting] = useState(false);
  const [reloading, setReloading] = useState(false);

  if (!snapshot) return null;
  const settings = snapshot.config.settings;
  const status = snapshot.status;

  const save = async (patch: Partial<Settings>) => {
    setSaving(true);
    await runSnapshot(() =>
      api.updateSettings({ ...settings, ...patch }),
    );
    setSaving(false);
  };

  const setEnabled = async (enabled: boolean) => {
    setSaving(true);
    await runSnapshot(() => api.setEnabled(enabled));
    setSaving(false);
  };

  const configurableOrder: RuleCategory[] = (() => {
    const ordered = settings.rulePriority.filter((category) =>
      RELEASE_CATEGORIES.includes(category),
    );
    for (const category of RELEASE_CATEGORIES) {
      if (!ordered.includes(category)) ordered.push(category);
    }
    return ordered;
  })();

  const movePriority = (category: RuleCategory, direction: -1 | 1) => {
    const index = configurableOrder.indexOf(category);
    const target = index + direction;
    if (target < 0 || target >= configurableOrder.length) return;
    const next = [...configurableOrder];
    [next[index], next[target]] = [next[target], next[index]];
    void save({ rulePriority: ["gesture", "longPress", ...next] });
  };

  const resetPriority = () => {
    void save({
      rulePriority: [
        "gesture",
        "longPress",
        "doubleClick",
        "release",
        "singleClick",
      ],
    });
  };

  const resetConfiguration = async () => {
    setResetting(true);
    const result = await runSnapshot(
      () => api.resetConfig(),
      "Configuration reset to defaults",
    );
    setResetting(false);
    if (result) setResetOpen(false);
  };

  const openConfigFolder = async () => {
    await run(() => api.openConfigFolder());
  };

  const reloadConfiguration = async () => {
    setReloading(true);
    await runSnapshot(
      () => api.reloadConfig(),
      "Configuration reloaded from disk",
    );
    setReloading(false);
  };

  const restartElevated = async () => {
    setRestarting(true);
    try {
      await api.relaunchElevated();
      toast("Restarting MouseMacro as administrator…", "info");
    } catch (error) {
      toast(errorMessage(error), "error");
      setRestarting(false);
    }
  };

  return (
    <div className="animate-in">
      <PageHeader
        eyebrow="Preferences"
        title="Settings"
        description="Control startup behaviour, appearance, timing and the input engine."
      />

      <div className="space-y-4">
        <SettingCard
          icon={Settings2}
          title="General"
          description="How MouseMacro behaves with Windows and its window."
        >
          <SettingRow
            title="Launch on Windows startup"
            description="Register MouseMacro to start automatically when you sign in."
            control={
              <Switch
                checked={status.autostart}
                disabled={saving}
                onChange={(value) =>
                  save({ launchAtStartup: value })
                }
                label="Launch on Windows startup"
              />
            }
          />
          <SettingRow
            title="Start minimized to the system tray"
            description="Open quietly in the tray so mappings are active immediately."
            control={
              <Switch
                checked={settings.startMinimized}
                disabled={saving}
                onChange={(value) => save({ startMinimized: value })}
                label="Start minimized"
              />
            }
          />
          <SettingRow
            title="Close window to tray"
            description="Keep the engine running when the window is closed. Use the tray menu to exit."
            control={
              <Switch
                checked={settings.closeToTray}
                disabled={saving}
                onChange={(value) => save({ closeToTray: value })}
                label="Close to tray"
              />
            }
            last
          />
        </SettingCard>

        <SettingCard
          icon={Palette}
          title="Appearance"
          description="A quiet interface that follows your preference."
        >
          <div className="grid gap-2 sm:grid-cols-3">
            {(
              [
                { value: "system", label: "System", icon: Monitor },
                { value: "light", label: "Light", icon: Sun },
                { value: "dark", label: "Dark", icon: Moon },
              ] as const
            ).map((option) => {
              const Icon = option.icon;
              const active = settings.theme === option.value;
              return (
                <button
                  key={option.value}
                  onClick={() => save({ theme: option.value })}
                  disabled={saving}
                  className={cn(
                    "focus-ring group relative overflow-hidden rounded-xl border p-3 text-left transition",
                    active
                      ? "border-zinc-900/20 bg-zinc-50 ring-1 ring-zinc-900/10 dark:border-white/25 dark:bg-white/[0.055] dark:ring-white/15"
                      : "border-zinc-200/80 hover:border-zinc-300 dark:border-white/[0.07] dark:hover:border-white/15",
                  )}
                >
                  <div
                    className={cn(
                      "mb-2.5 h-12 overflow-hidden rounded-lg border",
                      option.value === "dark"
                        ? "border-white/10 bg-[#0f0f11]"
                        : option.value === "light"
                          ? "border-zinc-200 bg-[#f6f6f7]"
                          : "border-zinc-200 bg-gradient-to-r from-[#f6f6f7] to-[#17171a] dark:border-white/10",
                    )}
                  >
                    <div className="flex h-full">
                      <div
                        className={cn(
                          "w-1/4 border-r",
                          option.value === "dark"
                            ? "border-white/10 bg-black/20"
                            : "border-zinc-200 bg-white/60",
                        )}
                      />
                      <div className="flex flex-1 flex-col justify-center gap-1 px-2">
                        <div
                          className={cn(
                            "h-1.5 w-3/4 rounded-full",
                            option.value === "light"
                              ? "bg-zinc-300"
                              : "bg-white/20",
                          )}
                        />
                        <div
                          className={cn(
                            "h-1.5 w-1/2 rounded-full",
                            option.value === "light"
                              ? "bg-zinc-200"
                              : "bg-white/10",
                          )}
                        />
                      </div>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <Icon className="size-3.5 text-zinc-400" />
                    <span className="text-[12.5px] font-medium">
                      {option.label}
                    </span>
                    {active && (
                      <span className="ml-auto size-1.5 rounded-full bg-zinc-900 dark:bg-white" />
                    )}
                  </div>
                </button>
              );
            })}
          </div>
        </SettingCard>

        <SettingCard
          icon={Zap}
          title="Input engine"
          description="Global mappings, gesture timings and conflict resolution."
        >
          <SettingRow
            title="Global mappings"
            description="When off, all physical input passes through untouched."
            control={
              <div className="flex items-center gap-3">
                <Badge tone={settings.enabled ? "success" : "neutral"} dot>
                  {settings.enabled ? "Active" : "Paused"}
                </Badge>
                <Switch
                  checked={settings.enabled}
                  disabled={saving}
                  onChange={setEnabled}
                  label="Global mappings"
                />
              </div>
            }
          />
          <SettingRow
            title="Long-press threshold"
            description="How long a button must be held before a long-press rule fires."
            control={
              <Select
                value={String(settings.longPressMs)}
                disabled={saving}
                onChange={(event) =>
                  save({ longPressMs: Number(event.target.value) })
                }
                className="w-[130px]"
              >
                {LONG_PRESS_OPTIONS.map((value) => (
                  <option key={value} value={value}>
                    {value} ms
                  </option>
                ))}
                {!LONG_PRESS_OPTIONS.includes(settings.longPressMs) && (
                  <option value={settings.longPressMs}>
                    {settings.longPressMs} ms
                  </option>
                )}
              </Select>
            }
          />
          <SettingRow
            title="Double-click window"
            description="Maximum gap between two clicks of the same button."
            control={
              <Select
                value={String(settings.doubleClickMs)}
                disabled={saving}
                onChange={(event) =>
                  save({ doubleClickMs: Number(event.target.value) })
                }
                className="w-[130px]"
              >
                {DOUBLE_CLICK_OPTIONS.map((value) => (
                  <option key={value} value={value}>
                    {value} ms
                  </option>
                ))}
                {!DOUBLE_CLICK_OPTIONS.includes(settings.doubleClickMs) && (
                  <option value={settings.doubleClickMs}>
                    {settings.doubleClickMs} ms
                  </option>
                )}
              </Select>
            }
          />
          <div className="px-4 pb-4 pt-3">
            <div className="mb-2.5 flex items-center gap-2">
              <SlidersHorizontal className="size-3.5 text-zinc-400" />
              <span className="text-[12.5px] font-medium">
                Resolution order
              </span>
              <button
                onClick={resetPriority}
                disabled={saving}
                className="focus-ring ml-auto text-[11px] font-medium text-zinc-400 transition hover:text-zinc-800 dark:hover:text-zinc-200"
              >
                Reset order
              </button>
            </div>
            <p className="mb-3 text-[11.5px] leading-relaxed text-secondary">
              Gestures and long presses always resolve first. This order decides
              which rule wins when a double click, a release rule and a single
              click all match the same button.
            </p>

            <div className="mb-2 flex items-center gap-2 rounded-lg border border-dashed border-zinc-200 px-3 py-2 dark:border-white/[0.07]">
              <Lock className="size-3 text-zinc-400" />
              <span className="text-[11.5px] font-medium text-zinc-500 dark:text-zinc-400">
                Gesture
              </span>
              <span className="text-[11px] text-secondary">
                hold + wheel or hold + button
              </span>
              <span className="ml-auto text-[10px] font-medium uppercase tracking-wider text-zinc-400">
                Highest
              </span>
            </div>
            <div className="mb-2 flex items-center gap-2 rounded-lg border border-dashed border-zinc-200 px-3 py-2 dark:border-white/[0.07]">
              <Lock className="size-3 text-zinc-400" />
              <span className="text-[11.5px] font-medium text-zinc-500 dark:text-zinc-400">
                Long press
              </span>
              <span className="text-[11px] text-secondary">
                fires at {settings.longPressMs} ms
              </span>
              <span className="ml-auto text-[10px] font-medium uppercase tracking-wider text-zinc-400">
                Fixed
              </span>
            </div>

            <div className="space-y-1.5">
              {configurableOrder.map((category, index) => (
                <div
                  key={category}
                  className="flex items-center gap-2.5 rounded-lg border border-zinc-200/80 bg-white px-3 py-2 dark:border-white/[0.07] dark:bg-white/[0.025]"
                >
                  <span className="grid size-5 place-items-center rounded bg-zinc-100 text-[10px] font-semibold text-zinc-500 dark:bg-white/[0.06] dark:text-zinc-400">
                    {index + 1}
                  </span>
                  <div className="min-w-0 flex-1">
                    <div className="text-[11.5px] font-medium">
                      {RULE_CATEGORY_LABEL[category]}
                    </div>
                    <div className="text-[10.5px] text-secondary">
                      {RULE_CATEGORY_HINT[category]}
                    </div>
                  </div>
                  <div className="flex items-center gap-0.5">
                    <button
                      onClick={() => movePriority(category, -1)}
                      disabled={index === 0 || saving}
                      aria-label="Move rule up"
                      className="focus-ring grid size-6 place-items-center rounded text-zinc-400 transition hover:bg-zinc-100 hover:text-zinc-700 disabled:opacity-25 dark:hover:bg-white/[0.07] dark:hover:text-zinc-200"
                    >
                      <ArrowUp className="size-3" />
                    </button>
                    <button
                      onClick={() => movePriority(category, 1)}
                      disabled={
                        index === configurableOrder.length - 1 || saving
                      }
                      aria-label="Move rule down"
                      className="focus-ring grid size-6 place-items-center rounded text-zinc-400 transition hover:bg-zinc-100 hover:text-zinc-700 disabled:opacity-25 dark:hover:bg-white/[0.07] dark:hover:text-zinc-200"
                    >
                      <ArrowDown className="size-3" />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </SettingCard>

        <SettingCard
          icon={Activity}
          title="Input diagnostics"
          description="Verify what the input hook really receives. Use this when a mapping does not fire."
        >
          <SettingRow
            title="Process input from other input tools"
            description="Mouse utilities, touchpad drivers and remote desktop synthesize events. Enable this if scrolling or side buttons from those tools never reach MouseMacro. MouseMacro's own synthetic input is always ignored."
            control={
              <Switch
                checked={settings.processInjectedInput}
                disabled={saving}
                onChange={(value) => save({ processInjectedInput: value })}
                label="Process input from other input tools"
              />
            }
          />
          <div className="px-4 pb-4 pt-3">
            <InputDiagnostics />
          </div>
        </SettingCard>

        <SettingCard
          icon={FolderOpen}
          title="Configuration & maintenance"
          description="Local, portable JSON storage. Nothing leaves your machine."
        >
          <div className="px-4 pb-4 pt-3">
            <div className="flex items-center gap-2 text-[12px] font-medium">
              <Gauge className="size-3.5 text-zinc-400" />
              Configuration file
            </div>
            <div className="mt-2 flex items-center gap-2 rounded-lg border border-zinc-200/80 bg-zinc-50 px-3 py-2 dark:border-white/[0.07] dark:bg-white/[0.025]">
              <code className="min-w-0 flex-1 truncate font-mono text-[11px] text-zinc-500 dark:text-zinc-400">
                {status.configPath}
              </code>
              <Button
                size="sm"
                variant="ghost"
                icon={<FolderOpen />}
                onClick={openConfigFolder}
              >
                Open folder
              </Button>
            </div>
            <p className="mt-2 text-[11px] leading-relaxed text-zinc-400 dark:text-zinc-500">
              Changes are written atomically. A timestamped backup is kept when
              the file cannot be parsed.
            </p>
          </div>

          <SettingRow
            title="Reload from disk"
            description="Pick up hand-edits to config.json without restarting. The file is re-read and normalised."
            control={
              <Button
                size="sm"
                variant="secondary"
                icon={<RefreshCw />}
                loading={reloading}
                onClick={reloadConfiguration}
              >
                Reload
              </Button>
            }
          />

          <SettingRow
            title="Run as administrator"
            description={
              status.elevated
                ? "MouseMacro can hook and simulate input for elevated applications."
                : "Windows blocks input injection into elevated apps unless MouseMacro is elevated too."
            }
            control={
              status.elevated ? (
                <Badge tone="success" dot>
                  <ShieldCheck className="size-3" /> Elevated
                </Badge>
              ) : (
                <Button
                  size="sm"
                  variant="secondary"
                  icon={<ShieldAlert />}
                  loading={restarting}
                  onClick={restartElevated}
                >
                  Restart elevated
                </Button>
              )
            }
          />
          <SettingRow
            title="Reset configuration"
            description="Restore the sample profiles and default settings. All custom mappings are removed."
            control={
              <Button
                size="sm"
                variant="secondary"
                icon={<RotateCcw />}
                onClick={() => setResetOpen(true)}
                className="text-rose-600 hover:text-rose-700 dark:text-rose-400"
              >
                Reset…
              </Button>
            }
            last
          />
        </SettingCard>
      </div>

      <Callout tone="warning" className="mt-4">
        <span className="flex items-start gap-2">
          <Timer className="mt-0.5 size-3.5 shrink-0" />
          Timing changes take effect immediately. A pending click from an old
          rule is cancelled and its synthetic holds are released.
        </span>
      </Callout>

      <ConfirmDialog
        open={resetOpen}
        onClose={() => setResetOpen(false)}
        onConfirm={resetConfiguration}
        loading={resetting}
        title="Reset everything?"
        description="This deletes all custom profiles and mappings, turns the engine off and disables launch at startup."
        confirmLabel="Reset configuration"
      >
        <Callout tone="danger">
          This cannot be undone. Export any profile you want to keep first.
        </Callout>
      </ConfirmDialog>
    </div>
  );
}

function SettingCard({
  icon: Icon,
  title,
  description,
  children,
}: {
  icon: typeof Settings2;
  title: string;
  description: string;
  children: React.ReactNode;
}) {
  return (
    <section className="surface overflow-hidden rounded-2xl">
      <div className="flex items-start gap-3 border-b border-zinc-100 px-4 py-3.5 dark:border-white/[0.055]">
        <div className="grid size-8 shrink-0 place-items-center rounded-lg bg-zinc-100 text-zinc-500 dark:bg-white/[0.06] dark:text-zinc-400">
          <Icon className="size-4" strokeWidth={1.7} />
        </div>
        <div>
          <h2 className="text-[13.5px] font-semibold">{title}</h2>
          <p className="mt-0.5 text-[11.5px] text-secondary">{description}</p>
        </div>
        <ChevronRight className="ml-auto mt-1.5 size-4 text-zinc-300 dark:text-zinc-700" />
      </div>
      {children}
    </section>
  );
}

function SettingRow({
  title,
  description,
  control,
  last,
}: {
  title: string;
  description: string;
  control: React.ReactNode;
  last?: boolean;
}) {
  return (
    <div
      className={cn(
        "flex items-center justify-between gap-6 px-4 py-3.5",
        !last && "border-b border-zinc-100 dark:border-white/[0.055]",
      )}
    >
      <div className="min-w-0">
        <div className="text-[12.5px] font-medium">{title}</div>
        <p className="mt-0.5 max-w-[560px] text-[11.5px] leading-relaxed text-secondary">
          {description}
        </p>
      </div>
      <div className="shrink-0">{control}</div>
    </div>
  );
}
