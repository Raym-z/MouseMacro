import type { ButtonHTMLAttributes, ReactNode } from "react";
import { Loader2 } from "lucide-react";
import { cn } from "../lib/cn";

type Variant = "primary" | "secondary" | "ghost" | "danger" | "subtle";
type Size = "sm" | "md" | "lg" | "icon";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
  loading?: boolean;
  icon?: ReactNode;
}

const variants: Record<Variant, string> = {
  primary:
    "bg-zinc-900 text-white hover:bg-zinc-800 active:bg-zinc-950 dark:bg-zinc-50 dark:text-zinc-900 dark:hover:bg-white dark:active:bg-zinc-200 shadow-sm",
  secondary:
    "border border-zinc-200 bg-white text-zinc-700 hover:bg-zinc-50 active:bg-zinc-100 dark:border-white/10 dark:bg-white/[0.045] dark:text-zinc-200 dark:hover:bg-white/[0.08]",
  ghost:
    "text-zinc-600 hover:bg-zinc-100 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-white/[0.07] dark:hover:text-zinc-100",
  danger:
    "bg-rose-600 text-white hover:bg-rose-500 active:bg-rose-700 shadow-sm",
  subtle:
    "bg-zinc-100 text-zinc-700 hover:bg-zinc-200/80 dark:bg-white/[0.07] dark:text-zinc-200 dark:hover:bg-white/[0.11]",
};

const sizes: Record<Size, string> = {
  sm: "h-8 rounded-lg px-2.5 text-[12.5px] gap-1.5",
  md: "h-9 rounded-[9px] px-3.5 text-[13px] gap-2",
  lg: "h-10 rounded-[10px] px-4 text-[13.5px] gap-2",
  icon: "size-8 rounded-lg justify-center",
};

export function Button({
  variant = "secondary",
  size = "md",
  loading,
  icon,
  className,
  children,
  disabled,
  ...rest
}: ButtonProps) {
  return (
    <button
      className={cn(
        "focus-ring inline-flex select-none items-center justify-center whitespace-nowrap font-medium transition-colors duration-150 disabled:pointer-events-none disabled:opacity-45",
        variants[variant],
        sizes[size],
        className,
      )}
      disabled={disabled || loading}
      {...rest}
    >
      {loading ? (
        <Loader2 className="size-3.5 animate-spin" />
      ) : (
        icon && <span className="shrink-0 [&>svg]:size-4">{icon}</span>
      )}
      {children}
    </button>
  );
}
