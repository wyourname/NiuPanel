import { type Ref } from "vue";
import { ElLoading } from "element-plus";
import * as taskApi from "../api/tasks";
import type { Task } from "@/types";

type UseTaskLogDownloadOptions = {
  activeLogTask: () => Task | undefined;
  selectedHistoryRunId: Ref<number | null>;
};

const stripAnsi = (content: string) =>
  content.replace(/\x1B\[[0-?]*[ -/]*[@-~]/g, "");

export function useTaskLogDownload({
  activeLogTask,
  selectedHistoryRunId,
}: UseTaskLogDownloadOptions) {
  const downloadLogs = async () => {
    const task = activeLogTask();
    if (!task) return;

    const loadingSvc = ElLoading.service({
      lock: true,
      text: "Fetching...",
      background: "rgba(0, 0, 0, 0.7)",
    });
    try {
      let content = "";
      if (selectedHistoryRunId.value) {
        const res = await taskApi.getTaskRunLog(
          task.id,
          selectedHistoryRunId.value,
          0,
          5 * 1024 * 1024,
        );
        content = stripAnsi(res.data.content || "");
      } else {
        const res = await taskApi.getLatestLog(task.id, 0, 5 * 1024 * 1024);
        content = stripAnsi(res.data.content || "");
      }

      const blob = new Blob([content], { type: "text/plain;charset=utf-8" });
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;

      const dateStr = new Date().toISOString().split("T")[0].replace(/-/g, "");
      const runIdStr = selectedHistoryRunId.value || "latest";
      link.download = `${task.name}_${dateStr}_${runIdStr}.log`;

      link.click();
      URL.revokeObjectURL(url);
    } finally {
      loadingSvc.close();
    }
  };

  return {
    downloadLogs,
  };
}
