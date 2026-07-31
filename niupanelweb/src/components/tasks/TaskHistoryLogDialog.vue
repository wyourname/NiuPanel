<template>
  <ResponsiveDialog
    v-model:visible="visibleValue"
    :title="`历史日志 - 运行 #${runId}`"
    width="860px"
    append-to-body
    destroy-on-close
  >
    <div
      class="h-[60vh] overflow-y-auto bg-[#1e1e1e] p-4 font-mono text-xs leading-relaxed"
      v-loading="loading"
    >
      <pre
        v-if="content"
        class="text-gray-200 whitespace-pre-wrap break-all m-0"
      >{{ content }}</pre>
      <div v-else-if="!loading" class="flex-center h-full text-gray-500">
        （日志为空）
      </div>
    </div>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { computed } from "vue";
import ResponsiveDialog from "../common/ResponsiveDialog.vue";

const props = defineProps<{
  content: string;
  loading: boolean;
  runId: number | null;
  visible: boolean;
}>();

const emit = defineEmits<{
  (event: "update:visible", visible: boolean): void;
}>();

const visibleValue = computed({
  get: () => props.visible,
  set: (value: boolean) => emit("update:visible", value),
});
</script>
