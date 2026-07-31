import { type ComputedRef, type Ref } from "vue";
import { ElMessage } from "element-plus";
import { useTaskStore } from "../stores/tasks";
import type { Task } from "@/types";

type UseTaskBulkActionsOptions = {
  selectedIds: Ref<number[]>;
  selectedTasks: ComputedRef<Task[]>;
  shareVisible: Ref<boolean>;
  taskStore: ReturnType<typeof useTaskStore>;
  tasksToShare: Ref<Task[]>;
};

export function useTaskBulkActions({
  selectedIds,
  selectedTasks,
  shareVisible,
  taskStore,
  tasksToShare,
}: UseTaskBulkActionsOptions) {
  const clearSelectedIds = () => {
    selectedIds.value = [];
  };

  const handleBulkRun = async () => {
    const targets = selectedTasks.value.filter(
      (task) => task.status !== "Running",
    );
    if (targets.length === 0) return ElMessage.info("选中任务均已在运行");
    await taskStore.batchRun(targets.map((task) => task.id));
    clearSelectedIds();
  };

  const handleBulkPause = async () => {
    const targets = selectedTasks.value.filter(
      (task) => task.status === "Running",
    );
    if (targets.length === 0) return ElMessage.info("没有运行中的任务可暂停");
    await taskStore.batchPause(targets.map((task) => task.id));
    clearSelectedIds();
  };

  const handleBulkResume = async () => {
    const targets = selectedTasks.value.filter(
      (task) => task.status === "Paused",
    );
    if (targets.length === 0) return ElMessage.info("没有暂停中的任务可恢复");
    await taskStore.batchResume(targets.map((task) => task.id));
    clearSelectedIds();
  };

  const handleBulkStop = async () => {
    const targets = selectedTasks.value.filter((task) =>
      ["Running", "Paused"].includes(task.status),
    );
    if (targets.length === 0)
      return ElMessage.info("没有运行或暂停的任务可停止");
    await taskStore.batchStop(targets.map((task) => task.id));
    clearSelectedIds();
  };

  const handleBulkEnable = async () => {
    await taskStore.batchEnable(selectedIds.value);
  };

  const handleBulkDisable = async () => {
    await taskStore.batchDisable(selectedIds.value);
  };

  const handleBulkPin = async () => {
    await taskStore.batchPin(selectedIds.value);
    clearSelectedIds();
  };

  const handleBulkUnpin = async () => {
    await taskStore.batchUnpin(selectedIds.value);
    clearSelectedIds();
  };

  const handleBulkShare = () => {
    tasksToShare.value = [...selectedTasks.value];
    shareVisible.value = true;
  };

  return {
    handleBulkRun,
    handleBulkPause,
    handleBulkResume,
    handleBulkStop,
    handleBulkEnable,
    handleBulkDisable,
    handleBulkPin,
    handleBulkUnpin,
    handleBulkShare,
  };
}
