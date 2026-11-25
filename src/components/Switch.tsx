import { cn } from "../lib/cn";

interface SwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  size?: "sm" | "md";
  label?: string;
}

export function Switch({
  checked,
  onChange,
  disabled,
  size = "md",
  label,
}: SwitchProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={(event) => {
        event.stopPropagation();
        onChange(!checked);
      }}
      className={cn(
        "focus-ring relative inline-flex shrink-0 items-center rounded-full transition-colors duration-200 disabled:opacity-45",
        size === "md" ? "h-[22px] w-[38px]" : "h-[18px] w-[32px]",
        checked
          ? "bg-zinc-900 dark:bg-zinc-100"
          : "bg-zinc-300/90 dark:bg-white/15",
      )}
    >
      <span
        className={cn(
          "absolute top-[2px] rounded-full bg-white shadow-[0_1px_2px_rgba(0,0,0,0.28)] transition-transform duration-200 dark:bg-zinc-900",
          size === "md" ? "size-[18px]" : "size-[14px]",
          checked
            ? size === "md"
              ? "translate-x-[18px]"
              : "translate-x-[16px]"
            : size === "md"
              ? "translate-x-[2px]"
              : "translate-x-[2px]",
        )}
      />
    </button>
  );
}
