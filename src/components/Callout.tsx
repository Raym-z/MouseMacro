import type { ReactNode } from "react";
import { AlertTriangle, CheckCircle2, Info, ShieldAlert } from "lucide-react";
import { cn } from "../lib/cn";

const tones = {
  info: {
    wrap: "border-indigo-500/15 bg-indigo-500/[0.06] text-indigo-900 dark:text-indigo-200",
    icon: "text-indigo-500 dark:text-indigo-400",
    Icon: Info,
  },
  warning: {
    wrap: "border-amber-500/20 bg-amber-500/[0.07] text-amber-900 dark:text-amber-200",
    icon: "text-amber-500 dark:text-amber-400",
    Icon: AlertTriangle,
  },
  danger: {
    wrap: "border-rose-500/20 bg-rose-500/[0.06] text-rose-900 dark:text-rose-200",
    icon: "text-rose-500 dark:text-rose-400",
    Icon: ShieldAlert,
  },
  success: {
    wrap: "border-emerald-500/20 bg-emerald-500/[0.06] text-emerald-900 dark:text-emerald-200",
    icon: "text-emerald-500 dark:text-emerald-400",
    Icon: CheckCircle2,
  },
};

export function Callout({
  tone = "info",
  title,
  children,
  action,
  className,
}: {
  tone?: keyof typeof tones;
  title?: string;
  children: ReactNode;
  action?: ReactNode;
  className?: string;
}) {
  const { wrap, icon, Icon } = tones[tone];
  return (
    <div
      className={cn(
        "flex items-start gap-3 rounded-xl border px-3.5 py-3 text-[12.5px] leading-relaxed",
        wrap,
        className,
      )}
    >
      <Icon className={cn("mt-0.5 size-4 shrink-0", icon)} strokeWidth={1.9} />
      <div className="min-w-0 flex-1">
        {title && <div className="mb-0.5 font-semibold">{title}</div>}
        <div className="opacity-90">{children}</div>
        {action && <div className="mt-2.5">{action}</div>}
      </div>
    </div>
  );
}
