<template>
  <el-drawer
    v-model="visibleValue"
    :title="`Variables #${taskId ?? ''}`"
    size="100%"
    direction="btt"
    destroy-on-close
    append-to-body
    class="modern-dialog"
  >
    <TaskVariableEditor
      v-if="visible && taskId"
      :task-id="taskId"
      @success="emit('success')"
      @cancel="visibleValue = false"
    />
  </el-drawer>
</template>

<script setup lang="ts">
import { computed } from "vue";
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
