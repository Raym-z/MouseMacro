import { useEffect, useState } from "react";
import { Activity, Play, Square } from "lucide-react";
import { api, errorMessage } from "../lib/api";
import type { InputSample } from "../lib/types";
import { cn } from "../lib/cn";
import { Button } from "./Button";

function StatusChip({
  children,
  tone,
}: {
  children: React.ReactNode;
  tone: "neutral" | "warning" | "success" | "info";
}) {
  return (
    <span
      className={cn(
        "rounded-full px-1.5 py-px text-[10px] font-medium leading-4",
        tone === "neutral" &&
          "bg-zinc-100 text-zinc-500 dark:bg-white/[0.07] dark:text-zinc-400",
        tone === "warning" &&
          "bg-amber-500/10 text-amber-700 dark:text-amber-300",
        tone === "success" &&
          "bg-emerald-500/10 text-emerald-700 dark:text-emerald-300",
        tone === "info" && "bg-indigo-500/10 text-indigo-700 dark:text-indigo-300",
      )}
    >
      {children}
    </span>
  );
}

function describe(sample: InputSample): string {
  if (sample.kind === "wheel") return sample.direction ?? "wheel";
  return sample.button ?? "";
}

/**
 * Live view of the raw mouse events the hook receives, started and stopped on
 * demand. Events are recorded before filtering, so synthetic input is visible
 * together with the reason it was ignored.
 */
export function InputDiagnostics() {
  const [running, setRunning] = useState(false);
  const [samples, setSamples] = useState<InputSample[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!running) return;
    let cancelled = false;
    const poll = async () => {
      try {
        const next = await api.getInputDiagnostics();
        if (!cancelled) {
          setSamples(next);
          setError(null);
        }
      } catch (caught) {
        if (!cancelled) setError(errorMessage(caught));
      }
    };
    void poll();
    const timer = window.setInterval(poll, 700);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [running]);

  const start = async () => {
    setError(null);
    setSamples([]);
    try {
      await api.clearInputDiagnostics();
    } catch (caught) {
      setError(errorMessage(caught));
      return;
    }
    setRunning(true);
  };

  const visible = samples.slice(0, 10);

  return (
    <div>
      <div className="flex items-center gap-2 text-[12px] font-medium">
        <Activity className="size-3.5 text-zinc-400" />
        Recent mouse input
        {running ? (
          <span className="flex items-center gap-1 text-[10.5px] font-normal text-emerald-600 dark:text-emerald-400">
            <span className="animate-pulse-dot size-1.5 rounded-full bg-emerald-500" />
            recording
          </span>
        ) : (
          <span className="text-[10.5px] font-normal text-zinc-400 dark:text-zinc-500">
            stopped
          </span>
        )}
        <span className="ml-auto flex items-center gap-2">
          <span className="text-[10.5px] font-normal text-zinc-400 dark:text-zinc-500">
            newest first
          </span>
          <Button
            size="sm"
            variant={running ? "secondary" : "subtle"}
            icon={running ? <Square /> : <Play />}
            onClick={() => {
              if (running) {
                setRunning(false);
              } else {
                void start();
              }
            }}
          >
            {running ? "Stop" : "Start"}
          </Button>
        </span>
      </div>

      <div className="mt-2 space-y-1">
        {error ? (
          <p className="rounded-lg border border-dashed border-zinc-200 px-3 py-2 text-[11.5px] text-rose-600 dark:border-white/[0.08] dark:text-rose-400">
            {error}
          </p>
        ) : visible.length === 0 ? (
          <p className="rounded-lg border border-dashed border-zinc-200 px-3 py-2 text-[11.5px] text-zinc-400 dark:border-white/[0.08] dark:text-zinc-500">
            {running
              ? "Recording — click or scroll to capture events."
              : "Not recording. Press Start, then click or scroll to test."}
          </p>
        ) : (
          visible.map((sample, index) => (
            <div
              key={`${sample.ageMs}-${index}`}
              className="flex items-center gap-2 rounded-lg border border-zinc-200/80 bg-white px-2 py-1 text-[11px] dark:border-white/[0.06] dark:bg-white/[0.025]"
            >
              <span className="w-9 shrink-0 text-right font-mono text-[10.5px] text-zinc-400 dark:text-zinc-500">
                {(sample.ageMs / 1000).toFixed(1)}s
              </span>
              <span className="w-11 shrink-0 font-medium capitalize text-zinc-600 dark:text-zinc-300">
                {sample.kind}
              </span>
              <span className="w-14 shrink-0 capitalize text-zinc-700 dark:text-zinc-200">
                {describe(sample)}
              </span>
              {sample.axis ? (
                <span className="w-[64px] shrink-0 text-zinc-400 dark:text-zinc-500">
                  {sample.axis}
                </span>
              ) : (
                <span className="w-[64px] shrink-0" />
              )}
              <span className="w-11 shrink-0 font-mono text-zinc-400 dark:text-zinc-500">
                {sample.kind === "wheel"
                  ? `${sample.delta > 0 ? "+" : ""}${sample.delta}`
                  : ""}
              </span>
              <span className="ml-auto flex shrink-0 items-center gap-1">
                {sample.ownInjection ? (
                  <StatusChip tone="neutral">own</StatusChip>
                ) : sample.injected ? (
                  <StatusChip tone="warning">synthetic</StatusChip>
                ) : null}
                {!sample.accepted && <StatusChip tone="warning">ignored</StatusChip>}
                {sample.matched && <StatusChip tone="success">matched</StatusChip>}
              </span>
            </div>
          ))
        )}
      </div>

      <p className="mt-2 text-[11px] leading-relaxed text-zinc-400 dark:text-zinc-500">
        Recording starts empty and keeps the most recent events even after you
        press Stop. The axis is what the device actually sends: a normal wheel
        reports <span className="font-medium">vertical</span> (up/down), while a
        thumb-wheel or tilt wheel reports{" "}
        <span className="font-medium">horizontal</span> (left/right) — that is
        expected. Events marked <em>synthetic</em> were generated by another
        program and are filtered unless the setting above is enabled.
      </p>
    </div>
  );
}
