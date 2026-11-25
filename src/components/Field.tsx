import type { ReactNode } from "react";
import { cn } from "../lib/cn";

export function Field({
  label,
  hint,
  children,
  className,
  htmlFor,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
  className?: string;
  htmlFor?: string;
}) {
  return (
    <div className={cn("space-y-1.5", className)}>
      <label
        htmlFor={htmlFor}
        className="block text-[12.5px] font-medium text-zinc-700 dark:text-zinc-300"
      >
        {label}
      </label>
      {children}
      {hint && (
        <p className="text-[11.5px] leading-relaxed text-zinc-400 dark:text-zinc-500">
          {hint}
        </p>
      )}
    </div>
  );
}

export function Segmented<T extends string>({
  options,
  value,
  onChange,
  size = "md",
  className,
  disabled,
}: {
  options: Array<{ value: T; label: ReactNode; title?: string }>;
  value: T;
  onChange: (value: T) => void;
  size?: "sm" | "md";
  className?: string;
  disabled?: boolean;
}) {
  return (
    <div
      className={cn(
        "inline-flex flex-wrap items-center gap-0.5 rounded-[10px] border border-zinc-200/80 bg-zinc-100/70 p-0.5 dark:border-white/[0.06] dark:bg-white/[0.04]",
        className,
      )}
      role="group"
    >
      {options.map((option) => {
        const active = option.value === value;
        return (
          <button
            key={option.value}
            type="button"
            title={option.title}
            disabled={disabled}
            onClick={() => onChange(option.value)}
            className={cn(
              "focus-ring rounded-[8px] font-medium transition-all duration-150 disabled:opacity-40",
              size === "sm"
                ? "h-[26px] px-2 text-[11.5px]"
                : "h-[30px] px-2.5 text-[12.5px]",
              active
                ? "bg-white text-zinc-900 shadow-[0_1px_2px_rgba(0,0,0,0.08)] dark:bg-white/[0.12] dark:text-white"
                : "text-zinc-500 hover:text-zinc-800 dark:text-zinc-400 dark:hover:text-zinc-200",
            )}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}
