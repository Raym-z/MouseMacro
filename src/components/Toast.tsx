import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { AlertTriangle, Check, Info, X } from "lucide-react";
import { cn } from "../lib/cn";

type ToastTone = "success" | "error" | "info";

interface ToastItem {
  id: string;
  message: string;
  tone: ToastTone;
}

interface ToastContextValue {
  toast: (message: string, tone?: ToastTone) => void;
}

const ToastContext = createContext<ToastContextValue | null>(null);

export function useToast(): ToastContextValue {
  const context = useContext(ToastContext);
  if (!context) throw new Error("useToast must be used inside ToastProvider");
  return context;
}

const toneIcon = {
  success: Check,
  error: AlertTriangle,
  info: Info,
};

export function ToastProvider({ children }: { children: ReactNode }) {
  const [items, setItems] = useState<ToastItem[]>([]);
  const timers = useRef(new Map<string, number>());

  const dismiss = useCallback((id: string) => {
    setItems((current) => current.filter((item) => item.id !== id));
    const timer = timers.current.get(id);
    if (timer) {
      window.clearTimeout(timer);
      timers.current.delete(id);
    }
  }, []);

  const toast = useCallback(
    (message: string, tone: ToastTone = "info") => {
      const id = crypto.randomUUID();
      setItems((current) => [...current.slice(-3), { id, message, tone }]);
      const timer = window.setTimeout(() => dismiss(id), tone === "error" ? 6000 : 3600);
      timers.current.set(id, timer);
    },
    [dismiss],
  );

  const value = useMemo(() => ({ toast }), [toast]);

  return (
    <ToastContext.Provider value={value}>
      {children}
      <div className="pointer-events-none fixed inset-x-0 bottom-5 z-[120] flex flex-col items-center gap-2 px-4">
        {items.map((item) => {
          const Icon = toneIcon[item.tone];
          return (
            <div
              key={item.id}
              className={cn(
                "animate-in pointer-events-auto flex max-w-[520px] items-start gap-2.5 rounded-xl border px-3.5 py-2.5 text-[13px] shadow-lg backdrop-blur",
                "border-zinc-200/90 bg-white/95 text-zinc-700",
                "dark:border-white/10 dark:bg-[#1b1b1e]/95 dark:text-zinc-200",
              )}
              role="status"
            >
              <span
                className={cn(
                  "mt-0.5 grid size-4 shrink-0 place-items-center rounded-full",
                  item.tone === "success" && "bg-emerald-500/15 text-emerald-600 dark:text-emerald-400",
                  item.tone === "error" && "bg-rose-500/15 text-rose-600 dark:text-rose-400",
                  item.tone === "info" && "bg-zinc-500/10 text-zinc-500 dark:text-zinc-300",
                )}
              >
                <Icon className="size-3" strokeWidth={2.5} />
              </span>
              <span className="leading-snug">{item.message}</span>
              <button
                className="focus-ring ml-1 mt-0.5 rounded text-zinc-400 transition hover:text-zinc-700 dark:hover:text-zinc-100"
                onClick={() => dismiss(item.id)}
                aria-label="Dismiss notification"
              >
                <X className="size-3.5" />
              </button>
            </div>
          );
        })}
      </div>
    </ToastContext.Provider>
  );
}
