<template>
  <el-select
    :model-value="modelValue"
    :placeholder="placeholder"
    class="w-full mb-2"
    :teleported="true"
    popper-class="mirror-select-popper"
    @update:model-value="emit('update:modelValue', String($event))"
  >
    <el-option
      v-for="item in presets"
      :key="item.url"
      :label="item.name"
      :value="item.url"
    >
      <div class="flex items-center justify-between w-full">
        <span class="text-xs">{{ item.name }}</span>
        <span
          v-if="!isMobile"
          class="text-gray-400 text-[10px] ml-4 font-mono"
        >
          {{ item.url }}
        </span>
      </div>
    </el-option>
  </el-select>
</template>

<script setup lang="ts">
import type { MirrorPreset } from "./envMirrorTypes";

withDefaults(
  defineProps<{
    isMobile: boolean;
    modelValue: string;
    presets: readonly MirrorPreset[];
    placeholder?: string;
  }>(),
  {
    placeholder: "选择预设镜像",
  },
);

const emit = defineEmits<{
  (event: "update:modelValue", value: string): void;
}>();
</script>
