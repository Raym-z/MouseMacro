import type { ReactNode } from "react";
import { createPortal } from "react-dom";
import { useEffect } from "react";
import { X } from "lucide-react";
import { cn } from "../lib/cn";

interface SheetProps {
  open: boolean;
  onClose: () => void;
  title: string;
  subtitle?: string;
  children: ReactNode;
  footer?: ReactNode;
  width?: string;
}

export function Sheet({
  open,
  onClose,
  title,
  subtitle,
  children,
  footer,
  width = "w-[560px]",
}: SheetProps) {
  useEffect(() => {
    if (!open) return;
    const handler = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, onClose]);

  if (!open) return null;

  return createPortal(
    <div className="fixed inset-0 z-[70] flex justify-end">
      <div
        className="animate-fade absolute inset-0 bg-zinc-950/30 backdrop-blur-[2px] dark:bg-black/50"
        onMouseDown={onClose}
      />
      <div
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className={cn(
          "animate-sheet relative flex h-full max-w-full flex-col border-l border-zinc-200/80 bg-white shadow-2xl dark:border-white/[0.07] dark:bg-[#101012] max-sm:w-full",
          width,
        )}
      >
        <div className="flex items-start justify-between gap-4 border-b border-zinc-100 px-6 py-5 dark:border-white/[0.06]">
          <div>
            <h2 className="text-[18px] font-semibold tracking-[-0.02em]">
              {title}
            </h2>
            {subtitle && (
              <p className="mt-1 text-[12.5px] leading-relaxed text-secondary">
                {subtitle}
              </p>
            )}
          </div>
          <button
            onClick={onClose}
            className="focus-ring grid size-8 place-items-center rounded-lg text-zinc-400 transition hover:bg-zinc-100 hover:text-zinc-700 dark:hover:bg-white/10 dark:hover:text-zinc-100"
            aria-label="Close"
          >
            <X className="size-4" />
          </button>
        </div>
        <div className="flex-1 overflow-y-auto px-6 py-5">{children}</div>
        {footer && (
          <div className="flex items-center justify-between gap-3 border-t border-zinc-100 bg-zinc-50/70 px-6 py-3.5 dark:border-white/[0.06] dark:bg-white/[0.02]">
            {footer}
          </div>
        )}
      </div>
    </div>,
    document.body,
  );
}
