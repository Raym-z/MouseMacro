import { ChevronDown } from "lucide-react";
import type { SelectHTMLAttributes } from "react";
import { cn } from "../lib/cn";

export function Select({
  className,
  children,
  ...rest
}: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <div className={cn("relative", className)}>
      <select
        className={cn(
          "focus-ring h-9 w-full appearance-none rounded-[9px] border border-zinc-200 bg-white pl-3 pr-8 text-[12.5px] text-zinc-800 shadow-[0_1px_1px_rgba(0,0,0,0.02)] transition-colors",
          "hover:border-zinc-300 dark:border-white/10 dark:bg-white/[0.04] dark:text-zinc-200 dark:hover:border-white/20",
          "disabled:opacity-50",
        )}
        {...rest}
      >
        {children}
      </select>
      <ChevronDown className="pointer-events-none absolute right-2.5 top-1/2 size-3.5 -translate-y-1/2 text-zinc-400" />
    </div>
  );
}
