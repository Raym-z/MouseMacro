import type { ReactNode } from "react";
import { cn } from "../lib/cn";

type Tone = "neutral" | "success" | "warning" | "danger" | "accent" | "outline";

const tones: Record<Tone, string> = {
  neutral:
    "border-zinc-200/80 bg-zinc-100/80 text-zinc-600 dark:border-white/[0.06] dark:bg-white/[0.06] dark:text-zinc-300",
  success:
    "border-emerald-500/15 bg-emerald-500/10 text-emerald-700 dark:text-emerald-400",
  warning:
    "border-amber-500/20 bg-amber-500/10 text-amber-700 dark:text-amber-400",
  danger: "border-rose-500/20 bg-rose-500/10 text-rose-700 dark:text-rose-400",
  accent:
    "border-indigo-500/15 bg-indigo-500/10 text-indigo-700 dark:text-indigo-300",
  outline:
    "border-zinc-200 bg-transparent text-zinc-600 dark:border-white/10 dark:text-zinc-400",
};

export function Badge({
  children,
  tone = "neutral",
  className,
  dot,
}: {
  children: ReactNode;
  tone?: Tone;
  className?: string;
  dot?: boolean;
}) {
  return (
    <span
      className={cn(
        "inline-flex h-[22px] items-center gap-1.5 rounded-full border px-2 text-[11px] font-medium leading-none",
        tones[tone],
        className,
      )}
    >
      {dot && (
        <span
          className={cn(
            "size-1.5 rounded-full",
            tone === "success" && "bg-emerald-500",
            tone === "warning" && "bg-amber-500",
            tone === "danger" && "bg-rose-500",
            tone === "accent" && "bg-indigo-500",
            (tone === "neutral" || tone === "outline") && "bg-zinc-400",
          )}
        />
      )}
      {children}
    </span>
  );
}
