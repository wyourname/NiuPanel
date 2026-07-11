<template>
  <div class="task-wizard flex h-full min-h-[520px] max-h-[82vh] flex-col">
    <div class="shrink-0 border-b border-light px-4 py-3 md:px-6">
      <div class="grid grid-cols-3 overflow-hidden rounded-md border border-light bg-base/60">
        <div
          v-for="(step, index) in stepItems"
          :key="step.label"
          class="flex min-w-0 items-center gap-2 border-r border-light px-3 py-2.5 last:border-r-0"
          :class="index === activeStep ? 'bg-card text-primary' : index < activeStep ? 'text-default' : 'text-muted'"
        >
          <span
            class="h-6 w-6 shrink-0 rounded text-[11px] font-bold flex-center"
            :class="index <= activeStep ? 'bg-primary text-white' : 'bg-soft text-muted'"
          >
            <span v-if="index < activeStep" class="i-ep-check"></span>
            <span v-else>{{ index + 1 }}</span>
          </span>
          <span class="min-w-0 truncate text-[11px] font-bold md:text-[12px]">{{ step.label }}</span>
        </div>
      </div>
    </div>

    <!-- Content Area (Scrollable) -->
    <div class="flex-1 overflow-x-hidden overflow-y-auto px-4 py-4 custom-scrollbar md:px-6">
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
        <ToolbarButton v-if="activeStep > 0" @click="activeStep--">
          <template #icon><div class="i-ep-arrow-left"></div></template>
          上一步
        </ToolbarButton>
        <ToolbarButton v-else variant="soft" @click="emit('cancel')">取消</ToolbarButton>
      </div>

      <div class="flex gap-3">
        <ToolbarButton v-if="activeStep < 2" variant="primary" @click="handleNext">
          下一步
          <template #icon><div class="i-ep-arrow-right"></div></template>
        </ToolbarButton>
        <ToolbarButton
          v-if="activeStep === 2"
          variant="primary"
          :disabled="submitting"
          @click="submit"
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

const activeStep = ref(0);

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
};

watch(scriptSourceMode, (newVal) => {
  if (newVal === "file" && browserItems.value.length === 0) {
    navigate("");
  }

  // Clear mutually exclusive fields when switching modes

  if (newVal === "file" || newVal === "upload") form.command = "";

  if (newVal === "command") form.path = "";
});

const handleNext = async () => {
  if (activeStep.value === 0) {
    if (scriptSourceMode.value === "command" && !form.command)
      return ElMessage.error("请输入命令");

    if (scriptSourceMode.value === "file" && !form.path)
      return ElMessage.error("请选择文件");

    if (
      scriptSourceMode.value === "upload" &&
      !uploadedFile.value &&
      !isEdit.value
    )
      return ElMessage.error("请上传文件");

    activeStep.value++;
  } else if (activeStep.value === 1) {
    if (await detailsStepRef.value?.validate()) {
      activeStep.value++;
    }
  }
};

onMounted(init);
</script>

<style scoped>
/* Scoped styles removed in favor of UnoCSS utility classes */
</style>
