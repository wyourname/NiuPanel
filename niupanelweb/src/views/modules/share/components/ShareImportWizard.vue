<template>
  <div class="flex flex-col gap-4">
    <!-- Compact Stepper -->
    <el-steps
      :active="downloadStep"
      finish-status="success"
      simple
      class="!bg-transparent !p-0 mb-2"
    >
      <el-step title="输入链接" />
      <el-step title="解析中" />
      <el-step title="确认内容" />
    </el-steps>

    <ShareImportInputStep
      v-if="downloadStep === 0"
      v-model:password="importForm.password"
      v-model:url="importForm.url"
      :downloading="downloading"
      @submit="handleSubmitImport"
    />

    <ShareImportParsingStep
      v-if="downloadStep === 1"
      :status="downloadStatus"
      @reset="resetImport"
      @retry="handleRetry"
    />

    <ShareImportPreviewStep
      v-if="previewPackage"
      :importing="importing"
      :package-info="previewPackage"
      :selected-tasks="selectedTasks"
      @confirm="confirmImport"
      @reset="resetImport"
      @toggle-all="toggleSelectAll"
      @toggle-task="toggleTaskSelection"
    />
  </div>
</template>

<script setup lang="ts">
import { useShareImportFlow } from "../composables/useShareImportFlow";
import ShareImportInputStep from "./ShareImportInputStep.vue";
import ShareImportParsingStep from "./ShareImportParsingStep.vue";
import ShareImportPreviewStep from "./ShareImportPreviewStep.vue";

const emit = defineEmits<{
  (event: "success"): void;
}>();

const {
  downloadStep,
  downloadStatus,
  downloading,
  handleConfirmImport,
  handleRetry,
  handleSubmitImport,
  importForm,
  importing,
  previewPackage,
  resetImport,
  selectedTasks,
  setImportUrl,
  toggleSelectAll,
  toggleTaskSelection,
} = useShareImportFlow();

const confirmImport = async () => {
  const success = await handleConfirmImport();
  if (success) emit("success");
};

defineExpose({ setImportUrl });
</script>
