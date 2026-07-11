<template>
  <div class="w-full h-full flex flex-col">
    <div v-if="loading" class="flex-1 flex items-center justify-center">
      <div class="i-ep-loading animate-spin text-3xl text-primary opacity-50"></div>
    </div>

    <div
      v-else-if="!hasHistory"
      class="flex-1 flex flex-col items-center justify-center text-muted opacity-50"
    >
      <div class="i-ep-timer text-5xl mb-4"></div>
      <span class="text-sm">暂无{{ emptyRecordType }}记录</span>
    </div>

    <div v-else class="flex-1 overflow-hidden">
      <ShareTaskRunHistoryList
        v-if="isTaskHistoryMode"
        :is-mobile="appStore.isMobile"
        :runs="taskHistory"
        @view-log="viewLog"
      />
      <ShareImportSourceHistoryList
        v-else
        :groups="importHistory"
        :is-mobile="appStore.isMobile"
        @copy-url="handleCopyUrl"
        @delete="handleDelete"
        @update="handleUpdate"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import useClipboard from "vue-clipboard3";
import * as shareApi from "../../../../api/share";
import * as taskApi from "../../../../api/tasks";
import { useAppStore } from "../../../../stores/app";
import type {
  DeleteImportedTasksParams,
  ImportSourceGroup,
  TaskRunHistoryItem,
} from "@/types";
import ShareImportSourceHistoryList from "./ShareImportSourceHistoryList.vue";
import ShareTaskRunHistoryList from "./ShareTaskRunHistoryList.vue";

type DeleteTargetType = "task" | "share" | "source";

const props = withDefaults(
  defineProps<{
    taskId?: number | null;
  }>(),
  {
    taskId: null,
  },
);

const emit = defineEmits<{
  (event: "update", url: string): void;
  (event: "view-log", logPath: string, runId: number): void;
}>();

const appStore = useAppStore();
const { toClipboard } = useClipboard();

const importHistory = ref<ImportSourceGroup[]>([]);
const taskHistory = ref<TaskRunHistoryItem[]>([]);
const loading = ref(false);

const isTaskHistoryMode = computed(() => props.taskId !== null);
const hasHistory = computed(() =>
  isTaskHistoryMode.value
    ? taskHistory.value.length > 0
    : importHistory.value.length > 0,
);
const emptyRecordType = computed(() =>
  isTaskHistoryMode.value ? "运行" : "导入",
);

const fetchHistory = async () => {
  loading.value = true;
  try {
    if (isTaskHistoryMode.value && props.taskId !== null) {
      const res = await taskApi.getTaskHistory(props.taskId, 1, 50);
      taskHistory.value = res.data?.items || [];
      importHistory.value = [];
      return;
    }

    const res = await shareApi.getImportHistory();
    importHistory.value = res.data || [];
    taskHistory.value = [];
  } catch (error) {
    console.error(error);
    ElMessage.error("获取历史记录失败");
  } finally {
    loading.value = false;
  }
};

const handleUpdate = (url: string) => {
  emit("update", url);
};

const handleCopyUrl = async (url: string) => {
  try {
    await toClipboard(url);
    ElMessage.success("链接已复制");
  } catch {
    ElMessage.error("复制失败");
  }
};

const buildDeleteParams = (
  id: number | string,
  type: DeleteTargetType,
): DeleteImportedTasksParams => {
  if (type === "share") {
    return { share_code: String(id) };
  }
  if (type === "source") {
    return { import_source: String(id) };
  }
  return { task_id: Number(id) };
};

const getDeleteConfirmMessage = (type: DeleteTargetType) =>
  type === "task"
    ? "确定要从记录中删除此导入任务吗？"
    : "确定要删除此分享记录及其包含的所有任务吗？此操作不可撤销。";

const isConfirmCancel = (error: unknown) =>
  error === "cancel" || error === "close";

const getErrorMessage = (error: unknown, fallback: string) =>
  error instanceof Error ? error.message : fallback;

const handleDelete = async (
  id: number | string,
  type: DeleteTargetType = "task",
) => {
  try {
    await ElMessageBox.confirm(getDeleteConfirmMessage(type), "删除确认", {
      confirmButtonText: "删除",
      confirmButtonClass: "el-button--danger",
      type: "warning",
    });

    await shareApi.deleteImportedTasks(buildDeleteParams(id, type));
    ElMessage.success("已成功移除");
    await fetchHistory();
  } catch (error) {
    if (isConfirmCancel(error)) return;
    ElMessage.error(getErrorMessage(error, "删除失败"));
  }
};

const viewLog = (runId: number) => {
  const record = taskHistory.value.find((run) => run.id === runId);
  if (record?.log_path) {
    emit("view-log", record.log_path, record.id);
    return;
  }
  ElMessage.warning("该记录暂无日志文件");
};

watch(
  () => props.taskId,
  () => {
    void fetchHistory();
  },
  { immediate: true },
);

defineExpose({ refresh: fetchHistory });
</script>

<style scoped>
:deep(.el-table__row) {
  transition: background-color 0.2s;
}

:deep(.el-table__row:hover) {
  background-color: var(--bg-hover) !important;
}
</style>
