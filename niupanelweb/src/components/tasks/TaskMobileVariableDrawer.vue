<template>
  <OverlayDrawer
    v-model:visible="visibleValue"
    :title="`环境变量 #${taskId ?? ''}`"
    variant="workspace"
    content-preset="workspace"
    destroy-on-close
    append-to-body
  >
    <TaskVariableEditor
      v-if="visible && taskId"
      :task-id="taskId"
      @success="emit('success')"
      @cancel="visibleValue = false"
    />
  </OverlayDrawer>
</template>

<script setup lang="ts">
import { computed } from "vue";
import OverlayDrawer from "../common/OverlayDrawer.vue";
import TaskVariableEditor from "./TaskVariableEditor.vue";

const props = defineProps<{
  taskId: number | null;
  visible: boolean;
}>();

const emit = defineEmits<{
  (event: "success"): void;
  (event: "update:visible", visible: boolean): void;
}>();

const visibleValue = computed({
  get: () => props.visible,
  set: (value: boolean) => emit("update:visible", value),
});
</script>
