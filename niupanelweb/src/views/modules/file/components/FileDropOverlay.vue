<template>
  <transition name="el-fade-in">
    <div
      v-if="visible"
      class="pointer-events-none absolute inset-0 z-50 flex flex-col items-center justify-center border-4 border-dashed border-primary/30 bg-card/95"
    >
      <div class="accent-subtle mb-6 h-20 w-20 rounded-md flex-center">
        <div class="i-ep-upload-filled text-5xl animate-bounce"></div>
      </div>
      <div class="text-sm font-bold text-primary">
        松开鼠标以上传文件
      </div>
    </div>
  </transition>
</template>

<script setup lang="ts">
defineProps<{
  visible: boolean;
}>();
</script>
