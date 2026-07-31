<template>
  <div
    class="shrink-0 border-b border-slate-900/8 bg-white/76 px-3 py-2.5 dark:border-white/8 dark:bg-white/[0.03]"
  >
    <div class="grid grid-cols-1 gap-2.5 md:grid-cols-[minmax(0,1fr)_190px]">
      <div class="flex min-w-0 items-center gap-2">
        <label class="w-16 shrink-0 text-[11px] font-bold text-secondary">
          密钥名称
        </label>
        <el-input
          :model-value="form.name"
          placeholder="用途标识"
          size="small"
          class="min-w-0"
          @update:model-value="emit('update:name', String($event))"
        />
      </div>
      <div class="flex min-w-0 items-center gap-2">
        <label class="w-16 shrink-0 text-[11px] font-bold text-secondary md:w-auto">
          {{ isEdit ? "过期日期" : "有效期" }}
        </label>
        <el-date-picker
          v-if="isEdit"
          :model-value="form.expires_at"
          type="datetime"
          placeholder="选择过期时间"
          size="small"
          class="!w-full"
          value-format="X"
          @update:model-value="emit('update:expiresAt', String($event ?? ''))"
        />
        <el-input-number
          v-else
          :model-value="form.expires_in_days"
          :min="1"
          :max="3650"
          size="small"
          class="!w-full"
          controls-position="right"
          @update:model-value="emit('update:expiresInDays', $event ?? 1)"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { ApiKeyFormState } from "../utils/apiKeyPermissions";

defineProps<{
  form: ApiKeyFormState;
  isEdit: boolean;
}>();

const emit = defineEmits<{
  (event: "update:expiresAt", value: string): void;
  (event: "update:expiresInDays", value: number): void;
  (event: "update:name", value: string): void;
}>();
</script>
