import { useState } from "react";
import {
  ChevronRight,
  Info,
  Layers,
  Layers3,
  LayoutDashboard,
  MousePointerClick,
  Settings,
  ShieldAlert,
} from "lucide-react";
import { useApp } from "../state/AppContext";
import { api } from "../lib/api";
import { cn } from "../lib/cn";
import { Wordmark } from "./Logo";
import { Switch } from "./Switch";
import type { View } from "../lib/types";

const NAV_ITEMS: Array<{
  view: View;
  label: string;
  icon: typeof LayoutDashboard;
  hint?: string;
}> = [
  { view: "dashboard", label: "Dashboard", icon: LayoutDashboard },
  { view: "mappings", label: "Button mappings", icon: MousePointerClick },
  { view: "layers", label: "Layers", icon: Layers3 },
  { view: "profiles", label: "Profiles", icon: Layers },
  { view: "exceptions", label: "Exceptions", icon: ShieldAlert },
  { view: "settings", label: "Settings", icon: Settings },
  { view: "about", label: "About", icon: Info },
];

export function Sidebar() {
  const { snapshot, view, setView, runSnapshot } = useApp();
  const [busy, setBusy] = useState(false);

  if (!snapshot) return null;
  const { config, status } = snapshot;
  const engineOn = config.settings.enabled && !status.panicActive;
  const profile =
    config.profiles.find((p) => p.id === status.effectiveProfileId) ??
    config.profiles.find((p) => p.id === config.activeProfileId);
  const autoApp =
    status.exceptionMode !== "always" ? status.foregroundApp : null;

  const toggle = async () => {
    setBusy(true);
    await runSnapshot(
      () => api.setEnabled(!engineOn),
      !engineOn ? "Mouse mappings enabled" : "Mouse mappings paused",
    );
    setBusy(false);
  };

  return (
    <aside className="flex h-full w-[236px] shrink-0 flex-col border-r border-zinc-200/70 bg-white/70 px-3 pb-3.5 pt-4 backdrop-blur-xl dark:border-white/[0.06] dark:bg-[#0f0f11]/90">
      <div className="px-1.5">
        <Wordmark />
      </div>

      <div className="mt-7 px-2 text-[9.5px] font-semibold uppercase tracking-[0.16em] text-zinc-400 dark:text-zinc-600">
        Workspace
      </div>
      <nav className="mt-1.5 space-y-0.5">
        {NAV_ITEMS.map(({ view: item, label, icon: Icon }) => {
          const activeItem = view === item;
          return (
            <button
              key={item}
              onClick={() => setView(item)}
              className={cn(
                "focus-ring group flex h-9 w-full items-center gap-2.5 rounded-[9px] px-2.5 text-[12.5px] font-medium transition-all duration-150",
                activeItem
                  ? "bg-zinc-900 text-white shadow-[0_1px_2px_rgba(0,0,0,0.12)] dark:bg-white/[0.11] dark:text-white dark:shadow-none"
                  : "text-zinc-500 hover:bg-zinc-100/80 hover:text-zinc-800 dark:text-zinc-400 dark:hover:bg-white/[0.045] dark:hover:text-zinc-200",
              )}
            >
              <Icon
                className={cn(
                  "size-4 shrink-0",
                  activeItem
                    ? "text-white dark:text-zinc-100"
                    : "text-zinc-400 group-hover:text-zinc-600 dark:text-zinc-500 dark:group-hover:text-zinc-300",
                )}
                strokeWidth={1.8}
              />
              <span className="truncate">{label}</span>
              {activeItem && (
                <ChevronRight className="ml-auto size-3 opacity-50" />
              )}
            </button>
          );
        })}
      </nav>

      <div className="mt-6 rounded-xl border border-zinc-200/70 bg-white/70 p-3 dark:border-white/[0.07] dark:bg-white/[0.025]">
        <div className="flex items-center justify-between gap-2">
          <div className="flex items-center gap-1.5">
            <span
              className={cn(
                "size-1.5 rounded-full",
                engineOn && status.hooksActive
                  ? "animate-pulse-dot bg-emerald-500"
                  : status.panicActive
                    ? "bg-rose-500"
                    : "bg-zinc-300 dark:bg-zinc-600",
              )}
            />
            <span className="text-[11px] font-semibold">
              {status.panicActive
                ? "Emergency stop"
                : engineOn
                  ? "Engine on"
                  : "Engine off"}
            </span>
          </div>
          <Switch
            size="sm"
            checked={engineOn}
            disabled={busy}
            onChange={toggle}
            label="Enable mappings"
          />
        </div>
        <button
          onClick={() => setView("profiles")}
          title={
            autoApp
              ? `Profile selected automatically for ${autoApp}`
              : "Active profile"
          }
          className="focus-ring mt-2 flex w-full items-center gap-1.5 rounded text-left text-[11px] text-zinc-400 transition hover:text-zinc-700 dark:text-zinc-500 dark:hover:text-zinc-300"
        >
          <Layers className="size-3 shrink-0" />
          <span className="truncate">
            {profile?.name ?? "No profile"}
            {autoApp ? ` · ${autoApp}` : ""}
          </span>
          <ChevronRight className="ml-auto size-3" />
        </button>
      </div>

      <div className="mt-auto space-y-2 px-1.5 pt-4">
        <div className="flex items-center justify-between text-[10px] text-zinc-400 dark:text-zinc-600">
          <span>MouseMacro {status.version}</span>
          <span className="flex items-center gap-1">
            <span
              className={cn(
                "size-1 rounded-full",
                status.hooksActive ? "bg-emerald-500/80" : "bg-amber-500",
              )}
            />
            {status.hooksActive ? "Hooks ready" : "Hook offline"}
          </span>
        </div>
        <div className="text-[9.5px] leading-relaxed text-zinc-300 dark:text-zinc-700">
          Local input engine · Windows
        </div>
      </div>
    </aside>
  );
}
