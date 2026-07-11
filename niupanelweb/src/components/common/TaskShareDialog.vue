<template>
  <ResponsiveDialog
    v-model:visible="dialogVisible"
    title="创建分享链接"
    width="800px"
    size="92%"
    :close-on-click-modal="false"
    @close="handleClose"
  >
    <div class="min-h-0 flex-1 overflow-y-auto p-4 sm:p-5">
      <el-steps
        :active="activeStep"
        finish-status="success"
        simple
        class="mb-5 rounded-lg border border-light !bg-subtle"
      >
        <el-step title="选择文件与关联" :icon="Select" />
        <el-step title="生成分享链接" :icon="Share" />
      </el-steps>

      <div class="step-content">
        <div v-show="activeStep === 0" class="step-container">
          <TaskShareFileSelector
            ref="fileSelectorRef"
            v-model:current-main-file="currentMainFile"
            :current-checked-files="currentCheckedFiles"
            :current-editing-task="currentEditingTask"
            :current-tree-data="currentTreeData"
            :is-mobile="isMobile"
            :loading-file-tree="loadingFileTree"
            :task-files="taskFiles"
            :tasks="props.tasks"
            @edit="startEdit"
            @select-all="selectAllFiles"
            @stop-edit="stopEdit"
            @tree-check="handleTreeCheck"
          />

          <TaskShareAdvancedOptions
            v-show="!currentEditingTask"
            v-model:burn-after-reading="burnAfterReading"
            v-model:expires-hours="expiresHours"
            v-model:include-envs="includeEnvs"
            v-model:max-uses="maxUses"
            v-model:share-note="shareNote"
            v-model:share-password="sharePassword"
            v-model:show-advanced="showAdvanced"
          />
        </div>

        <TaskShareResultStep
          v-show="activeStep === 1"
          :burn-after-reading="burnAfterReading"
          :share-link="shareLink"
          :share-password="sharePassword"
          @copy="copyLink"
        />
      </div>
    </div>

    <template #footer>
      <div class="dialog-footer" v-if="!currentEditingTask">
        <el-button @click="handleClose" class="h-10 !rounded-lg"
          >取消</el-button
        >
        <el-button
          v-if="activeStep > 0"
          @click="activeStep--"
          class="h-10 !rounded-lg"
          >上一步</el-button
        >
        <el-button
          type="primary"
          v-if="activeStep === 0"
          @click="handleGenerateToken"
          :loading="loadingGenerate"
          class="h-10 !rounded-lg"
        >
          生成并上传分享
        </el-button>
        <el-button
          type="primary"
          v-if="activeStep === 1"
          @click="handleClose"
          class="h-10 px-8 !rounded-lg"
          >完成</el-button
        >
      </div>
    </template>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { Select, Share } from "@element-plus/icons-vue";
import { useMobile } from "../../composables/useMobile";
import { useTaskShareDialog } from "../../composables/useTaskShareDialog";
import type { Task } from "@/types";
import TaskShareAdvancedOptions from "./TaskShareAdvancedOptions.vue";
import TaskShareFileSelector from "./TaskShareFileSelector.vue";
import TaskShareResultStep from "./TaskShareResultStep.vue";
import ResponsiveDialog from "./ResponsiveDialog.vue";

type TaskShareDialogProps = {
  visible?: boolean;
  tasks?: Task[];
};

const props = withDefaults(defineProps<TaskShareDialogProps>(), {
  visible: false,
  tasks: () => [],
});

const emit = defineEmits<{
  (event: "close"): void;
  (event: "shared"): void;
  (event: "update:visible", visible: boolean): void;
}>();

const { isMobile } = useMobile();

const dialogVisible = computed({
  get: () => props.visible,
  set: (val) => emit("update:visible", val),
});

const {
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
} = useTaskShareDialog({
  onShared: () => emit("shared"),
  tasks: () => props.tasks,
  visible: dialogVisible,
});

const handleClose = () => {
  dialogVisible.value = false;
  emit("close");
};
</script>
