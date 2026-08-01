<template>
  <ResponsiveDialog
    :visible="visible"
    title="手动卸载依赖"
    width="420px"
    @update:visible="emit('update:visible', $event)"
  >
    <div class="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto p-5">
      <div class="flex items-start gap-3 rounded-lg border border-amber-500/20 bg-amber-500/5 p-3.5">
        <span class="h-8 w-8 shrink-0 rounded-md bg-amber-500/10 text-amber-600 dark:text-amber-300 flex-center">
          <span class="i-ep-warning-filled text-[15px]"></span>
        </span>
        <p class="text-[10px] leading-4 text-secondary">
          请输入要卸载的包名。系统将尝试清理目标包及其相关依赖，此操作会提交为后台任务。
        </p>
      </div>
      <label class="text-[11px] font-semibold text-secondary" for="environment-package-name">包名</label>
      <el-input
        id="environment-package-name"
        :model-value="packageName"
        placeholder="请输入包名"
        clearable
        autocomplete="off"
        class="modern-input"
        @keyup.enter="emit('uninstall')"
        @update:model-value="emit('update:packageName', String($event))"
      />
    </div>
    <template #footer>
      <div class="flex w-full gap-3">
        <ToolbarButton block @click="emit('update:visible', false)">取消</ToolbarButton>
        <ToolbarButton block variant="danger" :disabled="!hasPackageName" @click="emit('uninstall')">
          立即卸载
        </ToolbarButton>
      </div>
    </template>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { computed } from "vue";
import ResponsiveDialog from "../../../../components/common/ResponsiveDialog.vue";
import ToolbarButton from "../../../../components/common/ToolbarButton.vue";

const props = defineProps<{
  packageName: string;
  visible: boolean;
}>();

const hasPackageName = computed(() => props.packageName.trim().length > 0);

const emit = defineEmits<{
  (event: "uninstall"): void;
  (event: "update:packageName", value: string): void;
  (event: "update:visible", value: boolean): void;
}>();
</script>
