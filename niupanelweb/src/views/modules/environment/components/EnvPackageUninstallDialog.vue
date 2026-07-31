<template>
  <ResponsiveDialog
    :visible="visible"
    title="手动卸载依赖"
    width="420px"
    @update:visible="emit('update:visible', $event)"
  >
    <div class="p-5 flex-1 overflow-y-auto space-y-4">
      <div class="text-xs text-muted mb-2">
        请输入要卸载的包名，系统将尝试清理其相关依赖。
      </div>
      <el-input
        :model-value="packageName"
        placeholder="请输入包名"
        class="modern-input"
        @keyup.enter="emit('uninstall')"
        @update:model-value="emit('update:packageName', String($event))"
      />
    </div>
    <template #footer>
      <div
        class="p-4 border-t border-light flex justify-end gap-3 shrink-0 bg-card"
      >
        <ToolbarButton @click="emit('update:visible', false)">取消</ToolbarButton>
        <ToolbarButton variant="primary" @click="emit('uninstall')">
          立即卸载
        </ToolbarButton>
      </div>
    </template>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import ResponsiveDialog from "../../../../components/common/ResponsiveDialog.vue";
import ToolbarButton from "../../../../components/common/ToolbarButton.vue";

defineProps<{
  packageName: string;
  visible: boolean;
}>();

const emit = defineEmits<{
  (event: "uninstall"): void;
  (event: "update:packageName", value: string): void;
  (event: "update:visible", value: boolean): void;
}>();
</script>
