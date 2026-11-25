import { Plus } from "lucide-react";
import { cn } from "../lib/cn";
import { keyLabel } from "../lib/format";

export function Kbd({ children }: { children: React.ReactNode }) {
  return (
    <kbd className="inline-flex h-[23px] min-w-[23px] items-center justify-center rounded-[6px] border border-zinc-200/90 bg-zinc-50 px-1.5 font-sans text-[11px] font-medium text-zinc-600 shadow-[0_1px_0_rgba(0,0,0,0.05)] dark:border-white/10 dark:bg-white/[0.06] dark:text-zinc-300">
      {children}
    </kbd>
  );
}

export function KeyCombo({
  keys,
  className,
  max,
}: {
  keys: string[];
  className?: string;
  max?: number;
}) {
  if (keys.length === 0) {
    return <span className="text-[12px] text-rose-500">Not recorded</span>;
  }
  const visible = max ? keys.slice(0, max) : keys;
  const hidden = max ? Math.max(0, keys.length - max) : 0;
  return (
    <span className={cn("inline-flex flex-wrap items-center gap-1", className)}>
      {visible.map((key, index) => (
        <span key={`${key}-${index}`} className="inline-flex items-center gap-1">
          {index > 0 && (
            <Plus className="size-2.5 text-zinc-400 dark:text-zinc-500" />
          )}
          <Kbd>{keyLabel(key)}</Kbd>
        </span>
      ))}
      {hidden > 0 && (
        <span className="text-[11px] text-zinc-400">+{hidden}</span>
      )}
    </span>
  );
}
