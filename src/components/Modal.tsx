import type { ReactNode } from "react";
import { createPortal } from "react-dom";
import { useEffect } from "react";
import { X } from "lucide-react";
import { cn } from "../lib/cn";
import { Button } from "./Button";

interface ModalProps {
  open: boolean;
  onClose: () => void;
  title: string;
  description?: string;
  children?: ReactNode;
  footer?: ReactNode;
  size?: "sm" | "md" | "lg";
  headerExtra?: ReactNode;
}

const widths = {
  sm: "max-w-[380px]",
  md: "max-w-[480px]",
  lg: "max-w-[620px]",
};

export function Modal({
  open,
  onClose,
  title,
  description,
  children,
  footer,
  size = "md",
  headerExtra,
}: ModalProps) {
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
    <div
      className="animate-fade fixed inset-0 z-[80] flex items-center justify-center bg-zinc-950/35 p-6 backdrop-blur-[3px] dark:bg-black/55"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className={cn(
          "animate-in surface w-full overflow-hidden rounded-2xl",
          widths[size],
        )}
      >
        <div className="flex items-start justify-between gap-4 px-5 pb-1 pt-5">
          <div>
            <h2 className="text-[16px] font-semibold tracking-[-0.01em]">
              {title}
            </h2>
            {description && (
              <p className="mt-1 text-[12.5px] leading-relaxed text-secondary">
                {description}
              </p>
            )}
          </div>
          <div className="flex items-center gap-1">
            {headerExtra}
            <button
              onClick={onClose}
              className="focus-ring grid size-7 place-items-center rounded-lg text-zinc-400 transition hover:bg-zinc-100 hover:text-zinc-700 dark:hover:bg-white/10 dark:hover:text-zinc-100"
              aria-label="Close"
            >
              <X className="size-4" />
            </button>
          </div>
        </div>
        <div className="px-5 py-4">{children}</div>
        {footer && (
          <div className="flex items-center justify-end gap-2 border-t border-zinc-100 bg-zinc-50/60 px-5 py-3 dark:border-white/[0.06] dark:bg-white/[0.02]">
            {footer}
          </div>
        )}
      </div>
    </div>,
    document.body,
  );
}

export function ConfirmDialog({
  open,
  onClose,
  onConfirm,
  title,
  description,
  confirmLabel = "Confirm",
  tone = "danger",
  loading,
  children,
}: {
  open: boolean;
  onClose: () => void;
  onConfirm: () => void;
  title: string;
  description?: string;
  confirmLabel?: string;
  tone?: "danger" | "primary";
  loading?: boolean;
  children?: ReactNode;
}) {
  return (
    <Modal
      open={open}
      onClose={onClose}
      title={title}
      description={description}
      size="sm"
      footer={
        <>
          <Button variant="ghost" onClick={onClose} disabled={loading}>
            Cancel
          </Button>
          <Button
            variant={tone === "danger" ? "danger" : "primary"}
            onClick={onConfirm}
            loading={loading}
          >
            {confirmLabel}
          </Button>
        </>
      }
    >
      {children}
    </Modal>
  );
}
