<template>
  <ResponsiveDialog
    v-model:visible="visibleValue"
    :title="task?.id ? '编辑任务' : '创建任务'"
    desktop-size="xl"
    content-preset="workspace"
    mobile-mode="fullscreen"
    append-to-body
    destroy-on-close
  >
    <TaskWizard
      v-if="visible"
      :initial-data="task || {}"
      @success="emit('success')"
      @cancel="visibleValue = false"
    />
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { TaskWizardInitialData } from "../../composables/useTaskWizardData";
import TaskWizard from "./TaskWizard.vue";
import ResponsiveDialog from "../common/ResponsiveDialog.vue";

const props = defineProps<{
  task: TaskWizardInitialData | null;
  visible: boolean;
}>();

const emit = defineEmits<{
  (event: "success"): void;
  (event: "update:visible", visible: boolean): void;
  (event: "cancel"): void;
}>();

const visibleValue = computed({
  get: () => props.visible,
  set: (value: boolean) => emit("update:visible", value),
});
</script>
