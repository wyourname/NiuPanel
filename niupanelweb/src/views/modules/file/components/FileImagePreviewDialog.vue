<template>
  <ResponsiveDialog
    v-model:visible="visible"
    title="图片预览"
    desktop-size="fluid"
    content-preset="workspace"
    mobile-mode="fullscreen"
    append-to-body
  >
    <div
      class="relative flex min-h-0 flex-1 items-center justify-center overflow-auto p-3 animate-fade-in md:p-4"
      @click="visible = false"
    >
      <img
        :src="src"
        alt="图片预览"
        class="max-w-full rounded-md border border-white/10 bg-checkboard object-contain"
        style="max-height: calc(var(--app-viewport-height) - 120px)"
      />
    </div>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { computed } from "vue";
import ResponsiveDialog from "@/components/common/ResponsiveDialog.vue";

const props = defineProps<{
  modelValue: boolean;
  src: string;
}>();

const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
}>();

const visible = computed({
  get: () => props.modelValue,
  set: (value) => emit("update:modelValue", value),
});
</script>

<style scoped>
.bg-checkboard {
  background-image:
    linear-gradient(45deg, #ccc 25%, transparent 25%),
    linear-gradient(-45deg, #ccc 25%, transparent 25%),
    linear-gradient(45deg, transparent 75%, #ccc 75%),
    linear-gradient(-45deg, transparent 75%, #ccc 75%);
  background-size: 20px 20px;
  background-position:
    0 0,
    0 10px,
    10px -10px,
    -10px 0px;
}
</style>
