import type { Ref } from "vue";
import { ElMessage } from "element-plus";
import * as taskApi from "../api/tasks";
import type { Task } from "@/types";

type RefreshTasks = (silent?: boolean) => Promise<void>;

type TaskOperationContext = {
  loading: Ref<boolean>;
  refreshTasks: RefreshTasks;
};

type BatchTaskApi<Args extends unknown[]> = (
  ids: number[],
  ...args: Args
) => Promise<unknown>;

export const createTaskOperations = ({
  loading,
  refreshTasks,
}: TaskOperationContext) => {
  const refreshSilently = () => {
    void refreshTasks(true);
  };

  const runTask = async (id: number) => {
    try {
      await taskApi.runTasks([id]);
      ElMessage.success("任务已启动");
      refreshSilently();
    } catch (e) {}
  };

  const stopTask = async (id: number) => {
    try {
      await taskApi.stopTasks([id]);
      ElMessage.success("停止命令已发送");
      refreshSilently();
    } catch (e) {}
  };

  const pauseTask = async (id: number) => {
    try {
      await taskApi.pauseTasks([id]);
      ElMessage.success("任务已暂停");
      refreshSilently();
    } catch (e) {}
  };

  const resumeTask = async (id: number) => {
    try {
      await taskApi.resumeTasks([id]);
      ElMessage.success("任务已恢复");
      refreshSilently();
    } catch (e) {}
  };

  const deleteTask = async (
    id: number,
    deleteVar = false,
    deleteScript = false,
  ) => {
    try {
      await taskApi.deleteTasks([id], deleteVar, deleteScript);
      ElMessage.success("删除成功");
      refreshSilently();
      return true;
    } catch (e) {
      return false;
    }
  };

  const toggleEnable = async (task: Task, enabled: boolean) => {
    const originalState = !enabled;
    task.enabled = enabled;
    try {
      if (enabled) {
        await taskApi.enableTasks([task.id]);
        ElMessage.success("已启用");
      } else {
        await taskApi.disableTasks([task.id]);
        ElMessage.success("已禁用");
      }
    } catch (e) {
      task.enabled = originalState;
    }
  };

  const pinTask = async (id: number) => {
    try {
      await taskApi.pinTasks([id]);
      refreshSilently();
    } catch (e) {}
  };

  const unpinTask = async (id: number) => {
    try {
      await taskApi.unpinTasks([id]);
      refreshSilently();
    } catch (e) {}
  };

  const executeBatch = async <Args extends unknown[]>(
    ids: number[],
    apiFunc: BatchTaskApi<Args>,
    successMsg = "操作成功",
    ...args: Args
  ) => {
    if (!ids.length) return;
    loading.value = true;
    try {
      await apiFunc(ids, ...args);
      ElMessage.success(successMsg);
      refreshSilently();
    } catch (e) {
    } finally {
      loading.value = false;
    }
  };

  const batchRun = (ids: number[]) =>
    executeBatch(ids, taskApi.runTasks, "批量启动成功");
  const batchStop = (ids: number[]) =>
    executeBatch(ids, taskApi.stopTasks, "批量停止成功");
  const batchPause = (ids: number[]) =>
    executeBatch(ids, taskApi.pauseTasks, "批量暂停成功");
  const batchResume = (ids: number[]) =>
    executeBatch(ids, taskApi.resumeTasks, "批量恢复成功");
  const batchEnable = (ids: number[]) =>
    executeBatch(ids, taskApi.enableTasks, "批量启用成功");
  const batchDisable = (ids: number[]) =>
    executeBatch(ids, taskApi.disableTasks, "批量禁用成功");
  const batchDelete = (
    ids: number[],
    deleteVar: boolean,
    deleteScript: boolean,
  ) =>
    executeBatch(
      ids,
      taskApi.deleteTasks,
      "批量删除成功",
      deleteVar,
      deleteScript,
    );
  const batchPin = (ids: number[]) =>
    executeBatch(ids, taskApi.pinTasks, "批量置顶成功");
  const batchUnpin = (ids: number[]) =>
    executeBatch(ids, taskApi.unpinTasks, "批量取消置顶成功");

  return {
    runTask,
    stopTask,
    pauseTask,
    resumeTask,
    deleteTask,
    toggleEnable,
    pinTask,
    unpinTask,
    batchRun,
    batchStop,
    batchPause,
    batchResume,
    batchEnable,
    batchDisable,
    batchDelete,
    batchPin,
    batchUnpin,
  };
};
