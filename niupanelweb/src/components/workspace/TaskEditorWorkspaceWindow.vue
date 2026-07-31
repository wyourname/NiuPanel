<template>
  <div class="h-full min-h-0 overflow-hidden bg-card">
    <TaskWizard
      v-if="mode === 'create' || (mode === 'edit' && task)"
      :key="mode === 'create' ? 'task-create' : `task-edit-${task?.id}`"
      class="h-full"
      :initial-data="wizardInitialData"
      @success="handleTaskWizardSuccess"
    />

    <TaskDesktopScriptWorkspace
      v-else-if="mode === 'script' && task"
      v-model:content="scriptContent"
      :language="scriptLanguage"
      :loading="scriptLoading"
      :options="scriptEditorOptions"
      :saving="scriptSaving"
      :task="task"
      @editor-mount="handleScriptEditorMount"
      @save="saveScriptContent"
    />

    <TaskVariableEditor
      v-else-if="mode === 'variables' && task"
      :key="`task-variables-${task.id}`"
      :task-id="task.id"
      @success="handleVariablesSaved"
    />

    <div v-else-if="mode === 'cron' && task" class="flex h-full min-h-0 flex-col bg-card">
      <div class="min-h-0 flex-1 overflow-y-auto px-5 py-4 custom-scrollbar">
        <section class="space-y-4 border-b border-light/80 pb-5">
          <span class="label-sm">定时表达式 (Cron)</span>
          <CronInput v-model="cronInput" />
        </section>

        <section class="space-y-5 py-5">
          <div class="flex items-center justify-between gap-4">
            <div class="flex items-center gap-2">
              <div class="i-ep-magic-stick text-lg text-primary"></div>
              <span class="label-sm font-bold">随机运行模式</span>
            </div>
            <el-switch v-model="enableRandom" />
          </div>

          <transition name="el-fade-in">
            <div
              v-if="enableRandom"
              class="rounded-lg border border-light/80 bg-base/40 p-4 dark:bg-white/[0.025]"
            >
              <div class="grid gap-4 sm:grid-cols-2">
                <div>
                  <span class="mb-2 block text-[11px] font-semibold text-secondary">
                    开始时间
                  </span>
                  <el-time-picker
                    v-model="randomStart"
                    class="!w-full"
                    format="HH:mm"
                    placeholder="开始时间"
                    value-format="HH:mm"
                  />
                </div>
                <div>
                  <span class="mb-2 block text-[11px] font-semibold text-secondary">
                    结束时间
                  </span>
                  <el-time-picker
                    v-model="randomEnd"
                    class="!w-full"
                    format="HH:mm"
                    placeholder="结束时间"
                    value-format="HH:mm"
                  />
                </div>
              </div>

              <div class="mt-4">
                <span class="mb-2 block text-[11px] font-semibold text-secondary">
                  每日运行次数
                </span>
                <el-input-number
                  v-model="randomCount"
                  :max="100"
                  :min="1"
                  class="!w-full"
                />
              </div>
            </div>
          </transition>
        </section>
      </div>

      <div class="flex h-12 shrink-0 items-center justify-end border-t border-light/80 px-4">
        <el-button
          type="primary"
          :loading="cronSaving"
          class="!h-8 !rounded-lg !px-4 font-bold"
          @click="saveCron"
        >
          保存规则
        </el-button>
      </div>
    </div>

    <div v-else class="h-full flex-center text-sm font-bold text-muted">
      任务不存在或已删除
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { ElMessage } from "element-plus";
import * as taskApi from "@/api/tasks";
import CronInput from "@/components/common/CronInput.vue";
import TaskDesktopScriptWorkspace from "@/components/tasks/TaskDesktopScriptWorkspace.vue";
import TaskVariableEditor from "@/components/tasks/TaskVariableEditor.vue";
import TaskWizard from "@/components/tasks/TaskWizard.vue";
import type { TaskScriptEditorRef } from "@/composables/taskPageTypes";
import { useTaskScriptEditorConfig } from "@/composables/useTaskScriptEditorConfig";
import { useTaskStore } from "@/stores/tasks";
import { useWorkspaceStore } from "@/stores/workspace";
import type { Task, TaskRandomConfig } from "@/types";
import type { TaskEditorWindowPayload } from "@/types/workspace";
import {
  isTaskScriptFileMode,
  readTaskScriptContent,
  writeTaskScriptContent,
} from "@/utils/taskScriptContent";

const props = defineProps<{
  payload: TaskEditorWindowPayload;
  windowId: string;
}>();

const taskStore = useTaskStore();
const workspace = useWorkspaceStore();
const task = ref<Task | null>(
  props.payload.task ? { ...props.payload.task } : null,
);
const mode = computed(() => props.payload.mode);

const wizardInitialData = computed(() => {
  if (mode.value === "edit") return task.value ?? undefined;
  if (!props.payload.uploadedFile) return undefined;

  return {
    scriptSourceMode: "upload" as const,
    uploadedFile: props.payload.uploadedFile,
  };
});

const currentTask = computed<Task | undefined>(() => task.value ?? undefined);
const currentScriptTask = ref<Task | null>(task.value);
const isFileMode = ref(false);
const editorWordWrap = ref(true);
const scriptContent = ref("");
const scriptLoading = ref(false);
const scriptSaving = ref(false);
const scriptEditorInstance = ref<TaskScriptEditorRef | null>(null);

const defaultRandomConfig = (): TaskRandomConfig => ({
  start: "09:00",
  end: "18:00",
  count: 3,
});

const cronInput = ref("");
const cronSaving = ref(false);
const enableRandom = ref(false);
const randomConfig = ref<TaskRandomConfig>(defaultRandomConfig());

const { scriptEditorOptions, scriptLanguage } = useTaskScriptEditorConfig({
  currentScriptTask,
  currentTask,
  editorWordWrap,
  isFileMode,
});

const resetCronEditor = (sourceTask: Task) => {
  cronInput.value = sourceTask.cron_schedule || "";
  enableRandom.value = Boolean(sourceTask.random_config);
  randomConfig.value = sourceTask.random_config
    ? { ...sourceTask.random_config }
    : defaultRandomConfig();
};

const loadScriptContent = async (sourceTask: Task) => {
  currentScriptTask.value = sourceTask;
  isFileMode.value = isTaskScriptFileMode(sourceTask);
  scriptLoading.value = true;

  try {
    scriptContent.value = await readTaskScriptContent(sourceTask);
  } catch {
    scriptContent.value = "";
    ElMessage.error("加载脚本失败");
  } finally {
    scriptLoading.value = false;
  }
};

watch(
  () => props.payload,
  (payload) => {
    task.value = payload.task ? { ...payload.task } : null;
    currentScriptTask.value = task.value;

    if (payload.mode === "script" && task.value) {
      void loadScriptContent(task.value);
    } else if (payload.mode === "cron" && task.value) {
      resetCronEditor(task.value);
    }
  },
  { immediate: true },
);

const randomStart = computed({
  get: () => randomConfig.value.start,
  set: (value: string) => {
    randomConfig.value = { ...randomConfig.value, start: value };
  },
});

const randomEnd = computed({
  get: () => randomConfig.value.end,
  set: (value: string) => {
    randomConfig.value = { ...randomConfig.value, end: value };
  },
});

const randomCount = computed({
  get: () => randomConfig.value.count,
  set: (value: number | undefined) => {
    randomConfig.value = { ...randomConfig.value, count: value || 1 };
  },
});

const handleScriptEditorMount = (editor: TaskScriptEditorRef) => {
  scriptEditorInstance.value = editor;
};

const saveScriptContent = async () => {
  if (!task.value) return;

  const content = scriptContent.value.replace(/\r\n/g, "\n");
  scriptSaving.value = true;

  try {
    await writeTaskScriptContent(task.value, content, isFileMode.value);
    if (!isFileMode.value) {
      task.value = { ...task.value, command: content };
    }
    ElMessage.success("保存成功");
    await taskStore.refreshTasks(true);
  } finally {
    scriptSaving.value = false;
  }
};

const saveCron = async () => {
  if (!task.value) return;

  cronSaving.value = true;

  try {
    const randomValue = enableRandom.value ? { ...randomConfig.value } : null;
    const cronValue = enableRandom.value ? "" : cronInput.value;

    await taskApi.updateTask(task.value.id, {
      cron_schedule: cronValue,
      random_config: randomValue,
    });
    task.value = {
      ...task.value,
      cron_schedule: cronValue,
      random_config: randomValue,
    };
    ElMessage.success("保存成功");
    await taskStore.refreshTasks(true);
  } finally {
    cronSaving.value = false;
  }
};

const handleVariablesSaved = () => {
  void taskStore.refreshTasks(true);
};

const handleTaskWizardSuccess = () => {
  void taskStore.refreshTasks(true);
  workspace.closeWindow(props.windowId);
};
</script>
