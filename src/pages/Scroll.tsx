import { useEffect, useState } from "react";
import { ArrowLeftRight, FlipHorizontal2, Gauge, MousePointer2, ShieldCheck } from "lucide-react";
import { useApp } from "../state/AppContext";
import { PageHeader } from "../components/PageHeader";
import { Badge } from "../components/Badge";
import { Button } from "../components/Button";
import { Callout } from "../components/Callout";
import { Field, Segmented } from "../components/Field";
import { Switch } from "../components/Switch";
import { api } from "../lib/api";
import type { ScrollSettings } from "../lib/types";

export function ScrollPage() {
  const { snapshot, runSnapshot, setView } = useApp();
  const [wheelLines, setWheelLines] = useState<number | null>(null);

  useEffect(() => {
    api
      .getWheelLines()
      .then(setWheelLines)
      .catch(() => setWheelLines(null));
  }, []);

  if (!snapshot) return null;
  const { config, status } = snapshot;
  const scroll = config.settings.scroll;
  const engineOn = config.settings.enabled && !status.panicActive;

  const patch = (next: Partial<ScrollSettings>, message?: string) =>
    runSnapshot(
      () =>
        api.updateSettings({
          ...config.settings,
          scroll: { ...scroll, ...next },
        }),
      message,
    );

  return (
    <div className="animate-in">
      <PageHeader
        eyebrow="Input"
        title="Scroll"
        description="Correct wheel direction and speed for the devices and applications you use. Changes apply to physical wheel events only."
        actions={
          <div className="flex items-center gap-2">
            <Badge tone={engineOn ? "success" : "warning"} dot>
              {engineOn ? "Active" : "Mappings paused"}
            </Badge>
            <Badge tone="outline">
              Windows: {wheelLines === null ? "—" : `${wheelLines} lines/notch`}
            </Badge>
          </div>
        }
      />

      <div className="grid gap-4 lg:grid-cols-2">
        <section className="surface rounded-2xl p-4">
          <div className="flex items-start gap-3">
            <div className="grid size-8 shrink-0 place-items-center rounded-lg border border-indigo-500/20 bg-indigo-500/[0.07] text-indigo-500">
              <FlipHorizontal2 className="size-4" />
            </div>
            <div>
              <h2 className="text-[13px] font-semibold">Direction</h2>
              <p className="text-[11.5px] leading-relaxed text-secondary">
                Flip the axes your device reports incorrectly.
              </p>
            </div>
          </div>

          <div className="mt-4 space-y-2">
            <ToggleRow
              label="Invert vertical scrolling"
              hint="Wheel up becomes down, and the other way around."
              checked={scroll.invertVertical}
              onChange={(invertVertical) =>
                void patch({ invertVertical }, "Vertical scroll inverted")
              }
            />
            <ToggleRow
              label="Invert horizontal scrolling"
              hint="For tilt wheels and horizontal touchpad gestures."
              checked={scroll.invertHorizontal}
              onChange={(invertHorizontal) =>
                void patch({ invertHorizontal }, "Horizontal scroll inverted")
              }
            />
            <ToggleRow
              label="Swap axes"
              hint="A vertical wheel scrolls horizontally and vice versa. Useful when a driver reports the wrong axis."
              checked={scroll.swapAxes}
              onChange={(swapAxes) =>
                void patch({ swapAxes }, "Wheel axes swapped")
              }
            />
          </div>
        </section>

        <section className="surface rounded-2xl p-4">
          <div className="flex items-start gap-3">
            <div className="grid size-8 shrink-0 place-items-center rounded-lg border border-emerald-500/20 bg-emerald-500/[0.07] text-emerald-500">
              <Gauge className="size-4" />
            </div>
            <div>
              <h2 className="text-[13px] font-semibold">Speed</h2>
              <p className="text-[11.5px] leading-relaxed text-secondary">
                Send more notches per physical notch.
              </p>
            </div>
          </div>

          <div className="mt-4">
            <Field
              label="Notches per physical notch"
              hint="Windows applications scroll in whole notches, so this can speed scrolling up but not make it sub-notch smooth."
            >
              <Segmented
                value={String(scroll.speed)}
                onChange={(value) =>
                  void patch({ speed: Number(value) }, `Scroll speed ×${value}`)
                }
                options={[1, 2, 3, 4, 5].map((speed) => ({
                  value: String(speed),
                  label: `×${speed}`,
                }))}
              />
            </Field>
          </div>

          <div className="mt-4">
            <ToggleRow
              label="Time-based acceleration"
              hint="Experimental: add extra notches when the wheel is turned quickly."
              checked={scroll.acceleration}
              onChange={(acceleration) =>
                void patch(
                  { acceleration },
                  acceleration
                    ? "Scroll acceleration enabled"
                    : "Scroll acceleration disabled",
                )
              }
            />
          </div>
        </section>
      </div>

      <section className="surface mt-4 rounded-2xl p-4">
        <div className="flex items-start gap-3">
          <div className="grid size-8 shrink-0 place-items-center rounded-lg border border-zinc-200 bg-zinc-50 text-zinc-500 dark:border-white/10 dark:bg-white/[0.03] dark:text-zinc-400">
            <MousePointer2 className="size-4" />
          </div>
          <div className="min-w-0 flex-1">
            <h2 className="text-[13px] font-semibold">Per-application overrides</h2>
            <p className="mt-0.5 text-[11.5px] leading-relaxed text-secondary">
              Keep inverted scrolling on the desktop while a game keeps normal
              scrolling. Open an application rule and set its wheel behaviour.
            </p>
            <div className="mt-3 flex flex-wrap items-center gap-2">
              <Button
                size="sm"
                variant="secondary"
                icon={<ArrowLeftRight />}
                onClick={() => setView("exceptions")}
              >
                Open Exceptions
              </Button>
              <Badge tone="outline">
                {config.exceptions.filter((rule) => rule.scroll).length} app
                {config.exceptions.filter((rule) => rule.scroll).length === 1
                  ? ""
                  : "s"}{" "}
                with an override
              </Badge>
            </div>
          </div>
        </div>
      </section>

      <Callout tone="info" className="mt-4">
        <span className="flex items-start gap-2">
          <ShieldCheck className="mt-0.5 size-3.5 shrink-0" />
          <span>
            MouseMacro only transforms wheel events it sees from the physical
            device. Its own injected scroll is tagged and ignored, so a scroll
            action never loops back through these settings. When mappings are
            paused or an app rule turns them off without a scroll override,
            wheels pass through untouched.
          </span>
        </span>
      </Callout>
    </div>
  );
}

function ToggleRow({
  label,
  hint,
  checked,
  onChange,
}: {
  label: string;
  hint: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <div className="flex items-center justify-between gap-3 rounded-xl border border-zinc-200/80 px-3 py-2.5 dark:border-white/[0.07]">
      <div className="min-w-0">
        <div className="text-[12.5px] font-medium">{label}</div>
        <div className="text-[11px] leading-relaxed text-secondary">{hint}</div>
      </div>
      <Switch checked={checked} onChange={onChange} label={label} />
    </div>
  );
}
