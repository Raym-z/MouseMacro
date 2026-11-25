import { useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api } from "../lib/api";
import type { CaptureMode, CapturePayload } from "../lib/types";

/**
 * Drives the backend capture mode and forwards the globally captured input.
 * The mode is always cleared when the caller stops listening.
 */
export function useCapture(
  mode: CaptureMode,
  onPayload: (payload: CapturePayload) => void,
  onEnded?: () => void,
  onError?: (message: string) => void,
) {
  const payloadRef = useRef(onPayload);
  const endedRef = useRef(onEnded);
  const errorRef = useRef(onError);
  payloadRef.current = onPayload;
  endedRef.current = onEnded;
  errorRef.current = onError;
  const [ready, setReady] = useState(false);

  useEffect(() => {
    if (mode === "none") {
      setReady(false);
      return;
    }

    let cancelled = false;
    const unlisteners: UnlistenFn[] = [];
    setReady(false);

    const start = async () => {
      try {
        const unCapture = await listen<CapturePayload>("input://capture", (event) => {
          if (!cancelled) payloadRef.current(event.payload);
        });
        unlisteners.push(unCapture);
        const unEnded = await listen("input://capture-ended", () => {
          if (!cancelled) endedRef.current?.();
        });
        unlisteners.push(unEnded);
        await api.setCaptureMode(mode);
        if (!cancelled) setReady(true);
      } catch (error) {
        if (!cancelled) {
          setReady(false);
          const message =
            typeof error === "string"
              ? error
              : error instanceof Error
                ? error.message
                : "Unknown error";
          errorRef.current?.(message);
          endedRef.current?.();
        }
      }
    };

    void start();

    return () => {
      cancelled = true;
      setReady(false);
      for (const unlisten of unlisteners) unlisten();
      void api.setCaptureMode("none").catch(() => undefined);
    };
  }, [mode]);

  return ready;
}
