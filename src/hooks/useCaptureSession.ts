import { useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api } from "../lib/api";
import type { CaptureEvent, CaptureKind } from "../lib/types";

const MAX_EVENTS = 64;
/** Grace period on top of the backend's own watchdog. */
const FRONTEND_TIMEOUT_MS = 21_000;

interface CaptureEventMessage {
  sessionId: number;
  event: CaptureEvent;
}

interface CaptureEndedMessage {
  sessionId: number;
  reason: string;
}

/**
 * Drives a backend capture session. The backend buffers every event, so events
 * that arrive before this hook's listener is attached are returned by
 * `start_capture` and merged in instead of being lost.
 *
 * `kind` is the source of truth: set it to start, clear it to stop.
 */
export function useCaptureSession(kind: CaptureKind | null, onEnded: () => void) {
  const [events, setEvents] = useState<CaptureEvent[]>([]);
  const [ready, setReady] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const endedRef = useRef(onEnded);
  endedRef.current = onEnded;
  const sessionRef = useRef<number | null>(null);

  useEffect(() => {
    if (!kind) {
      sessionRef.current = null;
      setReady(false);
      setError(null);
      // Keep the last events so the editor can show what was recorded after
      // Stop. They are cleared when the next session starts.
      return;
    }

    let cancelled = false;
    let unEvent: UnlistenFn | undefined;
    let unEnded: UnlistenFn | undefined;
    let lastSeq = 0;
    const early: CaptureEventMessage[] = [];

    const merge = (incoming: CaptureEvent[]) => {
      if (cancelled || incoming.length === 0) return;
      setEvents((current) => {
        const bySeq = new Map<number, CaptureEvent>();
        for (const event of current) bySeq.set(event.seq, event);
        for (const event of incoming) bySeq.set(event.seq, event);
        const merged = [...bySeq.values()]
          .sort((left, right) => left.seq - right.seq)
          .slice(-MAX_EVENTS);
        lastSeq = merged.length > 0 ? merged[merged.length - 1].seq : 0;
        return merged;
      });
    };

    const start = async () => {
      setEvents([]);
      setReady(false);
      setError(null);
      lastSeq = 0;
      try {
        // Listen before starting so the backend's first events are delivered.
        unEvent = await listen<CaptureEventMessage>("capture://event", (message) => {
          if (cancelled) return;
          if (sessionRef.current === null) {
            early.push(message.payload);
            return;
          }
          if (message.payload.sessionId !== sessionRef.current) return;
          merge([message.payload.event]);
        });
        unEnded = await listen<CaptureEndedMessage>("capture://ended", (message) => {
          if (cancelled) return;
          // Ignore endings for a replaced session, including one emitted while
          // start_capture is still returning.
          if (
            sessionRef.current === null ||
            message.payload.sessionId !== sessionRef.current
          ) {
            return;
          }
          sessionRef.current = null;
          setReady(false);
          endedRef.current();
        });

        const info = await api.startCapture(kind);
        if (cancelled) {
          void api.stopCapture(info.id).catch(() => undefined);
          return;
        }
        sessionRef.current = info.id;
        const late = early
          .filter((message) => message.sessionId === info.id)
          .map((message) => message.event);
        merge([...info.events, ...late]);
        setReady(true);
      } catch (caught) {
        if (cancelled) return;
        setReady(false);
        setError(
          typeof caught === "string"
            ? caught
            : caught instanceof Error
              ? caught.message
              : "Could not start recording.",
        );
        sessionRef.current = null;
        endedRef.current();
      }
    };

    void start();

    // Safety net: recover events if an event ever fails to arrive.
    const poll = window.setInterval(() => {
      const id = sessionRef.current;
      if (id === null) return;
      void api
        .getCaptureEvents(id, lastSeq)
        .then((missing) => merge(missing))
        .catch(() => undefined);
    }, 1_000);

    const watchdog = window.setTimeout(() => {
      const id = sessionRef.current;
      sessionRef.current = null;
      setReady(false);
      if (id !== null) void api.stopCapture(id).catch(() => undefined);
      endedRef.current();
    }, FRONTEND_TIMEOUT_MS);

    return () => {
      cancelled = true;
      window.clearInterval(poll);
      window.clearTimeout(watchdog);
      unEvent?.();
      unEnded?.();
      const id = sessionRef.current;
      sessionRef.current = null;
      if (id !== null) void api.stopCapture(id).catch(() => undefined);
    };
  }, [kind]);

  // Stop on Escape. Keyboard sessions swallow Escape in the backend, so this
  // mainly covers mouse sessions and a hook that failed to install.
  useEffect(() => {
    if (!kind) return;
    const handler = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      event.stopPropagation();
      const id = sessionRef.current;
      sessionRef.current = null;
      setReady(false);
      if (id !== null) void api.stopCapture(id).catch(() => undefined);
      endedRef.current();
    };
    window.addEventListener("keydown", handler, true);
    return () => window.removeEventListener("keydown", handler, true);
  }, [kind]);

  return { active: kind !== null, ready, events, error };
}
