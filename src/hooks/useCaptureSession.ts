import { useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api } from "../lib/api";
import { isModifierToken, tokenFromKeyboardEvent } from "../lib/keys";
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
export function useCaptureSession(
  kind: CaptureKind | null,
  onEnded: () => void,
  onLocalKey?: (keys: string[]) => void,
) {
  const [events, setEvents] = useState<CaptureEvent[]>([]);
  const [ready, setReady] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const endedRef = useRef(onEnded);
  endedRef.current = onEnded;
  const localRef = useRef(onLocalKey);
  localRef.current = onLocalKey;
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
    let finished = false;
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

    const finish = () => {
      if (cancelled || finished) return;
      finished = true;
      sessionRef.current = null;
      setReady(false);
      endedRef.current();
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
          if (cancelled || finished) return;
          // Ignore endings for a replaced session, including one emitted while
          // start_capture is still returning.
          if (
            sessionRef.current === null ||
            message.payload.sessionId !== sessionRef.current
          ) {
            return;
          }
          finish();
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

    // Safety net: recover events if an event ever fails to arrive, and notice
    // a session the backend ended without the UI hearing about it.
    const poll = window.setInterval(() => {
      const id = sessionRef.current;
      if (id === null || finished) return;
      void Promise.all([
        api.getCaptureEvents(id, lastSeq).catch(() => [] as CaptureEvent[]),
        api.activeCaptureSession().catch(() => null),
      ]).then(([missing, active]) => {
        if (cancelled || finished || sessionRef.current !== id) return;
        merge(missing);
        if (active === null || active !== id) {
          finish();
        }
      });
    }, 1_000);

    const watchdog = window.setTimeout(() => {
      const id = sessionRef.current;
      if (id !== null) void api.stopCapture(id).catch(() => undefined);
      finish();
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
      event.stopImmediatePropagation();
      const id = sessionRef.current;
      sessionRef.current = null;
      setReady(false);
      if (id !== null) void api.stopCapture(id).catch(() => undefined);
      endedRef.current();
    };
    window.addEventListener("keydown", handler, true);
    return () => window.removeEventListener("keydown", handler, true);
  }, [kind]);

  // Fallback for when the low-level hook does not see the keyboard: while a
  // keyboard session runs, the focused window still receives the keys. If the
  // hook works it swallows them and this listener never fires; if it does not,
  // the key is still recorded and Space/Enter cannot activate Stop.
  useEffect(() => {
    if (kind !== "keyboard") return;
    const handler = (event: KeyboardEvent) => {
      if (sessionRef.current === null) return;
      if (event.key === "Escape") return;
      event.preventDefault();
      event.stopImmediatePropagation();
      if (event.repeat) return;
      const token = tokenFromKeyboardEvent(event);
      if (!token || isModifierToken(token)) return;
      const modifiers = [
        event.ctrlKey ? "ctrl" : null,
        event.shiftKey ? "shift" : null,
        event.altKey ? "alt" : null,
        event.metaKey ? "win" : null,
      ].filter((modifier): modifier is string => modifier !== null);
      localRef.current?.([...modifiers, token]);
    };
    window.addEventListener("keydown", handler, true);
    return () => window.removeEventListener("keydown", handler, true);
  }, [kind]);

  return { active: kind !== null, ready, events, error };
}
