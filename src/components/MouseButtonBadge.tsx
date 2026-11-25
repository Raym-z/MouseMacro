import { cn } from "../lib/cn";
import type { MouseButton } from "../lib/types";
import { buttonLabel } from "../lib/format";

export function MouseButtonBadge({
  button,
  className,
  muted,
  hideLabel,
}: {
  button: MouseButton;
  className?: string;
  muted?: boolean;
  /** Shows only the mouse glyph, for use next to an existing text label. */
  hideLabel?: boolean;
}) {
  return (
    <span
      className={cn(
        "relative grid size-9 shrink-0 place-items-center overflow-hidden rounded-[10px] border text-[10px] font-semibold tracking-tight",
        muted
          ? "border-zinc-200/70 bg-zinc-50 text-zinc-400 dark:border-white/[0.06] dark:bg-white/[0.03] dark:text-zinc-600"
          : "border-zinc-200/90 bg-gradient-to-b from-white to-zinc-100 text-zinc-600 shadow-[inset_0_1px_0_rgba(255,255,255,0.9)] dark:border-white/10 dark:from-white/[0.09] dark:to-white/[0.035] dark:text-zinc-300 dark:shadow-none",
        className,
      )}
      title={`${buttonLabel(button)} button`}
    >
      <MouseGlyph className="absolute -right-1 -top-1 size-5 text-zinc-300/60 dark:text-white/10" />
      {!hideLabel && <span className="relative">{buttonLabel(button)}</span>}
    </span>
  );
}

export function MouseGlyph({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="none" className={className} aria-hidden>
      <rect
        x="6.5"
        y="2.5"
        width="11"
        height="19"
        rx="5.5"
        stroke="currentColor"
        strokeWidth="1.4"
      />
      <path d="M12 3v6" stroke="currentColor" strokeWidth="1.4" />
    </svg>
  );
}
