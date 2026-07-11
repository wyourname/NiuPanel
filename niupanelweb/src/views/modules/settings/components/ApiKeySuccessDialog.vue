<template>
  <ResponsiveDialog
    v-model:visible="visibleValue"
    title="密钥已生成"
    width="500px"
    append-to-body
  >
    <div class="flex flex-col gap-4 p-4">
      <div class="flex items-center gap-3">
        <div
          class="flex h-10 w-10 shrink-0 items-center justify-center rounded-md border border-emerald-300/50 bg-emerald-50 text-emerald-600 dark:border-emerald-400/20 dark:bg-emerald-500/10 dark:text-emerald-300"
        >
          <div class="i-ep-circle-check text-lg"></div>
        </div>
        <div class="min-w-0">
          <h3 class="text-sm font-bold text-default">只显示一次</h3>
          <p class="mt-0.5 text-[12px] text-secondary">
            关闭后无法再次查看完整密钥。
          </p>
        </div>
      </div>

      <div
        class="group relative w-full cursor-pointer break-all rounded-md border border-dashed border-emerald-400/45 bg-slate-950/[0.025] p-3 pr-11 font-mono text-[12px] leading-relaxed text-emerald-700 transition-colors hover:bg-white/80 dark:bg-white/[0.035] dark:text-emerald-300 dark:hover:bg-white/8"
        @click="emit('copy-token')"
      >
        <div class="font-bold">
          {{ token }}
        </div>
        <el-button
          link
          type="primary"
          class="absolute right-2 top-2"
          @click.stop="emit('copy-token')"
        >
          <div class="i-ep-copy-document text-lg"></div>
        </el-button>
      </div>

      <el-button
        type="primary"
        size="large"
        class="w-full !h-9 !rounded-md !text-sm font-bold"
        @click="visibleValue = false"
      >
        我已妥善保存
      </el-button>
    </div>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { computed } from "vue";
import ResponsiveDialog from "@/components/common/ResponsiveDialog.vue";

const props = defineProps<{
  token: string;
  visible: boolean;
}>();

const emit = defineEmits<{
  (event: "copy-token"): void;
  (event: "update:visible", visible: boolean): void;
}>();

const visibleValue = computed({
  get: () => props.visible,
  set: (visible: boolean) => emit("update:visible", visible),
});
</script>
