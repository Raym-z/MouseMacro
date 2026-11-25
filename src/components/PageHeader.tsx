import type { ReactNode } from "react";
import { cn } from "../lib/cn";

export function PageHeader({
  title,
  description,
  actions,
  eyebrow,
  className,
}: {
  title: string;
  description?: string;
  actions?: ReactNode;
  eyebrow?: string;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "mb-6 flex items-start justify-between gap-5",
        className,
      )}
    >
      <div className="min-w-0">
        {eyebrow && (
          <div className="mb-1.5 text-[11px] font-semibold uppercase tracking-[0.14em] text-zinc-400 dark:text-zinc-500">
            {eyebrow}
          </div>
        )}
        <h1 className="text-[24px] font-semibold tracking-[-0.03em]">{title}</h1>
        {description && (
          <p className="mt-1.5 max-w-[620px] text-[13px] leading-relaxed text-secondary">
            {description}
          </p>
        )}
      </div>
      {actions && (
        <div className="flex shrink-0 flex-wrap items-center justify-end gap-2">
          {actions}
        </div>
      )}
    </div>
  );
}

export function SectionTitle({
  children,
  action,
  className,
}: {
  children: ReactNode;
  action?: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("mb-3 flex items-end justify-between gap-3", className)}>
      <h2 className="text-[12px] font-semibold uppercase tracking-[0.12em] text-zinc-400 dark:text-zinc-500">
        {children}
      </h2>
      {action}
    </div>
  );
}
