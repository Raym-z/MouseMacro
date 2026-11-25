import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "../lib/cn";

export function EmptyState({
  icon: Icon,
  title,
  description,
  action,
  className,
}: {
  icon: LucideIcon;
  title: string;
  description: string;
  action?: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "flex flex-col items-center justify-center px-6 py-14 text-center",
        className,
      )}
    >
      <div className="grid size-12 place-items-center rounded-2xl border border-zinc-200/80 bg-zinc-50 text-zinc-400 dark:border-white/[0.06] dark:bg-white/[0.03] dark:text-zinc-500">
        <Icon className="size-5" strokeWidth={1.6} />
      </div>
      <h3 className="mt-4 text-[14px] font-semibold">{title}</h3>
      <p className="mt-1 max-w-[340px] text-[12.5px] leading-relaxed text-secondary">
        {description}
      </p>
      {action && <div className="mt-5">{action}</div>}
    </div>
  );
}
