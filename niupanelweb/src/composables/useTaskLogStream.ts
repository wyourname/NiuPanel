import { type Ref } from "vue";
import * as taskApi from "../api/tasks";
import type { TaskLogViewerRef } from "./taskPageTypes";
import type { Task } from "@/types";

type UseTaskLogStreamOptions = {
  activeLogTask: () => Task | undefined;
  logViewRef: Ref<TaskLogViewerRef | null>;
  selectedHistoryRunId: Ref<number | null>;
};

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

export function useTaskLogStream({
  activeLogTask,
  logViewRef,
  selectedHistoryRunId,
}: UseTaskLogStreamOptions) {
  let logEventSource: EventSource | null = null;

  const closeLogStream = () => {
    if (logEventSource) {
      logEventSource.close();
      logEventSource = null;
    }
  };

  const connectLogStream = () => {
    const task = activeLogTask();
    const id = task?.id;

    if (!id || !logViewRef.value) return;
    closeLogStream();

    if (selectedHistoryRunId.value) {
      logViewRef.value?.reset?.();
      logViewRef.value.init?.(async (offset: number, limit: number) => {
        const res = await taskApi.getTaskRunLog(
          id,
          selectedHistoryRunId.value!,
          offset,
          limit,
        );
        return res.data;
      });
      return;
    }

    if (task?.status === "Running") {
      logViewRef.value?.reset?.();
      const runId = typeof task.run_id === "number" ? task.run_id : null;
      logEventSource = runId
        ? taskApi.streamTaskRunLogs(id, runId)
        : taskApi.streamTaskLogs(id);
      logEventSource.addEventListener("log", (event: MessageEvent) => {
        logViewRef.value?.write?.(getLogEventContent(event.data));
      });
      logEventSource.addEventListener("history", (event: MessageEvent) => {
        logViewRef.value?.reset?.();
        logViewRef.value?.write?.(event.data);
      });
      logEventSource.onmessage = (event: MessageEvent) => {
        logViewRef.value?.write?.(getLogEventContent(event.data));
      };
      logEventSource.onerror = () => {
        closeLogStream();
      };
      return;
    }

    logViewRef.value.init?.(async (offset: number, limit: number) => {
      const res = await taskApi.getLatestLog(id, offset, limit);
      return res.data;
    });
  };

  return {
    closeLogStream,
    connectLogStream,
  };
}
