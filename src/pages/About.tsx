import { useState } from "react";
import {
  Activity,
  Check,
  Cpu,
  Fingerprint,
  Gauge,
  Keyboard,
  Lock,
  Mouse,
  MousePointerClick,
  ScrollText,
  ShieldAlert,
  ShieldCheck,
  Sparkles,
  Waypoints,
} from "lucide-react";
import { useApp } from "../state/AppContext";
import { PageHeader } from "../components/PageHeader";
import { LogoMark } from "../components/Logo";
import { Badge } from "../components/Badge";
import { Button } from "../components/Button";
import { api, errorMessage } from "../lib/api";
import { useToast } from "../components/Toast";

export function AboutPage() {
  const { snapshot, run } = useApp();
  const { toast } = useToast();
  const [restarting, setRestarting] = useState(false);

  if (!snapshot) return null;
  const { status } = snapshot;

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

  const features = [
    {
      icon: Mouse,
      title: "Five-button remapping",
      text: "Left, right, middle, B4 and B5, including wheel gestures.",
    },
    {
      icon: Keyboard,
      title: "Recorded shortcuts",
      text: "Capture key combinations instead of typing key names.",
    },
    {
      icon: Waypoints,
      title: "Modifier gestures",
      text: "Hold a button and scroll for zoom, or press a chord.",
    },
    {
      icon: Fingerprint,
      title: "Local by design",
      text: "Profiles live in a readable JSON file on this PC.",
    },
  ];

  return (
    <div className="animate-in">
      <PageHeader eyebrow="Information" title="About MouseMacro" />

      <section className="surface overflow-hidden rounded-2xl">
        <div className="relative px-6 py-7">
          <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(90%_150%_at_85%_-30%,rgba(99,102,241,0.09),transparent_60%)]" />
          <div className="relative flex flex-wrap items-start justify-between gap-6">
            <div className="flex items-start gap-4">
              <LogoMark className="size-14 rounded-[18px] [&_svg]:size-8" />
              <div>
                <div className="flex flex-wrap items-center gap-2">
                  <h2 className="text-[24px] font-semibold tracking-[-0.04em]">
                    MouseMacro
                  </h2>
                  <Badge tone="accent">v{status.version}</Badge>
                  {status.elevated && (
                    <Badge tone="success" dot>
                      Administrator
                    </Badge>
                  )}
                </div>
                <p className="mt-1.5 max-w-[560px] text-[13px] leading-relaxed text-secondary">
                  A native Windows input studio for remapping mouse buttons,
                  recording shortcuts and building hold-to-scroll gestures —
                  without manufacturer software.
                </p>
                <div className="mt-3 flex flex-wrap gap-1.5">
                  {["Tauri v2", "Rust", "React", "Windows 10 / 11"].map((tag) => (
                    <Badge key={tag} tone="outline">
                      {tag}
                    </Badge>
                  ))}
                </div>
              </div>
            </div>
            <div className="grid grid-cols-2 gap-2 text-[11px]">
              <StatusTile
                label="Engine"
                value={snapshot.config.settings.enabled ? "Active" : "Paused"}
                good={snapshot.config.settings.enabled}
              />
              <StatusTile
                label="Mouse hook"
                value={status.hooksActive ? "Installed" : "Offline"}
                good={status.hooksActive}
              />
              <StatusTile
                label="Elevation"
                value={status.elevated ? "Elevated" : "Standard"}
                good={status.elevated}
                neutral={!status.elevated}
              />
              <StatusTile
                label="Profile"
                value={
                  snapshot.config.profiles.find(
                    (p) => p.id === snapshot.config.activeProfileId,
                  )?.name ?? "—"
                }
                good
              />
            </div>
          </div>
        </div>
      </section>

      <div className="mt-4 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        {features.map(({ icon: Icon, title, text }) => (
          <div key={title} className="surface rounded-xl p-3.5">
            <div className="flex size-8 items-center justify-center rounded-lg bg-zinc-100 text-zinc-500 dark:bg-white/[0.06] dark:text-zinc-400">
              <Icon className="size-4" strokeWidth={1.7} />
            </div>
            <div className="mt-2.5 text-[12.5px] font-semibold">{title}</div>
            <p className="mt-1 text-[11.5px] leading-relaxed text-secondary">
              {text}
            </p>
          </div>
        ))}
      </div>

      <div className="mt-4 grid gap-4 lg:grid-cols-2">
        <section className="surface rounded-2xl p-5">
          <h3 className="flex items-center gap-2 text-[13px] font-semibold">
            <Cpu className="size-4 text-zinc-400" />
            How the input engine works
          </h3>
          <p className="mt-2 text-[12px] leading-relaxed text-secondary">
            A dedicated thread installs a Windows low-level mouse hook and keeps
            a native message loop alive. Every physical event is classified into
            an abstract input event and handed to a pure Rust state machine. That
            engine decides whether the event is consumed and which actions to
            emit. Actions are played back with SendInput and tagged, so the hook
            never re-processes its own synthetic input.
          </p>
          <div className="mt-3 space-y-1.5">
            {[
              "Gestures and chords resolve before long presses and clicks.",
              "Deferred clicks replay the original event if no rule matches.",
              "Sustained key and mouse holds are released on button up, config change and shutdown.",
              "Rule timing is driven by a condition variable — no polling loop.",
            ].map((line) => (
              <div key={line} className="flex items-start gap-2 text-[11.5px] leading-relaxed text-secondary">
                <Check className="mt-0.5 size-3 shrink-0 text-emerald-500" />
                {line}
              </div>
            ))}
          </div>
        </section>

        <section className="surface rounded-2xl p-5">
          <h3 className="flex items-center gap-2 text-[13px] font-semibold">
            <ShieldAlert className="size-4 text-zinc-400" />
            Privileges & limitations
          </h3>
          <div className="mt-2.5 space-y-2.5 text-[11.5px] leading-relaxed text-secondary">
            <p>
              Standard Windows security prevents a normal process from sending
              input to elevated applications. If your shortcuts do nothing in an
              admin app, restart MouseMacro with{" "}
              <span className="font-medium text-zinc-700 dark:text-zinc-300">
                Restart elevated
              </span>
              .
            </p>
            <p>
              Some full-screen games and anti-cheat systems may block synthetic
              input. Media keys and Win-key combinations can also be reserved by
              Windows. These are operating system limits, not silently ignored
              mappings.
            </p>
            <p>
              MouseMacro never sends data anywhere. There is no telemetry, cloud
              sync or network listener.
            </p>
          </div>
          {!status.elevated ? (
            <Button
              className="mt-3"
              size="sm"
              variant="secondary"
              icon={<ShieldAlert />}
              loading={restarting}
              onClick={restartElevated}
            >
              Restart as administrator
            </Button>
          ) : (
            <Badge tone="success" className="mt-3">
              <ShieldCheck className="size-3" /> Running elevated
            </Badge>
          )}
        </section>
      </div>

      <div className="mt-4 grid gap-4 lg:grid-cols-2">
        <section className="surface rounded-2xl p-5">
          <h3 className="flex items-center gap-2 text-[13px] font-semibold">
            <ScrollText className="size-4 text-zinc-400" />
            Configuration
          </h3>
          <div className="mt-3 flex items-center gap-2 rounded-lg border border-zinc-200/80 bg-zinc-50 px-3 py-2 dark:border-white/[0.07] dark:bg-white/[0.025]">
            <code className="min-w-0 flex-1 truncate font-mono text-[11px] text-zinc-500 dark:text-zinc-400">
              {status.configPath}
            </code>
            <Button
              size="sm"
              variant="ghost"
              icon={<Gauge />}
              onClick={() => run(() => api.openConfigFolder())}
            >
              Open
            </Button>
          </div>
          <p className="mt-2 text-[11px] leading-relaxed text-zinc-400 dark:text-zinc-500">
            Profiles can be exported from the Profiles page and imported on
            another PC. Settings are stored separately from profiles so mapping
            files stay portable.
          </p>
          <div className="mt-3 flex flex-wrap items-center gap-2">
            <Badge tone="outline">{status.platform}</Badge>
            <Badge tone="outline">Schema v{snapshot.config.schemaVersion}</Badge>
            <Badge tone="outline">{snapshot.config.profiles.length} profiles</Badge>
          </div>
        </section>

        <section className="surface rounded-2xl p-5">
          <h3 className="flex items-center gap-2 text-[13px] font-semibold">
            <Lock className="size-4 text-zinc-400" />
            Safety behaviour
          </h3>
          <div className="mt-3 space-y-2">
            {[
              ["Disabled engine", "Physical input is never swallowed."],
              ["No matching rule", "The original click is replayed."],
              ["App shutdown", "All synthetic holds are released."],
              ["Interrupted gesture", "State resets on button up or capture mode."],
            ].map(([title, text]) => (
              <div key={title} className="flex items-baseline justify-between gap-3 text-[11.5px]">
                <span className="font-medium">{title}</span>
                <span className="text-right text-secondary">{text}</span>
              </div>
            ))}
          </div>
          <div className="mt-3 flex items-center gap-2 rounded-lg bg-emerald-500/[0.06] px-2.5 py-2 text-[11px] text-emerald-700 dark:text-emerald-300">
            <Activity className="size-3" />
            Hook status: {status.hooksActive ? "receiving input" : "not running"}
          </div>
        </section>
      </div>

      <div className="mt-5 flex flex-wrap items-center justify-between gap-3 text-[11px] text-zinc-400 dark:text-zinc-500">
        <div className="flex items-center gap-1.5">
          <Sparkles className="size-3" />
          MouseMacro {status.version} · Built with Tauri, Rust and React
        </div>
        <div className="flex items-center gap-1.5">
          <MousePointerClick className="size-3" />
          Windows 10 and 11 · x64
        </div>
      </div>
    </div>
  );
}

function StatusTile({
  label,
  value,
  good,
  neutral,
}: {
  label: string;
  value: string;
  good?: boolean;
  neutral?: boolean;
}) {
  return (
    <div className="min-w-[104px] rounded-lg border border-zinc-200/80 bg-white/60 px-2.5 py-2 dark:border-white/[0.07] dark:bg-white/[0.03]">
      <div className="text-[9.5px] font-medium uppercase tracking-[0.12em] text-zinc-400 dark:text-zinc-500">
        {label}
      </div>
      <div className="mt-0.5 flex items-center gap-1.5 text-[11.5px] font-medium">
        <span
          className={
            neutral
              ? "size-1.5 rounded-full bg-zinc-400"
              : good
                ? "size-1.5 rounded-full bg-emerald-500"
                : "size-1.5 rounded-full bg-zinc-300 dark:bg-zinc-600"
          }
        />
        {value}
      </div>
    </div>
  );
}
