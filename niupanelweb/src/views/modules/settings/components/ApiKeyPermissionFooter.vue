<template>
  <div
    class="flex shrink-0 items-center justify-between gap-3 border-t border-slate-900/8 bg-white/76 px-3 py-2.5 dark:border-white/8 dark:bg-white/[0.03]"
  >
    <div v-if="!isMobile" class="text-[12px] font-bold text-secondary">
      已选 {{ selectedCount }} 项
    </div>

    <div class="flex w-full gap-2 md:w-auto">
      <el-button
        v-if="!isMobile"
        class="!h-8 !rounded-lg !px-4 !text-[12px] font-bold"
        @click="emit('cancel')"
      >
        取消
      </el-button>
      <el-button
        type="primary"
        :loading="submitting"
        class="!h-10 flex-1 !rounded-lg !px-4 text-[13px] font-bold md:!h-8 md:flex-none md:!text-[12px]"
        @click="emit('submit')"
      >
        {{ isEdit ? "保存修改" : "确认生成密钥" }}
      </el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  isEdit: boolean;
  isMobile: boolean;
  selectedCount: number;
  submitting: boolean;
}>();

const emit = defineEmits<{
  (event: "cancel"): void;
  (event: "submit"): void;
}>();
</script>
