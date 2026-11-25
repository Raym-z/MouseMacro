import { useState } from "react";
import {
  ArrowRight,
  Check,
  Layers,
  MousePointerClick,
  ScanLine,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { Modal } from "./Modal";
import { Button } from "./Button";
import { Switch } from "./Switch";
import { Select } from "./Select";
import { Badge } from "./Badge";
import { Callout } from "./Callout";
import { MouseButtonBadge } from "./MouseButtonBadge";
import { useCapture } from "../hooks/useCapture";
import { useApp } from "../state/AppContext";
import { api } from "../lib/api";
import { MOUSE_BUTTONS, type MouseButton } from "../lib/types";
import { buttonLabel } from "../lib/format";
import { cn } from "../lib/cn";

/** First-run setup, shown until `settings.onboarded` is true. */
export function Onboarding() {
  const { snapshot, runSnapshot, refresh } = useApp();
  const [step, setStep] = useState(0);
  const [detecting, setDetecting] = useState(false);
  const [detected, setDetected] = useState<Set<MouseButton>>(new Set());
  const [profileId, setProfileId] = useState(snapshot?.config.activeProfileId ?? "");
  const [enable, setEnable] = useState(true);
  const [launch, setLaunch] = useState(
    snapshot?.config.settings.launchAtStartup ?? false,
  );
  const [minimized, setMinimized] = useState(
    snapshot?.config.settings.startMinimized ?? false,
  );
  const [finishing, setFinishing] = useState(false);

  useCapture(
    detecting ? "monitor" : "none",
    (payload) => {
      if (payload.kind === "mouse" && payload.event === "down") {
        setDetected((current) => new Set(current).add(payload.button));
      }
    },
    () => setDetecting(false),
    () => setDetecting(false),
  );

  if (!snapshot) return null;
  const settings = snapshot.config.settings;
  if (settings.onboarded) return null;

  const finish = async () => {
    setFinishing(true);
    setDetecting(false);
    const result = await runSnapshot(async () => {
      if (profileId && profileId !== snapshot.config.activeProfileId) {
        await api.setActiveProfile(profileId);
      }
      return api.updateSettings({
        ...settings,
        enabled: enable,
        launchAtStartup: launch,
        startMinimized: minimized,
        onboarded: true,
      });
    }, "Setup complete — MouseMacro is ready");
    setFinishing(false);
    if (result) await refresh();
  };

  const skip = async () => {
    setFinishing(true);
    setDetecting(false);
    await runSnapshot(
      () =>
        api.updateSettings({
          ...settings,
          onboarded: true,
        }),
      "Setup skipped — you can change everything later in Settings",
    );
    setFinishing(false);
  };

  const detectedList = MOUSE_BUTTONS.filter((button) => detected.has(button));

  return (
    <Modal
      open
      onClose={() => void skip()}
      title="Welcome to MouseMacro"
      description="Three short steps: see what is possible, identify your buttons, then turn mapping on."
      size="lg"
      headerExtra={
        <div className="flex items-center gap-1.5 pr-1">
          {[0, 1, 2].map((index) => (
            <span
              key={index}
              className={cn(
                "h-1.5 rounded-full transition-all",
                index === step
                  ? "w-5 bg-zinc-900 dark:bg-white"
                  : "w-1.5 bg-zinc-200 dark:bg-white/15",
              )}
            />
          ))}
        </div>
      }
      footer={
        <>
          <Button
            variant="ghost"
            onClick={skip}
            disabled={finishing}
            className="mr-auto"
          >
            Skip setup
          </Button>
          {step > 0 && (
            <Button
              variant="ghost"
              onClick={() => setStep((current) => current - 1)}
              disabled={finishing}
            >
              Back
            </Button>
          )}
          {step < 2 ? (
            <Button
              variant="primary"
              onClick={() => setStep((current) => current + 1)}
              icon={<ArrowRight />}
            >
              Continue
            </Button>
          ) : (
            <Button
              variant="primary"
              onClick={finish}
              loading={finishing}
              icon={<Check />}
            >
              Finish setup
            </Button>
          )}
        </>
      }
    >
      {step === 0 && (
        <div className="space-y-4">
          <div className="flex items-start gap-3 rounded-xl border border-indigo-500/15 bg-indigo-500/[0.06] p-3.5">
            <Sparkles className="mt-0.5 size-4 shrink-0 text-indigo-500" />
            <div className="text-[12.5px] leading-relaxed text-indigo-900 dark:text-indigo-200">
              <div className="font-semibold">A second set of mouse buttons</div>
              <div className="mt-0.5 opacity-85">
                Remap side buttons, turn holds into gestures, and keep the
                normal click when nothing matches.
              </div>
            </div>
          </div>
          <div className="grid gap-2 sm:grid-cols-2">
            {[
              {
                icon: MousePointerClick,
                title: "Buttons",
                text: "Single, double, long press, release and chords.",
              },
              {
                icon: Layers,
                title: "Layers",
                text: "Hold a button to temporarily change the others.",
              },
              {
                icon: ScanLine,
                title: "Gestures",
                text: "Hold a button and scroll, or press a second button.",
              },
              {
                icon: ShieldCheck,
                title: "Safe by default",
                text: "Unmatched input is replayed, never silently lost.",
              },
            ].map((item) => (
              <div
                key={item.title}
                className="rounded-xl border border-zinc-200/80 p-3 dark:border-white/[0.07]"
              >
                <item.icon className="size-4 text-zinc-400" />
                <div className="mt-2 text-[12.5px] font-medium">
                  {item.title}
                </div>
                <p className="mt-0.5 text-[11.5px] leading-relaxed text-secondary">
                  {item.text}
                </p>
              </div>
            ))}
          </div>
          <p className="text-[11px] leading-relaxed text-zinc-400 dark:text-zinc-500">
            Everything stays on this PC. Configuration lives in a readable JSON
            file, and the default emergency stop is{" "}
            <span className="font-medium">Ctrl + Alt + Shift + F12</span>.
          </p>
        </div>
      )}

      {step === 1 && (
        <div className="space-y-4">
          <p className="text-[12.5px] leading-relaxed text-secondary">
            Press side buttons to see which ones your mouse exposes. This step is
            optional — it never changes a mapping by itself.
          </p>
          <div className="rounded-xl border border-zinc-200 bg-zinc-50/60 p-3.5 dark:border-white/[0.08] dark:bg-white/[0.02]">
            <div className="flex items-center gap-2">
              <Button
                size="sm"
                variant={detecting ? "subtle" : "secondary"}
                icon={<ScanLine />}
                onClick={() => setDetecting((value) => !value)}
              >
                {detecting ? "Stop detecting" : "Detect buttons"}
              </Button>
              {detecting && (
                <span className="flex items-center gap-1.5 text-[11.5px] text-emerald-600 dark:text-emerald-400">
                  <span className="animate-pulse-dot size-1.5 rounded-full bg-emerald-500" />
                  listening — press any mouse button
                </span>
              )}
            </div>
            <div className="mt-3 flex flex-wrap gap-1.5">
              {detectedList.length === 0 ? (
                <span className="text-[11.5px] text-zinc-400 dark:text-zinc-500">
                  No buttons detected yet.
                </span>
              ) : (
                detectedList.map((button) => (
                  <span
                    key={button}
                    className="inline-flex items-center gap-1.5 rounded-lg border border-zinc-200 bg-white px-2 py-1 text-[11.5px] font-medium dark:border-white/10 dark:bg-white/[0.04]"
                  >
                    <MouseButtonBadge button={button} />
                    {buttonLabel(button)}
                  </span>
                ))
              )}
            </div>
          </div>
          <Callout tone="info">
            Most mice expose <strong>B4</strong> and <strong>B5</strong> on the
            side. If your buttons do not appear, the vendor software may be
            synthesizing them — see Settings → Input diagnostics later.
          </Callout>
        </div>
      )}

      {step === 2 && (
        <div className="space-y-4">
          <div>
            <div className="mb-1.5 text-[12.5px] font-medium">Start profile</div>
            <Select
              value={profileId}
              onChange={(event) => setProfileId(event.target.value)}
            >
              {snapshot.config.profiles.map((profile) => (
                <option key={profile.id} value={profile.id}>
                  {profile.name}
                </option>
              ))}
            </Select>
            <p className="mt-1 text-[11.5px] leading-relaxed text-zinc-400 dark:text-zinc-500">
              You can switch profiles at any time from the sidebar or the tray.
            </p>
          </div>

          <div className="divide-y divide-zinc-100 rounded-xl border border-zinc-200/80 dark:divide-white/[0.055] dark:border-white/[0.07]">
            <ToggleRow
              title="Enable mappings now"
              description="Until this is on, every click passes through untouched."
              checked={enable}
              onChange={setEnable}
            />
            <ToggleRow
              title="Launch on Windows startup"
              description="Start with Windows so mappings are ready after sign-in."
              checked={launch}
              onChange={setLaunch}
            />
            <ToggleRow
              title="Start minimized to the tray"
              description="Open quietly in the tray instead of showing a window."
              checked={minimized}
              onChange={setMinimized}
            />
          </div>

          <div className="flex items-center gap-2 text-[11px] text-zinc-400 dark:text-zinc-500">
            <Badge tone="outline">open source</Badge>
            <span>No account, telemetry or network calls.</span>
          </div>
          <p className="text-[11px] leading-relaxed text-zinc-400 dark:text-zinc-500">
            Games and tools that run as administrator may need MouseMacro to run
            elevated too. That option is in Settings → Run as administrator.
          </p>
        </div>
      )}
    </Modal>
  );
}

function ToggleRow({
  title,
  description,
  checked,
  onChange,
}: {
  title: string;
  description: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <div className="flex items-center justify-between gap-4 px-3 py-2.5">
      <div>
        <div className="text-[12.5px] font-medium">{title}</div>
        <div className="text-[11px] text-secondary">{description}</div>
      </div>
      <Switch checked={checked} onChange={onChange} label={title} />
    </div>
  );
}
