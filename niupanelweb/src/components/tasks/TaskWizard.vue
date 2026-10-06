<template>
  <div class="task-wizard flex min-h-0 flex-1 flex-col" :aria-busy="initializing || submitting">
    <div class="shrink-0 border-b border-light px-4 py-3 md:px-6">
      <div class="grid grid-cols-3 overflow-hidden rounded-md border border-light bg-base/60">
        <button
          v-for="(step, index) in stepItems"
          type="button"
          :disabled="initializing || submitting || (!isEdit && index > activeStep)"
          :aria-current="index === activeStep ? 'step' : undefined"
          @click="activeStep = index"
          :key="step.label"
          class="flex min-h-11 min-w-0 items-center justify-center gap-2 border-r border-light px-2 py-2.5 last:border-r-0 disabled:cursor-default focus-visible:outline-2 focus-visible:outline-primary"
          :class="index === activeStep ? 'bg-card text-primary' : index < activeStep ? 'text-default' : 'text-muted'"
        >
          <span
            class="h-6 w-6 shrink-0 rounded text-[11px] font-bold flex-center"
            :class="index <= activeStep ? 'accent-subtle' : 'bg-soft text-muted'"
          >
            <span v-if="index < activeStep" class="i-ep-check"></span>
            <span v-else>{{ index + 1 }}</span>
          </span>
          <span class="min-w-0 text-xs font-semibold">{{ step.label }}</span>
        </button>
      </div>
    </div>

    <!-- Content Area (Scrollable) -->
    <div class="task-wizard-content min-h-0 flex-1 overflow-x-hidden overflow-y-auto px-4 py-5 custom-scrollbar md:px-6"
      v-loading="initializing">
      <!-- Step 1: Script Config -->
      <TaskWizardScriptStep
        v-show="activeStep === 0"
        v-model:command="form.command"
        v-model:path="form.path"
        v-model:script-source-mode="scriptSourceMode"
        v-model:search-query="searchQuery"
        :browser-items="browserItems"
        :browser-loading="browserLoading"
        :current-path="currentPath"
        :path-parts="pathParts"
        :uploaded-file="uploadedFile"
        @browser-item-click="handleBrowserItemClick"
        @clear-uploaded-file="uploadedFile = null"
        @navigate="navigate"
        @navigate-up="navigateUp"
        @script-upload="handleScriptUpload"
      />

      <!-- Step 2: Basic Details -->
      <TaskWizardDetailsStep
        v-show="activeStep === 1"
        ref="detailsStepRef"
        v-model:cron-description="cronDescription"
        v-model:cron-valid="cronValid"
        :all-tasks="allTasks"
        :form="form"
        :initial-task-id="props.initialData?.id"
      />

      <!-- Step 3: Environment & Vars -->
      <TaskWizardEnvironmentStep
        v-show="activeStep === 2"
        v-model:variable-mode="variableMode"
        v-model:variables-bulk="variablesBulk"
        :form="form"
        :node-versions="nodeVersions"
        :python-versions="pythonVersions"
        :variables-list="variablesList"
      />
    </div>

    <!-- Footer -->
    <div
      class="flex shrink-0 items-center justify-between border-t border-light bg-card px-4 py-3 md:px-6"
    >
      <div class="flex gap-3">
        <ToolbarButton v-if="activeStep > 0 && !isEdit" :disabled="submitting" @click="activeStep--">
          <template #icon><div class="i-ep-arrow-left"></div></template>
          上一步
        </ToolbarButton>
        <ToolbarButton v-else :disabled="submitting" variant="soft" @click="emit('cancel')">取消</ToolbarButton>
      </div>

      <div class="flex gap-3">
        <ToolbarButton v-if="activeStep < 2 && !isEdit" :disabled="initializing || submitting" variant="primary" @click="handleNext">
          下一步
          <template #icon><div class="i-ep-arrow-right"></div></template>
        </ToolbarButton>
        <ToolbarButton
          v-if="activeStep === 2 || isEdit"
          variant="primary"
          :disabled="initializing || submitting"
          @click="handleSubmit"
          class="!px-8"
        >
          <template #icon><div class="i-ep-check"></div></template>
          {{ submitting ? "提交中..." : isEdit ? "保存修改" : "立即创建" }}
        </ToolbarButton>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { ElMessage } from "element-plus";
import {
  createTaskWizardForm,
  type TaskWizardScriptSourceMode,
} from "../../composables/taskWizardHelpers";
import {
  useTaskWizardData,
  type TaskWizardInitialData,
} from "../../composables/useTaskWizardData";
import { useTaskWizardFileBrowser } from "../../composables/useTaskWizardFileBrowser";
import { useTaskWizardSubmit } from "../../composables/useTaskWizardSubmit";
import { useTaskWizardVariables } from "../../composables/useTaskWizardVariables";
import TaskWizardDetailsStep from "./TaskWizardDetailsStep.vue";
import TaskWizardEnvironmentStep from "./TaskWizardEnvironmentStep.vue";
import TaskWizardScriptStep from "./TaskWizardScriptStep.vue";
import ToolbarButton from "../common/ToolbarButton.vue";

type TaskWizardDetailsStepExpose = {
  validate: () => Promise<boolean>;
};

const props = defineProps<{
  initialData?: TaskWizardInitialData;
}>();

const emit = defineEmits<{
  (e: "success"): void;
  (e: "cancel"): void;
}>();

const stepItems = [
  { label: "脚本来源" },
  { label: "任务配置" },
  { label: "环境与变量" },
];
const initialData = computed(() => props.initialData);

// State

const activeStep = ref(props.initialData?.id ? 1 : 0);
const initializing = ref(true);

const cronDescription = ref("");
const cronValid = ref(true);

const scriptSourceMode = ref<TaskWizardScriptSourceMode>("upload");

const isEdit = computed(() => !!initialData.value?.id);

// Form Data

const form = reactive(createTaskWizardForm());

const detailsStepRef = ref<TaskWizardDetailsStepExpose | null>(null);

const {
  getSubmitVariables,
  setVariables,
  variableMode,
  variablesBulk,
  variablesList,
} = useTaskWizardVariables();

const { allTasks, loadWizardData, nodeVersions, pythonVersions } =
  useTaskWizardData({
    form,
    initialData,
    isEdit,
    scriptSourceMode,
    setVariables,
  });

const {
  applyUploadedFile,
  browserItems,
  browserLoading,
  currentPath,
  handleBrowserItemClick,
  handleScriptUpload,
  navigate,
  navigateUp,
  pathParts,
  searchQuery,
  uploadedFile,
} = useTaskWizardFileBrowser({
  form,
  isEdit,
  nodeVersions,
  pythonVersions,
});

const { submit, submitting } = useTaskWizardSubmit({
  form,
  getSubmitVariables,
  initialData,
  isEdit,
  onSuccess: () => emit("success"),
  scriptSourceMode,
  uploadedFile,
});

const init = async () => {
  await loadWizardData();

  const initialUploadedFile = initialData.value?.uploadedFile;
  if (initialUploadedFile && !isEdit.value) {
    scriptSourceMode.value = "upload";
    applyUploadedFile(initialUploadedFile);
  }

  if (scriptSourceMode.value === "file") {
    await navigate("");
  }
  initializing.value = false;
};

watch(scriptSourceMode, (newVal) => {
  if (newVal === "file" && browserItems.value.length === 0) {
    navigate("");
  }

  // Clear mutually exclusive fields when switching modes

  if (newVal === "file" || newVal === "upload") form.command = "";

  if (newVal === "command") form.path = "";
});

const validateScript = () => {
  if (scriptSourceMode.value === "command" && !form.command.trim()) {
    ElMessage.error("请输入执行命令");
    return false;
  }
  if (scriptSourceMode.value === "file" && !form.path) {
    ElMessage.error("请选择脚本文件");
    return false;
  }
  if (scriptSourceMode.value === "upload" && !uploadedFile.value) {
    ElMessage.error("请上传脚本文件");
    return false;
  }
  return true;
};

const validateDetails = async () => {
  if (!await detailsStepRef.value?.validate()) return false;
  if (!form.enableRandom && form.cron_schedule && !cronValid.value) {
    ElMessage.error("请修正定时表达式");
    return false;
  }
  if (form.enableRandom && (!form.random_config.start || !form.random_config.end || form.random_config.start >= form.random_config.end)) {
    ElMessage.error("随机执行的截止时间必须晚于起始时间");
    return false;
  }
  return true;
};

const handleNext = async () => {
  if (initializing.value || submitting.value) return;
  if (activeStep.value === 0 && validateScript()) activeStep.value = 1;
  else if (activeStep.value === 1 && await validateDetails()) activeStep.value = 2;
};

const handleSubmit = async () => {
  if (initializing.value || submitting.value) return;
  if (!validateScript()) { activeStep.value = 0; return; }
  if (!await validateDetails()) { activeStep.value = 1; return; }
  await submit();
};

onMounted(init);
</script>

<style scoped>
.task-wizard {
  height: min(700px, calc(var(--app-viewport-height) - 120px));
}
.task-wizard :deep(.toolbar-button) { min-height: 44px; }
.task-wizard-content :deep(.el-input__wrapper),
.task-wizard-content :deep(.el-select__wrapper) { min-height: 40px; }
.task-wizard-content :deep(.el-form-item__label) { font-weight: 600; }
@media (max-width: 768px) {
  .task-wizard { height: 100%; }
  .task-wizard-content :deep(.el-input__wrapper),
  .task-wizard-content :deep(.el-select__wrapper) { min-height: 44px; }
}
</style>
