import { useEffect, useState } from "react";
import { Badge } from "./Badge";
import { cn } from "../lib/cn";
import { buttonLabel, scrollLabel } from "../lib/format";
import { isModifierToken, keyTokenLabel } from "../lib/keys";
import type { CaptureDecision, CaptureEvent } from "../lib/types";

export interface CaptureLogEntry {
  id: string;
  /** The key-down (or wheel) event this entry represents. */
  event: CaptureEvent;
  label: string;
  decision: CaptureDecision;
  /** The matching release has been seen. */
  complete: boolean;
  repeat: boolean;
  selectable: boolean;
}

function comboLabel(event: CaptureEvent): string {
  const key = keyTokenLabel(event.token ?? "");
  return [...event.modifiers.map((modifier) => keyTokenLabel(modifier)), key].join(
    " + ",
  );
}

/**
 * Collapses the raw event stream into one row per gesture:
 * a mouse down/up pair becomes “Left click”, a keyboard press becomes
 * “Ctrl + Shift + S” using the modifiers held at that moment.
 */
export function captureLogEntries(events: CaptureEvent[]): CaptureLogEntry[] {
  const entries: CaptureLogEntry[] = [];
  const open = new Map<string, CaptureLogEntry>();

  for (const event of events) {
    const usable =
      event.decision !== "filtered" && event.decision !== "own";

    if (event.source === "mouse") {
      if (event.kind === "wheel") {
        entries.push({
          id: `w${event.seq}`,
          event,
          label: `Wheel ${event.direction ? scrollLabel(event.direction).toLowerCase() : ""}`.trim(),
          decision: event.decision,
          complete: true,
          repeat: false,
          selectable: false,
        });
        continue;
      }
      const slot = `m:${event.button}`;
      if (event.kind === "down") {
        if (event.repeat) continue;
        const entry: CaptureLogEntry = {
          id: `m${event.seq}`,
          event,
          label: `${buttonLabel(event.button)} click`,
          decision: event.decision,
          complete: false,
          repeat: false,
          selectable: usable,
        };
        entries.push(entry);
        open.set(slot, entry);
      } else {
        const entry = open.get(slot);
        if (entry) {
          entry.complete = true;
          open.delete(slot);
        } else {
          entries.push({
            id: `m${event.seq}`,
            event,
            label: `${buttonLabel(event.button)} release`,
            decision: event.decision,
            complete: true,
            repeat: false,
            selectable: false,
          });
        }
      }
      continue;
    }

    // Keyboard: modifier-only rows are noise; combinations are attached to
    // the non-modifier key that completes them.
    if (event.token && isModifierToken(event.token)) continue;

    if (event.kind === "down") {
      if (event.repeat) {
        const last = entries[entries.length - 1];
        if (last) last.repeat = true;
        continue;
      }
      const entry: CaptureLogEntry = {
        id: `k${event.seq}`,
        event,
        label: comboLabel(event),
        decision: event.decision,
        complete: false,
        repeat: false,
        selectable: usable,
      };
      entries.push(entry);
      open.set(`k:${event.token}`, entry);
    } else {
      const entry = open.get(`k:${event.token}`);
      if (entry) {
        entry.complete = true;
        open.delete(`k:${event.token}`);
      }
    }
  }

  return entries;
}

function decisionBadge(entry: CaptureLogEntry) {
  switch (entry.decision) {
    case "filtered":
      return <Badge tone="warning">Filtered</Badge>;
    case "own":
      return <Badge tone="outline">Own input</Badge>;
    default:
      return null;
  }
}

/**
 * Live feed of what the hooks actually saw. Entries are clickable so the user
 * can choose which of several captured inputs should become the mapping.
 */
export function CaptureLog({
  events,
  ready,
  active = true,
  selectedSeq,
  onPick,
  className,
}: {
  events: CaptureEvent[];
  ready: boolean;
  active?: boolean;
  selectedSeq?: number;
  onPick?: (event: CaptureEvent) => void;
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

  const entries = captureLogEntries(events).slice(-40);

  return (
    <div
      className={cn(
        "rounded-xl border border-zinc-200 bg-zinc-50/70 dark:border-white/[0.08] dark:bg-white/[0.02]",
        className,
      )}
      aria-live="polite"
    >
      <div className="flex items-center justify-between gap-2 px-3 pt-2.5">
        <span className="text-[11.5px] font-medium">
          {active
            ? ready
              ? "Listening — press a button or key"
              : "Starting…"
            : "Last input"}
        </span>
        <span className="text-[10.5px] text-zinc-400 dark:text-zinc-500">
          {entries.length > 0
            ? `${entries.length} gesture${entries.length === 1 ? "" : "s"}`
            : active
              ? "Esc to stop"
              : ""}
        </span>
      </div>

      {entries.length === 0 ? (
        <p className="px-3 pb-2.5 pt-1.5 text-[11px] leading-relaxed text-zinc-400 dark:text-zinc-500">
          {!active
            ? "No input was recorded."
            : waited
              ? "No input seen yet. Vendor mouse software can hide buttons — enable “Process input from other input tools” in Settings to see them."
              : "Input appears here as soon as the hook sees it."}
        </p>
      ) : (
        <ul className="mt-1 max-h-44 space-y-1 overflow-y-auto px-2 pb-2.5">
          {entries.map((entry) => {
            const selected = selectedSeq === entry.event.seq;
            const disabled = !entry.selectable || !onPick;
            return (
              <li key={entry.id}>
                <button
                  type="button"
                  disabled={disabled}
                  onClick={() => onPick?.(entry.event)}
                  className={cn(
                    "flex w-full items-center justify-between gap-2 rounded-lg px-2 py-1.5 text-left text-[11.5px] transition",
                    disabled ? "cursor-default" : "hover:bg-zinc-100 dark:hover:bg-white/[0.06]",
                    selected &&
                      "bg-emerald-500/10 text-emerald-700 dark:text-emerald-400",
                    entry.decision === "filtered" && "opacity-70",
                    entry.decision === "own" && "opacity-60",
                  )}
                  title={
                    entry.selectable
                      ? "Use this input for the mapping"
                      : "Seen during the session, not selectable"
                  }
                >
                  <span className="min-w-0 truncate font-mono">
                    {entry.label}
                    {entry.repeat && (
                      <span className="ml-1 text-zinc-400">· repeat</span>
                    )}
                  </span>
                  <span className="flex shrink-0 items-center gap-1.5">
                    {decisionBadge(entry)}
                    {selected ? (
                      <Badge tone="success">Using</Badge>
                    ) : entry.selectable && onPick ? (
                      <span className="text-[10px] text-zinc-400 dark:text-zinc-500">
                        Use
                      </span>
                    ) : null}
                  </span>
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
