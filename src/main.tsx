import { Component, type ErrorInfo, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { ToastProvider } from "./components/Toast";
import { AppProvider } from "./state/AppContext";
import "./index.css";

class ErrorBoundary extends Component<
  { children: ReactNode },
  { error: Error | null }
> {
  state = { error: null as Error | null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("MouseMacro UI error", error, info);
  }

  render() {
    if (this.state.error) {
      return (
        <div className="flex h-full flex-col items-center justify-center gap-3 p-8 text-center">
          <div className="text-[15px] font-semibold">
            The interface hit an unexpected error
          </div>
          <div className="max-w-[520px] text-[12px] text-zinc-500">
            {this.state.error.message}
          </div>
          <button
            className="mt-2 rounded-lg bg-zinc-900 px-3 py-2 text-[12px] font-medium text-white dark:bg-white dark:text-zinc-900"
            onClick={() => window.location.reload()}
          >
            Reload interface
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

createRoot(document.getElementById("root")!).render(
  <ErrorBoundary>
    <ToastProvider>
      <AppProvider>
        <App />
      </AppProvider>
    </ToastProvider>
  </ErrorBoundary>,
);
