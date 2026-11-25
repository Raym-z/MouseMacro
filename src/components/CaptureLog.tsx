import { useEffect, useState } from "react";
import { Badge } from "./Badge";
import { cn } from "../lib/cn";
import { buttonLabel, scrollLabel } from "../lib/format";
import { keyTokenLabel } from "../lib/keys";
import type { CaptureEvent } from "../lib/types";

function eventLabel(event: CaptureEvent): string {
  if (event.source === "mouse") {
    if (event.kind === "wheel") {
      return `Wheel ${event.direction ? scrollLabel(event.direction).toLowerCase() : ""}`.trim();
    }
    const button = buttonLabel(event.button);
    return `${button} ${event.kind}`;
  }
  const key = event.token ? keyTokenLabel(event.token) : "key";
  const parts = [...event.modifiers.map((modifier) => keyTokenLabel(modifier)), key];
  return `${parts.join(" + ")} ${event.kind === "down" ? "down" : "up"}${
    event.repeat ? " · repeat" : ""
  }`;
}

function decisionBadge(event: CaptureEvent) {
  switch (event.decision) {
    case "selected":
      return <Badge tone="success">Selected</Badge>;
    case "filtered":
      return <Badge tone="warning">Filtered</Badge>;
    case "own":
      return <Badge tone="outline">Own input</Badge>;
    case "seen":
      return null;
  }
}

/**
 * Live feed of what the hooks actually saw during a capture session. It makes
 * "the button did nothing" visible: filtered, own and unmatched input are all
 * shown instead of silently disappearing.
 */
export function CaptureLog({
  events,
  ready,
  active = true,
  className,
}: {
  events: CaptureEvent[];
  ready: boolean;
  active?: boolean;
  className?: string;
}) {
  const [waited, setWaited] = useState(false);

  useEffect(() => {
    if (!ready || events.length > 0) {
      setWaited(false);
      return;
    }
    const timer = window.setTimeout(() => setWaited(true), 2_000);
    return () => window.clearTimeout(timer);
  }, [ready, events.length]);

  const visible = events.slice(-8);

  return (
    <div
      className={cn(
        "rounded-xl border border-zinc-200 bg-zinc-50/70 px-3 py-2.5 dark:border-white/[0.08] dark:bg-white/[0.02]",
        className,
      )}
      aria-live="polite"
    >
      <div className="flex items-center justify-between gap-2">
        <span className="text-[11.5px] font-medium">
          {active
            ? ready
              ? "Listening — press a button or key"
              : "Starting…"
            : "Last input"}
        </span>
        <span className="text-[10.5px] text-zinc-400 dark:text-zinc-500">
          {events.length > 0 ? `${events.length} seen` : active ? "Esc to stop" : ""}
        </span>
      </div>

      {visible.length === 0 ? (
        <p className="mt-1.5 text-[11px] leading-relaxed text-zinc-400 dark:text-zinc-500">
          {!active
            ? "No input was recorded."
            : waited
              ? "No input seen yet. Vendor mouse software can hide buttons — enable “Process input from other input tools” in Settings to see them."
              : "Input appears here as soon as the hook sees it."}
        </p>
      ) : (
        <ul className="mt-1.5 space-y-1">
          {visible.map((event) => (
            <li
              key={event.seq}
              className={cn(
                "flex items-center justify-between gap-2 rounded-lg px-2 py-1 text-[11.5px]",
                event.decision === "selected" &&
                  "bg-emerald-500/10 text-emerald-700 dark:text-emerald-400",
                event.decision === "filtered" && "opacity-70",
                event.decision === "own" && "opacity-60",
              )}
            >
              <span className="truncate font-mono">{eventLabel(event)}</span>
              {decisionBadge(event)}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
