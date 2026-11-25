import { useEffect, useMemo, useRef, useState } from "react";
import { Check, ChevronDown, Keyboard, Search, X } from "lucide-react";
import { Button } from "./Button";
import { Kbd } from "./Kbd";
import { CaptureLog } from "./CaptureLog";
import { cn } from "../lib/cn";
import {
  MODIFIER_KEYS,
  MODIFIER_TOKENS,
  isModifierToken,
  keyTokenLabel,
  searchKeys,
} from "../lib/keys";
import type { CaptureEvent } from "../lib/types";

interface KeySelectorProps {
  mode: "combo" | "single";
  keys: string[];
  onChange: (keys: string[]) => void;
  /** Starts or stops the shared capture session. */
  onRecord?: () => void;
  recording?: boolean;
  recordReady?: boolean;
  recordEvents?: CaptureEvent[];
  /** Keep showing the last log after Stop. */
  showLog?: boolean;
}

/** Keeps Ctrl, Shift, Alt, Win in a stable order regardless of click order. */
function orderModifiers(tokens: string[]): string[] {
  return MODIFIER_TOKENS.filter((token) => tokens.includes(token));
}

/**
 * Manual keyboard picker. Everything here works without listening to global
 * keyboard input: the user searches a catalog and clicks a key, or presses a
 * key while this picker (not the global hook) has focus.
 */
export function KeySelector({
  mode,
  keys,
  onChange,
  onRecord,
  recording = false,
  recordReady = false,
  recordEvents,
  showLog = false,
}: KeySelectorProps) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const containerRef = useRef<HTMLDivElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const searchRef = useRef<HTMLInputElement>(null);

  const modifiers = keys.filter(isModifierToken);
  const primary = keys.find((key) => !isModifierToken(key)) ?? null;
  const results = useMemo(
    () =>
      searchKeys(
        query,
        mode === "combo" ? MODIFIER_TOKENS : [],
        mode === "single",
      ),
    [query, mode],
  );
  const totalMatches = results.reduce((sum, result) => sum + result.matches.length, 0);

  const closePanel = () => {
    setOpen(false);
    setQuery("");
  };

  // Close when clicking outside of the selector.
  useEffect(() => {
    if (!open) return;
    const handler = (event: MouseEvent) => {
      if (!containerRef.current?.contains(event.target as Node)) closePanel();
    };
    window.addEventListener("mousedown", handler);
    return () => window.removeEventListener("mousedown", handler);
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const timer = window.setTimeout(() => {
      panelRef.current?.scrollIntoView({ block: "nearest", behavior: "smooth" });
      searchRef.current?.focus({ preventScroll: true });
    }, 40);
    return () => window.clearTimeout(timer);
  }, [open]);

  const applyKey = (token: string) => {
    if (mode === "single") {
      onChange([token]);
    } else {
      onChange([...orderModifiers(modifiers), token]);
    }
    closePanel();
  };

  const toggleModifier = (token: string) => {
    const next = modifiers.includes(token)
      ? modifiers.filter((item) => item !== token)
      : [...modifiers, token];
    const rest = keys.filter((key) => !isModifierToken(key));
    onChange([...orderModifiers(next), ...rest]);
  };

  const clear = () => {
    onChange([]);
  };

  const chipClass = (active: boolean) =>
    cn(
      "focus-ring inline-flex h-8 min-w-[38px] items-center justify-center gap-1 rounded-lg border px-2 text-[12px] font-medium transition",
      active
        ? "border-zinc-900 bg-zinc-900 text-white dark:border-zinc-100 dark:bg-zinc-100 dark:text-zinc-900"
        : "border-zinc-200 bg-white text-zinc-600 hover:border-zinc-300 hover:bg-zinc-50 dark:border-white/10 dark:bg-white/[0.04] dark:text-zinc-300 dark:hover:bg-white/[0.08]",
    );

  return (
    <div
      ref={containerRef}
      className="rounded-xl border border-zinc-200 bg-zinc-50/60 p-3 dark:border-white/[0.08] dark:bg-white/[0.02]"
    >
      <div className="flex items-center justify-between gap-3">
        <div className="flex items-center gap-2 text-[12px] font-medium">
          <Keyboard className="size-3.5 text-zinc-400" />
          {mode === "single" ? "Keyboard key" : "Keyboard shortcut"}
        </div>
        <div className="flex items-center gap-1">
          {keys.length > 0 && (
            <Button size="sm" variant="ghost" onClick={clear}>
              Clear
            </Button>
          )}
          {onRecord && (
            <Button
              size="sm"
              variant={recording ? "subtle" : "ghost"}
              onClick={onRecord}
            >
              {recording ? "Stop" : "Record"}
            </Button>
          )}
          <Button
            size="sm"
            variant={open ? "subtle" : "secondary"}
            icon={
              <ChevronDown
                className={cn(
                  "transition-transform duration-150",
                  open && "rotate-180",
                )}
              />
            }
            onClick={() => {
              if (open) {
                closePanel();
              } else {
                setOpen(true);
              }
            }}
          >
            Choose
          </Button>
        </div>
      </div>

      <div className="mt-2.5 flex min-h-[30px] flex-wrap items-center gap-1.5">
        {keys.length > 0 ? (
          <>
            {keys.map((key, index) => (
              <span key={`${key}-${index}`} className="inline-flex items-center gap-1">
                {index > 0 && (
                  <span className="text-[11px] text-zinc-400">+</span>
                )}
                <Kbd>{keyTokenLabel(key)}</Kbd>
              </span>
            ))}
          </>
        ) : (
          <span className="text-[12px] text-zinc-400 dark:text-zinc-500">
            {mode === "single"
              ? "No key selected — choose one from the list."
              : "No keys selected — pick modifiers and a key."}
          </span>
        )}
      </div>

      {(recording || (showLog && (recordEvents?.length ?? 0) > 0)) && (
        <CaptureLog
          events={recordEvents ?? []}
          ready={recordReady}
          active={recording}
          className="mt-2.5"
        />
      )}

      {open && (
        <div
          ref={panelRef}
          className="animate-fade mt-3 overflow-hidden rounded-xl border border-zinc-200 bg-white shadow-sm dark:border-white/10 dark:bg-[#141416]"
        >
          {mode === "combo" && (
            <div className="flex flex-wrap items-center gap-1.5 border-b border-zinc-100 px-2.5 py-2 dark:border-white/[0.06]">
              <span className="mr-0.5 text-[11px] font-medium text-zinc-400">
                Modifiers
              </span>
              {MODIFIER_KEYS.map((modifier) => (
                <button
                  key={modifier.token}
                  type="button"
                  onClick={() => toggleModifier(modifier.token)}
                  className={chipClass(modifiers.includes(modifier.token))}
                >
                  {modifier.label}
                </button>
              ))}
            </div>
          )}

          <div className="flex items-center gap-2 border-b border-zinc-100 px-2.5 py-1.5 dark:border-white/[0.06]">
            <Search className="size-3.5 shrink-0 text-zinc-400" />
            <input
              ref={searchRef}
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Escape") {
                  event.preventDefault();
                  event.nativeEvent.stopImmediatePropagation();
                  closePanel();
                }
                if (event.key === "Enter" && totalMatches === 1) {
                  const only = results[0]?.matches[0];
                  if (only) applyKey(only.token);
                }
              }}
              placeholder="Search keys — try “page”, “f5”, “semi”…"
              className="h-8 min-w-0 flex-1 bg-transparent text-[12.5px] outline-none placeholder:text-zinc-400 dark:placeholder:text-zinc-600"
            />
            {query && (
              <button
                type="button"
                onClick={() => setQuery("")}
                className="focus-ring grid size-6 place-items-center rounded-md text-zinc-400 hover:bg-zinc-100 hover:text-zinc-600 dark:hover:bg-white/10"
                aria-label="Clear search"
              >
                <X className="size-3.5" />
              </button>
            )}
          </div>

          <div className="max-h-[224px] space-y-2.5 overflow-y-auto px-2.5 py-2.5">
            {results.length === 0 ? (
              <p className="py-3 text-center text-[12px] text-zinc-400">
                No key matches “{query}”.
              </p>
            ) : (
              results.map((result) => (
                <div key={result.group.id}>
                  <div className="mb-1 text-[10.5px] font-semibold uppercase tracking-wide text-zinc-400 dark:text-zinc-500">
                    {result.group.label}
                  </div>
                  <div className="flex flex-wrap gap-1">
                    {result.matches.map((option) => {
                      const active = primary === option.token;
                      return (
                        <button
                          key={option.token}
                          type="button"
                          onClick={() => applyKey(option.token)}
                          className={chipClass(active)}
                          title={option.token}
                        >
                          {active && <Check className="size-3" />}
                          {option.label}
                        </button>
                      );
                    })}
                  </div>
                </div>
              ))
            )}
          </div>

          <div className="border-t border-zinc-100 px-2.5 py-1.5 text-[11px] leading-relaxed text-zinc-400 dark:border-white/[0.06] dark:text-zinc-500">
            {mode === "combo"
              ? "Modifiers are optional. Keys are pressed in the order shown."
              : "Pick one key. A modifier on its own (Ctrl, Shift…) is allowed."}
          </div>
        </div>
      )}
    </div>
  );
}
