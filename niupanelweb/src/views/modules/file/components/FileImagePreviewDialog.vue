<template>
  <el-dialog
    v-model="visible"
    width="80%"
    align-center
    append-to-body
    class="bg-transparent"
    :show-close="false"
  >
    <div
      class="relative flex items-center justify-center p-4 animate-fade-in"
      @click="visible = false"
    >
      <img
        :src="src"
        class="max-h-[85vh] max-w-full rounded-md border border-white/10 bg-checkboard object-contain"
      />
      <div
        class="absolute right-4 top-4 h-9 w-9 cursor-pointer rounded-md bg-black/55 text-white flex-center transition-colors hover:bg-black/70"
        title="关闭预览"
        aria-label="关闭图片预览"
        @click.stop="visible = false"
      >
        <div class="i-ep-close text-2xl"></div>
      </div>
    </div>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed } from "vue";

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
