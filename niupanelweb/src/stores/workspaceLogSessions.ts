import { shallowReactive } from "vue";
import { defineStore } from "pinia";
import * as taskApi from "@/api/tasks";

type LogSubscriber = (content: string) => void;

type WorkspaceLogSession = {
  key: string;
  taskId: number;
  runId: number | null;
  buffer: string[];
  eventSource: EventSource | null;
  subscribers: Set<LogSubscriber>;
  closeTimer: ReturnType<typeof setTimeout> | null;
  status: "idle" | "streaming" | "closed" | "error";
};

const MAX_BUFFER_LINES = 3000;

const getLogEventContent = (payload: string) => {
  try {
    const parsed: unknown = JSON.parse(payload);
    if (typeof parsed === "string") return parsed;
    if (
      parsed &&
      typeof parsed === "object" &&
      "content" in parsed &&
      typeof parsed.content === "string"
    ) {
      return parsed.content;
    }
  } catch {
    // Plain text fallback for legacy event payloads.
  }

  return payload;
};

export const useWorkspaceLogSessionStore = defineStore(
  "workspace-log-sessions",
  () => {
    const sessions = shallowReactive(new Map<string, WorkspaceLogSession>());

    const createSession = (
      key: string,
      taskId: number,
      runId: number | null,
    ): WorkspaceLogSession => ({
      key,
      taskId,
      runId,
      buffer: [],
      eventSource: null,
      subscribers: new Set(),
      closeTimer: null,
      status: "idle",
    });

    const ensureSession = (
      key: string,
      taskId: number,
      runId: number | null,
    ) => {
      let session = sessions.get(key);
      if (!session) {
        session = createSession(key, taskId, runId);
        sessions.set(key, session);
      }

      if (session.runId !== runId) {
        session.eventSource?.close();
        session.eventSource = null;
        session.buffer = [];
        session.runId = runId;
        session.status = "idle";
      }

      return session;
    };

    const append = (session: WorkspaceLogSession, content: string) => {
      session.buffer.push(content);
      if (session.buffer.length > MAX_BUFFER_LINES) {
        session.buffer.splice(0, session.buffer.length - MAX_BUFFER_LINES);
      }
      session.subscribers.forEach((subscriber) => subscriber(content));
    };

    const connectLive = (
      key: string,
      taskId: number,
      runId: number | null,
    ) => {
      const session = ensureSession(key, taskId, runId);
      if (session.eventSource) return session;

      session.status = "streaming";
      const eventSource = runId
        ? taskApi.streamTaskRunLogs(taskId, runId)
        : taskApi.streamTaskLogs(taskId);

      eventSource.addEventListener("log", (event: MessageEvent) => {
        append(session, getLogEventContent(event.data));
      });
      eventSource.addEventListener("history", (event: MessageEvent) => {
        session.buffer = [event.data];
        session.subscribers.forEach((subscriber) => subscriber(event.data));
      });
      eventSource.onmessage = (event: MessageEvent) => {
        append(session, getLogEventContent(event.data));
      };
      eventSource.onerror = () => {
        session.status = "error";
        eventSource.close();
        session.eventSource = null;
      };

      session.eventSource = eventSource;
      return session;
    };

    const subscribe = (
      key: string,
      taskId: number,
      runId: number | null,
      subscriber: LogSubscriber,
    ) => {
      const session = ensureSession(key, taskId, runId);
      if (session.closeTimer) {
        clearTimeout(session.closeTimer);
        session.closeTimer = null;
      }

      session.subscribers.add(subscriber);
      session.buffer.forEach((line) => subscriber(line));

      return () => {
        session.subscribers.delete(subscriber);
        if (session.subscribers.size > 0) return;

        session.closeTimer = setTimeout(() => {
          if (session.subscribers.size > 0) return;
          session.eventSource?.close();
          session.eventSource = null;
          session.status = "closed";
        }, 45_000);
      };
    };

    const closeSession = (key: string) => {
      const session = sessions.get(key);
      if (!session) return;

      session.eventSource?.close();
      if (session.closeTimer) clearTimeout(session.closeTimer);
      sessions.delete(key);
    };

    return {
      closeSession,
      connectLive,
      sessions,
      subscribe,
    };
  },
);
