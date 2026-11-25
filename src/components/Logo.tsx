import { cn } from "../lib/cn";

export function LogoMark({ className }: { className?: string }) {
  return (
    <span
      className={cn(
        "grid size-8 shrink-0 place-items-center rounded-[10px] bg-zinc-900 text-white shadow-sm dark:bg-white dark:text-zinc-900",
        className,
      )}
      aria-hidden
    >
      <svg viewBox="0 0 32 32" fill="none" className="size-[19px]">
        <rect
          x="9"
          y="5.5"
          width="14"
          height="21"
          rx="7"
          stroke="currentColor"
          strokeWidth="1.7"
        />
        <path d="M16 6v8" stroke="currentColor" strokeWidth="1.7" />
        <path
          d="M13 11.5h6"
          stroke="currentColor"
          strokeWidth="1.2"
          opacity="0.55"
        />
      </svg>
    </span>
  );
}

export function Wordmark() {
  return (
    <div className="flex items-center gap-2.5">
      <LogoMark />
      <div className="leading-none">
        <div className="text-[14px] font-semibold tracking-[-0.02em]">
          MouseMacro
        </div>
        <div className="mt-0.5 text-[10px] font-medium uppercase tracking-[0.14em] text-zinc-400 dark:text-zinc-500">
          Input studio
        </div>
      </div>
    </div>
  );
}
