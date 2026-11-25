import { useState, useEffect } from "react";
import {
  Activity,
  ArrowRight,
  Check,
  CircleDot,
  Clock,
  Layers,
  Mouse,
  MousePointerClick,
  Play,
  Power,
  Radar,
  RotateCcw,
  Sparkles,
  Square,
  Zap,
} from "lucide-react";
import { useApp } from "../state/AppContext";
import { useToast } from "../components/Toast";
import { api } from "../lib/api";
import { PageHeader, SectionTitle } from "../components/PageHeader";
import { Callout } from "../components/Callout";
import { Badge } from "../components/Badge";
import { Button } from "../components/Button";
import { Switch } from "../components/Switch";
import { MouseButtonBadge, MouseGlyph } from "../components/MouseButtonBadge";
import { ActionDisplay } from "../components/ActionDisplay";
import { useCapture } from "../hooks/useCapture";
import type { CapturePayload, MouseButton, ScrollDirection } from "../lib/types";
import {
  actionLabel,
  buttonLabel,
  sortMappings,
  triggerDescription,
  triggerLabel,
} from "../lib/format";
import { cn } from "../lib/cn";

export function DashboardPage() {
  const { snapshot, runSnapshot, setView } = useApp();
  const { toast } = useToast();
  const [busy, setBusy] = useState(false);
  const [listening, setListening] = useState(false);
  const [last, setLast] = useState<CapturePayload | null>(null);

  const ready = useCapture(
    listening ? "monitor" : "none",
    (payload) => {
      if (payload.kind === "mouse") setLast(payload);
    },
  );

  useEffect(() => {
    if (!listening) return;
    const timer = window.setTimeout(() => setListening(false), 25_000);
    return () => window.clearTimeout(timer);
  }, [listening]);

  if (!snapshot) return null;
  const { config, status } = snapshot;
  const profile = config.profiles.find((p) => p.id === config.activeProfileId);
  const mappings = profile?.mappings ?? [];
  const enabledMappings = mappings.filter((mapping) => mapping.enabled);
  const active = config.settings.enabled;
  const recent = sortMappings(mappings).slice(0, 4);

  const toggle = async () => {
    setBusy(true);
    await runSnapshot(
      () => api.setEnabled(!active),
      !active ? "Mouse mappings enabled" : "Mouse mappings paused",
    );
    setBusy(false);
  };

  const dismissWarnings = async () => {
    const next = await runSnapshot(() => api.clearWarnings());
    if (next) toast("Notice dismissed", "info");
  };

  const warnings = [
    status.loadWarning,
    status.hookWarning,
    status.lastError,
  ].filter((value): value is string => Boolean(value));

  return (
    <div className="animate-in">
      <PageHeader
        eyebrow="Overview"
        title="Dashboard"
        description="Your mouse, tuned to the way you work. Everything runs natively in the background."
        actions={
          <>
            <Button
              variant="secondary"
              icon={<Layers />}
              onClick={() => setView("profiles")}
            >
              {config.profiles.length} profiles
            </Button>
            <Button
              variant="primary"
              icon={<MousePointerClick />}
              onClick={() => setView("mappings")}
            >
              Manage mappings
            </Button>
          </>
        }
      />

      {warnings.length > 0 && (
        <Callout
          tone="warning"
          title="Input engine notice"
          className="mb-4"
          action={
            <Button size="sm" variant="ghost" onClick={dismissWarnings}>
              Dismiss
            </Button>
          }
        >
          {warnings.join(" ")}
        </Callout>
      )}

      <section className="surface relative overflow-hidden rounded-2xl">
        <div
          className={cn(
            "pointer-events-none absolute inset-0 opacity-90 transition-opacity duration-500",
            active
              ? "bg-[radial-gradient(120%_160%_at_85%_-20%,rgba(16,185,129,0.12),transparent_55%)]"
              : "bg-[radial-gradient(120%_160%_at_85%_-20%,rgba(120,120,130,0.09),transparent_55%)]",
          )}
        />
        <div className="relative flex flex-col gap-6 p-6 sm:flex-row sm:items-center sm:justify-between">
          <div className="min-w-0">
            <div className="mb-3 flex flex-wrap items-center gap-2">
              <Badge tone={active ? "success" : "neutral"} dot>
                {active ? "Engine running" : "Engine paused"}
              </Badge>
              <Badge tone="outline">
                {status.hooksActive ? "Hooks installed" : "Hooks not running"}
              </Badge>
              <Badge tone="outline">{profile?.name ?? "No profile"}</Badge>
            </div>
            <h2 className="text-[26px] font-semibold leading-tight tracking-[-0.035em]">
              {active ? "Mappings are live." : "Mappings are paused."}
            </h2>
            <p className="mt-2 max-w-[430px] text-[13px] leading-relaxed text-secondary">
              {active
                ? `MouseMacro is listening globally. ${enabledMappings.length} active mapping${enabledMappings.length === 1 ? "" : "s"} in ${profile?.name ?? "your profile"} will apply in every app.`
                : "Turn the engine on to apply your mappings in every application. Input is never blocked while paused."}
            </p>
          </div>

          <div className="flex items-center gap-3 sm:pr-2">
            <div className="text-right">
              <div className="text-[12px] font-medium">
                {active ? "Enabled" : "Disabled"}
              </div>
              <div className="text-[11px] text-secondary">
                Master switch
              </div>
            </div>
            <Switch
              checked={active}
              disabled={busy}
              onChange={toggle}
              label="Enable MouseMacro mappings"
            />
            <button
              onClick={toggle}
              disabled={busy}
              aria-label={active ? "Pause mappings" : "Enable mappings"}
              className={cn(
                "focus-ring ml-2 grid size-[60px] place-items-center rounded-full border transition-all duration-200 active:scale-[0.97]",
                active
                  ? "border-emerald-500/20 bg-emerald-500/10 text-emerald-600 shadow-[0_0_0_6px_rgba(16,185,129,0.05)] dark:text-emerald-400"
                  : "border-zinc-200 bg-white text-zinc-400 hover:text-zinc-700 dark:border-white/10 dark:bg-white/[0.04] dark:text-zinc-500 dark:hover:text-zinc-200",
              )}
            >
              <Power className="size-6" strokeWidth={1.7} />
            </button>
          </div>
        </div>
      </section>

      <div className="mt-4 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <StatCard
          icon={Mouse}
          label="Configured mappings"
          value={`${enabledMappings.length}`}
          hint={`of ${mappings.length} total · ${profile?.name ?? "—"}`}
        />
        <StatCard
          icon={Activity}
          label="Input detection"
          value={status.hooksActive ? "Global" : "Offline"}
          hint={
            status.hooksActive
              ? "Low-level mouse hook active"
              : "Hooks could not be installed"
          }
          tone={status.hooksActive ? "success" : "danger"}
        />
        <StatCard
          icon={Clock}
          label="Gesture timing"
          value={`${config.settings.longPressMs} ms`}
          hint={`Double click ${config.settings.doubleClickMs} ms`}
        />
        <StatCard
          icon={RotateCcw}
          label="Launch at startup"
          value={status.autostart ? "On" : "Off"}
          hint={status.autostart ? "Starts with Windows" : "Manual start only"}
          tone={status.autostart ? "success" : undefined}
        />
      </div>

      <div className="mt-6 grid gap-4 xl:grid-cols-[1.35fr_1fr]">
        <section>
          <SectionTitle
            action={
              <button
                onClick={() => setView("mappings")}
                className="focus-ring flex items-center gap-1 rounded text-[11.5px] font-medium text-zinc-500 transition hover:text-zinc-900 dark:text-zinc-400 dark:hover:text-zinc-100"
              >
                View all <ArrowRight className="size-3" />
              </button>
            }
          >
            {profile?.name ?? "Active profile"}
          </SectionTitle>
          <div className="surface overflow-hidden rounded-2xl">
            {recent.length === 0 ? (
              <div className="flex flex-col items-center px-6 py-12 text-center">
                <div className="grid size-11 place-items-center rounded-2xl border border-zinc-200/80 bg-zinc-50 text-zinc-400 dark:border-white/[0.06] dark:bg-white/[0.03]">
                  <MousePointerClick className="size-5" strokeWidth={1.6} />
                </div>
                <h3 className="mt-3 text-[13.5px] font-semibold">
                  No mappings yet
                </h3>
                <p className="mt-1 max-w-[300px] text-[12px] leading-relaxed text-secondary">
                  Turn B4, B5 or any mouse button into shortcuts, scroll
                  gestures and chords.
                </p>
                <Button
                  className="mt-4"
                  size="sm"
                  variant="primary"
                  icon={<Sparkles />}
                  onClick={() => setView("mappings")}
                >
                  Create your first mapping
                </Button>
              </div>
            ) : (
              <div className="divide-y divide-zinc-100 dark:divide-white/[0.055]">
                {recent.map((mapping) => (
                  <button
                    key={mapping.id}
                    onClick={() => setView("mappings")}
                    className="group flex w-full items-center gap-3 px-4 py-3 text-left transition hover:bg-zinc-50/90 dark:hover:bg-white/[0.025]"
                  >
                    <MouseButtonBadge
                      button={mapping.button}
                      muted={!mapping.enabled}
                    />
                    <div className="min-w-0 flex-1">
                      <div className="truncate text-[13px] font-medium">
                        {mapping.name || actionLabel(mapping.action)}
                      </div>
                      <div className="mt-0.5 truncate text-[11.5px] text-secondary">
                        {triggerDescription(mapping)}
                      </div>
                    </div>
                    <span className="hidden sm:block">
                      <ActionDisplay action={mapping.action} compact />
                    </span>
                    <Badge tone={mapping.enabled ? "neutral" : "outline"}>
                      {triggerLabel(mapping.trigger)}
                    </Badge>
                    <ArrowRight className="size-3.5 text-zinc-300 transition group-hover:text-zinc-500 dark:text-zinc-700 dark:group-hover:text-zinc-400" />
                  </button>
                ))}
              </div>
            )}
          </div>
        </section>

        <section>
          <SectionTitle
            action={
              listening ? (
                <span className="flex items-center gap-1.5 text-[11px] font-medium text-emerald-600 dark:text-emerald-400">
                  <span className="animate-pulse-dot size-1.5 rounded-full bg-emerald-500" />
                  Listening
                </span>
              ) : undefined
            }
          >
            Test input
          </SectionTitle>
          <div className="surface flex h-[calc(100%-28px)] flex-col rounded-2xl p-4">
            <div className="flex items-center justify-between gap-2">
              <div className="flex items-center gap-2 text-[12.5px] font-medium">
                <Radar className="size-4 text-zinc-400" />
                Mouse identifier
              </div>
              <Button
                size="sm"
                variant={listening ? "subtle" : "secondary"}
                icon={listening ? <Square /> : <Play />}
                disabled={listening && !ready}
                onClick={() => {
                  setLast(null);
                  setListening((value) => !value);
                }}
              >
                {listening ? "Stop" : "Listen"}
              </Button>
            </div>

            <div
              className={cn(
                "mt-3 flex flex-1 flex-col items-center justify-center rounded-xl border border-dashed px-4 py-6 text-center transition-colors",
                listening
                  ? "border-emerald-500/25 bg-emerald-500/[0.035]"
                  : "border-zinc-200/90 bg-zinc-50/50 dark:border-white/[0.08] dark:bg-white/[0.015]",
              )}
            >
              {last?.kind === "mouse" ? (
                <>
                  {last.event === "wheel" ? (
                    <ScrollDirectionGlyph direction={last.direction} />
                  ) : (
                    <MouseButtonBadge
                      button={last.button as MouseButton}
                      className="size-12 rounded-2xl text-[12px]"
                    />
                  )}
                  <div className="mt-3 text-[15px] font-semibold tracking-[-0.01em]">
                    {last.event === "wheel"
                      ? `Scroll ${last.direction}`
                      : `${buttonLabel(last.button as MouseButton)} button`}
                  </div>
                  <div className="mt-0.5 text-[11.5px] text-secondary">
                    {last.event === "wheel"
                      ? "Horizontal / vertical wheel detected"
                      : `${last.event === "down" ? "Pressed" : "Released"} globally · ${last.label}`}
                  </div>
                </>
              ) : (
                <>
                  <MouseGlyph
                    className={cn(
                      "size-10",
                      listening
                        ? "text-emerald-500/60"
                        : "text-zinc-300 dark:text-zinc-700",
                    )}
                  />
                  <div className="mt-2 text-[12.5px] font-medium">
                    {listening ? "Press any mouse button" : "Not listening"}
                  </div>
                  <p className="mt-1 max-w-[230px] text-[11px] leading-relaxed text-secondary">
                    {listening
                      ? "Press a button or scroll. Detection never blocks the input."
                      : "Check which physical button Windows reports, including B4, B5 and horizontal scroll."}
                  </p>
                </>
              )}
            </div>

            <div className="mt-3 flex items-start gap-2 text-[11px] leading-relaxed text-zinc-400 dark:text-zinc-500">
              <Zap className="mt-0.5 size-3 shrink-0" />
              Listening is read-only and auto-stops after 25 seconds.
            </div>
          </div>
        </section>
      </div>

      <section className="mt-6">
        <SectionTitle>Gesture model</SectionTitle>
        <div className="surface grid gap-px overflow-hidden rounded-2xl bg-zinc-100 sm:grid-cols-3 dark:bg-white/[0.055]">
          {[
            {
              step: "01",
              icon: CircleDot,
              title: "Hold a button",
              text: "B4 or B5 becomes a temporary modifier. Nothing fires while it is held.",
            },
            {
              step: "02",
              icon: Mouse,
              title: "Add a gesture",
              text: "Scroll or press another button. The gesture wins and the tap is cancelled.",
            },
            {
              step: "03",
              icon: Check,
              title: "Release cleanly",
              text: "Let go without a gesture and the single-click action runs exactly once.",
            },
          ].map(({ step, icon: Icon, title, text }) => (
            <div key={step} className="bg-white p-4 dark:bg-[#141416]">
              <div className="flex items-center gap-2">
                <span className="grid size-6 place-items-center rounded-md bg-zinc-100 text-[9px] font-semibold text-zinc-500 dark:bg-white/[0.06] dark:text-zinc-400">
                  {step}
                </span>
                <Icon className="size-3.5 text-zinc-400" />
              </div>
              <div className="mt-2.5 text-[12.5px] font-semibold">{title}</div>
              <p className="mt-1 text-[11.5px] leading-relaxed text-secondary">
                {text}
              </p>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}

function ScrollDirectionGlyph({ direction }: { direction: ScrollDirection }) {
  const rotate = {
    up: "rotate-0",
    down: "rotate-180",
    left: "-rotate-90",
    right: "rotate-90",
  }[direction];
  return (
    <div className="grid size-12 place-items-center rounded-2xl border border-zinc-200/80 bg-white text-zinc-500 dark:border-white/10 dark:bg-white/[0.04] dark:text-zinc-300">
      <svg viewBox="0 0 24 24" className={cn("size-6", rotate)} fill="none">
        <rect
          x="8"
          y="2.5"
          width="8"
          height="19"
          rx="4"
          stroke="currentColor"
          strokeWidth="1.5"
        />
        <path
          d="M8 9h8"
          stroke="currentColor"
          strokeWidth="1.5"
          opacity="0.5"
        />
        <path
          d="M12 5.5v3"
          stroke="currentColor"
          strokeWidth="1.8"
          strokeLinecap="round"
        />
      </svg>
    </div>
  );
}

function StatCard({
  icon: Icon,
  label,
  value,
  hint,
  tone,
}: {
  icon: typeof Mouse;
  label: string;
  value: string;
  hint: string;
  tone?: "success" | "danger";
}) {
  return (
    <div className="surface rounded-xl px-3.5 py-3.5">
      <div className="flex items-center gap-2 text-[11px] font-medium text-secondary">
        <Icon className="size-3.5" />
        {label}
        {tone && (
          <span
            className={cn(
              "ml-auto size-1.5 rounded-full",
              tone === "success" ? "bg-emerald-500" : "bg-rose-500",
            )}
          />
        )}
      </div>
      <div className="mt-1.5 text-[18px] font-semibold tracking-[-0.02em]">
        {value}
      </div>
      <div className="mt-0.5 truncate text-[11px] text-zinc-400 dark:text-zinc-500">
        {hint}
      </div>
    </div>
  );
}
