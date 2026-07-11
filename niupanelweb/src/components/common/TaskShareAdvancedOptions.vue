<template>
  <div class="mt-4 pt-4 border-t border-base">
    <div class="flex flex-wrap gap-4 items-center">
      <el-checkbox
        :model-value="includeEnvs"
        @update:model-value="updateBoolean('includeEnvs', $event)"
      >
        包含环境变量
      </el-checkbox>
      <el-button link type="primary" @click="emit('update:showAdvanced', !showAdvanced)">
        {{ showAdvanced ? "隐藏高级选项" : "显示高级选项" }}
      </el-button>
    </div>

    <div
      v-if="showAdvanced"
      class="mt-3 rounded-lg border border-light bg-subtle p-4"
    >
      <el-form label-position="top" size="default" class="advanced-form">
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
          <el-form-item label="备注信息" class="!mb-0 md:col-span-2">
            <el-input
              :model-value="shareNote"
              placeholder="输入分享备注..."
              @update:model-value="emit('update:shareNote', String($event))"
            />
          </el-form-item>
          <el-form-item label="访问密码" class="!mb-0">
            <el-input
              :model-value="sharePassword"
              type="password"
              placeholder="可选 (默认无)"
              show-password
              @update:model-value="emit('update:sharePassword', String($event))"
            />
          </el-form-item>
          <el-form-item label="有效期 (小时)" class="!mb-0">
            <el-input-number
              :model-value="expiresHours"
              :min="-1"
              class="!w-full"
              controls-position="right"
              @update:model-value="updateNumber('expiresHours', $event, -1)"
            />
          </el-form-item>
          <el-form-item label="最大下载次数" class="!mb-0">
            <el-input-number
              :model-value="maxUses"
              :min="1"
              class="!w-full"
              controls-position="right"
              @update:model-value="updateNumber('maxUses', $event, 1)"
            />
          </el-form-item>
          <div class="flex items-end pb-2">
            <el-checkbox
              :model-value="burnAfterReading"
              @update:model-value="updateBoolean('burnAfterReading', $event)"
            >
              <div class="flex items-center gap-1 text-sm">
                <el-icon class="text-rose-500"><Warning /></el-icon>
                <span>阅后即焚 (下载后失效)</span>
              </div>
            </el-checkbox>
          </div>
        </div>
      </el-form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { Warning } from "@element-plus/icons-vue";

defineProps<{
  burnAfterReading: boolean;
  expiresHours: number;
  includeEnvs: boolean;
  maxUses: number;
  shareNote: string;
  sharePassword: string;
  showAdvanced: boolean;
}>();

const emit = defineEmits<{
  (event: "update:burnAfterReading", value: boolean): void;
  (event: "update:expiresHours", value: number): void;
  (event: "update:includeEnvs", value: boolean): void;
  (event: "update:maxUses", value: number): void;
  (event: "update:shareNote", value: string): void;
  (event: "update:sharePassword", value: string): void;
  (event: "update:showAdvanced", value: boolean): void;
}>();

type BooleanField = "burnAfterReading" | "includeEnvs";
type NumberField = "expiresHours" | "maxUses";

const updateBoolean = (field: BooleanField, value: string | number | boolean) => {
  if (field === "burnAfterReading") {
    emit("update:burnAfterReading", Boolean(value));
    return;
  }
  emit("update:includeEnvs", Boolean(value));
};

const updateNumber = (
  field: NumberField,
  value: number | undefined,
  fallback: number,
) => {
  if (field === "expiresHours") {
    emit("update:expiresHours", value ?? fallback);
    return;
  }
  emit("update:maxUses", value ?? fallback);
};
</script>
