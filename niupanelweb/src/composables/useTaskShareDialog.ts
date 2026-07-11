import {
  nextTick,
  reactive,
  ref,
  toValue,
  watch,
  type MaybeRefOrGetter,
  type Ref,
} from "vue";
import { ElMessage } from "element-plus";
import useClipboard from "vue-clipboard3";
import * as shareApi from "../api/share";
import type { FileNode, Task, TaskExportSpec } from "@/types";
import type {
  CheckedFileOption,
  TaskShareFileSelection,
  TaskShareFileSelectorExpose,
} from "@/components/common/taskShareDialogTypes";

type UseTaskShareDialogOptions = {
  onShared: () => void;
  tasks: MaybeRefOrGetter<Task[]>;
  visible: Ref<boolean>;
};

const createDefaultShareSelection = (task: Task): TaskShareFileSelection => ({
  main: task.path || null,
  deps: new Set(),
});

export function useTaskShareDialog({
  onShared,
  tasks,
  visible,
}: UseTaskShareDialogOptions) {
  const { toClipboard } = useClipboard();

  const activeStep = ref(0);
  const includeEnvs = ref(false);
  const showAdvanced = ref(false);

  const loadingFileTree = ref(false);
  const fileSelectorRef = ref<TaskShareFileSelectorExpose | null>(null);
  const currentTreeData = ref<FileNode[]>([]);

  const taskFiles = reactive<Record<number, TaskShareFileSelection>>({});
  const currentEditingTask = ref<Task | null>(null);
  const currentMainFile = ref("");
  const currentCheckedFiles = ref<CheckedFileOption[]>([]);

  const sharePassword = ref("");
  const expiresHours = ref(720);
  const maxUses = ref(999);
  const burnAfterReading = ref(false);
  const shareNote = ref("");
  const shareLink = ref("");
  const loadingGenerate = ref(false);

  const resetDialog = () => {
    activeStep.value = 0;
    includeEnvs.value = false;
    currentEditingTask.value = null;

    Object.keys(taskFiles).forEach((key) => delete taskFiles[Number(key)]);

    shareLink.value = "";
    loadingGenerate.value = false;

    showAdvanced.value = false;
    sharePassword.value = "";
    expiresHours.value = 720;
    maxUses.value = 999;
    burnAfterReading.value = false;
    shareNote.value = "";
    currentMainFile.value = "";
    currentCheckedFiles.value = [];
  };

  const loadFileTree = async () => {
    loadingFileTree.value = true;
    try {
      const allTaskIds = toValue(tasks).map((task) => task.id);
      const res = await shareApi.getTaskFileTree(allTaskIds);
      currentTreeData.value = res.data.tree;
    } catch (error) {
      ElMessage.error("加载文件树失败");
      console.error(error);
    } finally {
      loadingFileTree.value = false;
    }
  };

  const handleTreeCheck = () => {
    if (!fileSelectorRef.value) return;

    const checkedNodes = fileSelectorRef.value.getCheckedNodes();
    currentCheckedFiles.value = checkedNodes
      .filter((node) => !node.is_dir)
      .map((node) => ({
        label: node.name,
        value: node.path,
      }));

    const mainFileStillChecked = currentCheckedFiles.value.some(
      (file) => file.value === currentMainFile.value,
    );

    if (currentMainFile.value && !mainFileStillChecked) {
      currentMainFile.value = "";
    }
    if (!currentMainFile.value && currentCheckedFiles.value.length > 0) {
      currentMainFile.value = currentCheckedFiles.value[0].value;
    }
  };

  const startEdit = (task: Task) => {
    currentEditingTask.value = task;
    const taskData = taskFiles[task.id] || createDefaultShareSelection(task);
    currentMainFile.value = taskData.main || "";

    nextTick(() => {
      if (!fileSelectorRef.value) return;

      const keys = Array.from(taskData.deps || []);
      if (taskData.main) keys.push(taskData.main);
      fileSelectorRef.value.setCheckedKeys(keys);
      handleTreeCheck();
    });
  };

  const stopEdit = () => {
    if (fileSelectorRef.value && currentEditingTask.value) {
      const checkedNodes = fileSelectorRef.value.getCheckedNodes();
      const filesOnly = checkedNodes
        .filter((node) => !node.is_dir)
        .map((node) => node.path);

      const main = currentMainFile.value;
      const deps = new Set(filesOnly.filter((file) => file !== main));

      taskFiles[currentEditingTask.value.id] = { main, deps };
    }
    currentEditingTask.value = null;
    currentMainFile.value = "";
    currentCheckedFiles.value = [];
  };

  const selectAllFiles = () => {
    if (!fileSelectorRef.value || currentTreeData.value.length === 0) return;
    fileSelectorRef.value.setCheckedNodes(currentTreeData.value);
    handleTreeCheck();
  };

  const initializeDialog = async () => {
    resetDialog();

    const currentTasks = toValue(tasks);
    currentTasks.forEach((task) => {
      taskFiles[task.id] = createDefaultShareSelection(task);
    });

    await loadFileTree();

    if (currentTasks.length === 1) {
      startEdit(currentTasks[0]);
    }
  };

  const handleGenerateToken = async () => {
    const tasksToExport: TaskExportSpec[] = [];

    toValue(tasks).forEach((task) => {
      const data = taskFiles[task.id];
      if (data?.main) {
        tasksToExport.push({
          task_id: task.id,
          files: [
            {
              main_file: data.main,
              dependencies: Array.from(data.deps),
            },
          ],
          include_envs: includeEnvs.value,
        });
      }
    });

    if (tasksToExport.length === 0) {
      ElMessage.warning("请至少为一个任务关联主文件");
      return;
    }

    loadingGenerate.value = true;
    try {
      const res = await shareApi.createShare({
        tasks: tasksToExport,
        password: sharePassword.value || null,
        max_uses: maxUses.value,
        expires_in_hours: expiresHours.value,
        burn_after_reading: burnAfterReading.value,
        note: shareNote.value || `Shared ${tasksToExport.length} tasks`,
      });

      shareLink.value = res.data.link;
      activeStep.value = 1;
      onShared();
    } catch (error) {
      ElMessage.error(
        `生成失败: ${error instanceof Error ? error.message : "未知错误"}`,
      );
      console.error(error);
    } finally {
      loadingGenerate.value = false;
    }
  };

  const copyLink = async () => {
    try {
      await toClipboard(shareLink.value);
      ElMessage.success("链接已复制");
    } catch {
      ElMessage.error("复制失败");
    }
  };

  watch(
    visible,
    async (isVisible) => {
      if (isVisible) {
        await initializeDialog();
      }
    },
    { immediate: true },
  );

  return {
    activeStep,
    burnAfterReading,
    copyLink,
    currentCheckedFiles,
    currentEditingTask,
    currentMainFile,
    currentTreeData,
    expiresHours,
    fileSelectorRef,
    handleGenerateToken,
    handleTreeCheck,
    includeEnvs,
    loadingFileTree,
    loadingGenerate,
    maxUses,
    selectAllFiles,
    shareLink,
    shareNote,
    sharePassword,
    showAdvanced,
    startEdit,
    stopEdit,
    taskFiles,
  };
}
