import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api, errorMessage } from "../lib/api";
import type { Snapshot, View } from "../lib/types";
import { useToast } from "../components/Toast";

interface AppContextValue {
  snapshot: Snapshot | null;
  loading: boolean;
  fatalError: string | null;
  view: View;
  setView: (view: View) => void;
  refresh: () => Promise<void>;
  runSnapshot: (
    action: () => Promise<Snapshot>,
    successMessage?: string,
  ) => Promise<Snapshot | null>;
  run: <T>(
    action: () => Promise<T>,
    successMessage?: string,
  ) => Promise<T | null>;
  applySnapshot: (snapshot: Snapshot) => void;
}

const AppContext = createContext<AppContextValue | null>(null);

export function useApp(): AppContextValue {
  const context = useContext(AppContext);
  if (!context) throw new Error("useApp must be used inside AppProvider");
  return context;
}

const VIEWS: View[] = [
  "dashboard",
  "mappings",
  "layers",
  "profiles",
  "exceptions",
  "scroll",
  "settings",
  "about",
];

export function AppProvider({ children }: { children: ReactNode }) {
  const { toast } = useToast();
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [fatalError, setFatalError] = useState<string | null>(null);
  const [view, setView] = useState<View>("dashboard");

  const refresh = useCallback(async () => {
    try {
      const next = await api.getSnapshot();
      setSnapshot(next);
    } catch (error) {
      toast(errorMessage(error), "error");
    }
  }, [toast]);

  const applySnapshot = useCallback((next: Snapshot) => {
    setSnapshot(next);
  }, []);

  const runSnapshot = useCallback(
    async (action: () => Promise<Snapshot>, successMessage?: string) => {
      try {
        const next = await action();
        setSnapshot(next);
        if (successMessage) toast(successMessage, "success");
        return next;
      } catch (error) {
        toast(errorMessage(error), "error");
        return null;
      }
    },
    [toast],
  );

  const run = useCallback(
    async <T,>(action: () => Promise<T>, successMessage?: string) => {
      try {
        const result = await action();
        if (successMessage) toast(successMessage, "success");
        return result;
      } catch (error) {
        toast(errorMessage(error), "error");
        return null;
      }
    },
    [toast],
  );

  useEffect(() => {
    let mounted = true;
    const unlisteners: UnlistenFn[] = [];

    const setup = async () => {
      try {
        const initial = await api.getSnapshot();
        if (!mounted) return;
        setSnapshot(initial);
        setLoading(false);

        unlisteners.push(
          await listen("state:changed", async () => {
            try {
              const next = await api.getSnapshot();
              if (mounted) setSnapshot(next);
            } catch {
              // The window may be closing; ignore transient failures.
            }
          }),
        );
        unlisteners.push(
          await listen<string>("app://error", (event) => {
            toast(event.payload, "error");
          }),
        );
        unlisteners.push(
          await listen<string>("app://navigate", (event) => {
            if (VIEWS.includes(event.payload as View)) {
              setView(event.payload as View);
            }
          }),
        );
        unlisteners.push(
          await listen<string>("app://panic", (event) => {
            toast(event.payload, "error");
          }),
        );

        try {
          const ready = await api.appReady();
          if (mounted) setSnapshot(ready);
        } catch {
          // The Rust fallback timer reveals the window if this fails.
        }
      } catch (error) {
        if (mounted) {
          setFatalError(errorMessage(error));
          setLoading(false);
        }
      }
    };

    void setup();
    return () => {
      mounted = false;
      for (const unlisten of unlisteners) unlisten();
    };
  }, [toast]);

  const theme = snapshot?.config.settings.theme;

  useEffect(() => {
    if (!theme) return;
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const dark = theme === "dark" || (theme === "system" && media.matches);
      document.documentElement.classList.toggle("dark", dark);
      document.documentElement.style.colorScheme = dark ? "dark" : "light";
    };
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [theme]);

  const value = useMemo(
    () => ({
      snapshot,
      loading,
      fatalError,
      view,
      setView,
      refresh,
      runSnapshot,
      run,
      applySnapshot,
    }),
    [snapshot, loading, fatalError, view, refresh, runSnapshot, run, applySnapshot],
  );

  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
}
