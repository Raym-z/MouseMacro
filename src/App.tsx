import { AlertTriangle, RefreshCw } from "lucide-react";
import { useApp } from "./state/AppContext";
import { Sidebar } from "./components/Sidebar";
import { LogoMark } from "./components/Logo";
import { Button } from "./components/Button";
import { DashboardPage } from "./pages/Dashboard";
import { MappingsPage } from "./pages/Mappings";
import { LayersPage } from "./pages/Layers";
import { ProfilesPage } from "./pages/Profiles";
import { ExceptionsPage } from "./pages/Exceptions";
import { SettingsPage } from "./pages/Settings";
import { AboutPage } from "./pages/About";
import { Onboarding } from "./components/Onboarding";

export function App() {
  const { snapshot, loading, fatalError, view } = useApp();

  if (loading) {
    return (
      <div className="flex h-full items-center justify-center bg-[#f5f5f6] dark:bg-[#0c0c0e]">
        <div className="flex flex-col items-center">
          <LogoMark className="size-11 rounded-[14px] [&_svg]:size-6" />
          <div className="mt-4 text-[12.5px] font-medium text-zinc-500 dark:text-zinc-400">
            Starting input engine…
          </div>
          <div className="mt-3 h-0.5 w-24 overflow-hidden rounded-full bg-zinc-200 dark:bg-white/10">
            <div className="animate-pulse-dot h-full w-1/2 rounded-full bg-zinc-900 dark:bg-white" />
          </div>
        </div>
      </div>
    );
  }

  if (fatalError || !snapshot) {
    return (
      <div className="flex h-full items-center justify-center bg-[#f5f5f6] p-8 dark:bg-[#0c0c0e]">
        <div className="surface max-w-[460px] rounded-2xl p-6 text-center">
          <div className="mx-auto grid size-11 place-items-center rounded-2xl bg-rose-500/10 text-rose-500">
            <AlertTriangle className="size-5" />
          </div>
          <h1 className="mt-4 text-[16px] font-semibold">
            MouseMacro could not start
          </h1>
          <p className="mt-2 text-[12.5px] leading-relaxed text-secondary">
            {fatalError ??
              "The application state could not be loaded. Close MouseMacro and try again."}
          </p>
          <Button
            className="mt-5"
            variant="primary"
            icon={<RefreshCw />}
            onClick={() => window.location.reload()}
          >
            Retry
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-full overflow-hidden bg-[#f5f5f6] text-zinc-900 dark:bg-[#0c0c0e] dark:text-zinc-100">
      <Sidebar />
      <main className="relative min-w-0 flex-1 overflow-y-auto">
        <div className="pointer-events-none absolute inset-x-0 top-0 h-52 bg-gradient-to-b from-white/70 to-transparent dark:from-white/[0.025]" />
        <div className="relative mx-auto w-full max-w-[1120px] px-8 py-8 pb-16">
          {view === "dashboard" && <DashboardPage />}
          {view === "mappings" && <MappingsPage />}
          {view === "layers" && <LayersPage />}
          {view === "profiles" && <ProfilesPage />}
          {view === "exceptions" && <ExceptionsPage />}
          {view === "settings" && <SettingsPage />}
          {view === "about" && <AboutPage />}
        </div>
      </main>
      <Onboarding />
    </div>
  );
}
