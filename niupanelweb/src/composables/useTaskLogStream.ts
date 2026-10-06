import { onScopeDispose, ref, type Ref } from "vue";
import { createLogConnection, type LogConnectionState } from "@/utils/logConnection";
import * as taskApi from "../api/tasks";
import type { TaskLogViewerRef } from "./taskPageTypes";
import type { Task } from "@/types";

type UseTaskLogStreamOptions = {
  activeLogTask: () => Task | undefined;
  logViewRef: Ref<TaskLogViewerRef | null>;
  selectedHistoryRunId: Ref<number | null>;
};

export function useTaskLogStream({
  activeLogTask,
  logViewRef,
  selectedHistoryRunId,
}: UseTaskLogStreamOptions) {
  let connection: ReturnType<typeof createLogConnection> | null = null;
  const logConnectionState = ref<LogConnectionState>('idle');
  const closeLogStream = () => { connection?.close(); connection = null; logConnectionState.value = 'idle'; };
  const retryLogConnection = () => connection?.retry();
  onScopeDispose(closeLogStream);

  const connectLogStream = () => {
    const task = activeLogTask();
    const id = task?.id;

    closeLogStream();
    if (!id || !logViewRef.value) return;

    if (selectedHistoryRunId.value) {
      const historyRunId = selectedHistoryRunId.value;
      logViewRef.value?.reset?.();
      logViewRef.value.init?.(async (offset: number, limit: number) => {
        const res = await taskApi.getTaskRunLog(
          id,
          historyRunId,
          offset,
          limit,
        );
        return res.data;
      });
      return;
    }

    if (task?.status === "Running" || task?.status === "Paused") {
      const viewer = logViewRef.value;
      viewer.reset?.();
      const runId = typeof task.run_id === "number" ? task.run_id : null;
      connection = createLogConnection({
        open: () => runId ? taskApi.streamTaskRunLogs(id, runId) : taskApi.streamTaskLogs(id),
        readMissing: async (offset, limit) => (await (runId ? taskApi.getTaskRunLog(id, runId, offset, limit) : taskApi.getLatestLog(id, offset, limit))).data,
        write: (content, reset) => { if (reset) viewer.reset?.(); viewer.write?.(content); },
        status: state => { logConnectionState.value = state; },
      });
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
    logConnectionState,
    retryLogConnection,
  };
}
