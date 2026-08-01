<template>
  <ResponsiveDialog
    :visible="visible"
    title="安装新依赖"
    width="560px"
    @update:visible="emit('update:visible', $event)"
  >
    <div class="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto p-5">
      <div class="flex items-start gap-3 rounded-lg border border-light bg-soft/50 p-3.5">
        <span class="h-8 w-8 shrink-0 rounded-md accent-subtle flex-center">
          <span class="i-ep-download text-[15px]"></span>
        </span>
        <p class="text-[10px] leading-4 text-secondary">
          每行填写一个包名，可附带版本号，例如 <code class="font-mono text-default">requests==2.32.3</code>、<code class="font-mono text-default">pnpm</code>。
        </p>
      </div>
      <label class="text-[11px] font-semibold text-secondary" for="environment-package-list">包列表</label>
      <el-input
        id="environment-package-list"
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
      <div class="flex w-full gap-3">
        <ToolbarButton block :disabled="installing" @click="emit('update:visible', false)">取消</ToolbarButton>
        <ToolbarButton
          block
          variant="primary"
          :disabled="installing || !hasPackages"
          @click="emit('install')"
        >
          {{ installing ? "提交中..." : "开始安装" }}
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
  installing: boolean;
  packageText: string;
  visible: boolean;
}>();

const hasPackages = computed(() => props.packageText.trim().length > 0);

const emit = defineEmits<{
  (event: "install"): void;
  (event: "update:packageText", value: string): void;
  (event: "update:visible", value: boolean): void;
}>();
</script>
