<template>
  <ResponsiveDialog
    :visible="visible"
    title="安装新依赖"
    width="560px"
    @update:visible="emit('update:visible', $event)"
  >
    <div class="p-5 flex-1 overflow-y-auto space-y-4">
      <div
        class="rounded-md border border-light bg-base/60 px-4 py-3 text-xs leading-6 text-secondary"
      >
        每行输入一个包名，支持版本号，例如 `requests==2.32.3`、`pnpm`。
      </div>
      <el-input
        :model-value="packageText"
        type="textarea"
        :rows="7"
        placeholder="requests
pandas==2.2.3
pnpm"
        class="modern-input font-mono"
        @update:model-value="emit('update:packageText', String($event))"
      />
    </div>
    <template #footer>
      <div
        class="p-4 border-t border-light flex justify-end gap-3 shrink-0 bg-card"
      >
        <ToolbarButton @click="emit('update:visible', false)">取消</ToolbarButton>
        <ToolbarButton
          variant="primary"
          :disabled="installing"
          @click="emit('install')"
        >
          {{ installing ? "提交中..." : "开始安装" }}
        </ToolbarButton>
      </div>
    </template>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import ResponsiveDialog from "../../../../components/common/ResponsiveDialog.vue";
import ToolbarButton from "../../../../components/common/ToolbarButton.vue";

defineProps<{
  installing: boolean;
  packageText: string;
  visible: boolean;
}>();

const emit = defineEmits<{
  (event: "install"): void;
  (event: "update:packageText", value: string): void;
  (event: "update:visible", value: boolean): void;
}>();
</script>
