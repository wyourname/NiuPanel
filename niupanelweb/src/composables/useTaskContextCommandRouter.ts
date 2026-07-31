import { type Ref } from "vue";
import { useTaskStore } from "../stores/tasks";
import type { TaskContextCommand, TaskDetailTab } from "./taskPageTypes";
import type { Task } from "@/types";

type UseTaskContextCommandRouterOptions = {
  activeDetailTab: Ref<TaskDetailTab>;
  handleDelete: (id: number) => void;
  handleEnterSelection: (task: Task) => void;
  openCronEditor: (task: Task) => void;
  openEdit: (task: Task) => void;
  openShare: (task: Task) => void;
  selectTask: (task: Task) => void;
  selectionMode: Ref<boolean>;
  taskStore: ReturnType<typeof useTaskStore>;
};

export function useTaskContextCommandRouter({
  activeDetailTab,
  handleDelete,
  handleEnterSelection,
  openCronEditor,
  openEdit,
  openShare,
  selectTask,
  selectionMode,
  taskStore,
}: UseTaskContextCommandRouterOptions) {
  const handleContextCommand = (command: TaskContextCommand, task: Task) => {
    if (selectionMode.value && command !== "select") return;

    if (command === "run") taskStore.runTask(task.id);
    else if (command === "stop") taskStore.stopTask(task.id);
    else if (command === "edit") openEdit(task);
    else if (command === "script") {
      selectTask(task);
      activeDetailTab.value = "script";
    } else if (command === "vars") {
      selectTask(task);
      activeDetailTab.value = "var";
    } else if (command === "cron") openCronEditor(task);
    else if (command === "share") openShare(task);
    else if (command === "select") handleEnterSelection(task);
    else if (command === "pin") {
      if (task.is_pinned) taskStore.unpinTask(task.id);
      else taskStore.pinTask(task.id);
    } else if (command === "delete") handleDelete(task.id);
  };

  return {
    handleContextCommand,
  };
}
