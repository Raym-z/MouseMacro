import type { Action } from "../lib/types";
import { actionKeys, actionLabel } from "../lib/format";
import { KeyCombo } from "./Kbd";
import { cn } from "../lib/cn";

export function ActionDisplay({
  action,
  compact,
  className,
}: {
  action: Action;
  compact?: boolean;
  className?: string;
}) {
  const keys = actionKeys(action);
  const isHold = action.type === "holdKeys";
  if (keys) {
    return (
      <span className={cn("inline-flex items-center gap-1.5", className)}>
        {isHold && (
          <span className="text-[10.5px] font-medium uppercase tracking-wide text-zinc-400 dark:text-zinc-500">
            Hold
          </span>
        )}
        <KeyCombo keys={keys} max={compact ? 3 : undefined} />
      </span>
    );
  }
  return (
    <span
      className={cn(
        "text-[12px] text-zinc-600 dark:text-zinc-300",
        className,
      )}
    >
      {actionLabel(action)}
    </span>
  );
}
