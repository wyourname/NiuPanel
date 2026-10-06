<template>
  <form class="dependency-install-form" aria-label="安装依赖" @submit.prevent="submit">
    <label class="sr-only" for="environment-package-list">安装依赖名称</label>
    <div class="dependency-install-controls">
      <el-input
        id="environment-package-list"
        ref="inputRef"
        :model-value="packageText"
        :disabled="busy"
        type="textarea"
        :rows="1"
        resize="none"
        :placeholder="placeholder"
        :title="`${help} ${scopeHint} Ctrl / ⌘ + Enter 安装。`"
        aria-describedby="environment-package-help environment-package-status"
        :aria-invalid="Boolean(validationError)"
        spellcheck="false"
        autocapitalize="off"
        autocorrect="off"
        class="dependency-install-input modern-input"
        @update:model-value="emit('update:packageText', String($event))"
        @keydown.ctrl.enter.prevent="submit"
        @keydown.meta.enter.prevent="submit"
      />
      <ToolbarButton type="submit" variant="primary" class="dependency-install-submit !min-h-11 gap-2" :disabled="!canSubmit">
        <template #icon>
          <span :class="installing ? 'i-ep-loading animate-spin motion-reduce:animate-none' : 'i-ep-plus'" aria-hidden="true"></span>
        </template>
        {{ installing ? '提交中…' : packageCount ? `安装 ${packageCount} 个依赖` : '安装依赖' }}
      </ToolbarButton>
    </div>
    <p id="environment-package-help" class="sr-only">{{ help }} {{ scopeHint }} Ctrl / ⌘ + Enter 安装。</p>
    <p
      id="environment-package-status"
      :role="validationError ? 'alert' : 'status'"
      :class="[
        validationError ? 'mb-0 mt-1 break-words text-xs leading-5' : 'sr-only',
        validationError ? 'text-rose-600 dark:text-rose-400' : 'text-secondary',
      ]"
    >
      {{ validationError || (busy ? '当前操作完成后可继续安装，列表将自动刷新。' : packageCount ? `已识别 ${packageCount} 个依赖` : '') }}
    </p>
  </form>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import type { Env } from "@/types";
import type { InputInstance } from "element-plus";
import ToolbarButton from "@/components/common/ToolbarButton.vue";

const props = defineProps<{
  env?: Env | null;
  installing: boolean;
  busy: boolean;
  packageText: string;
  packageCount: number;
  validationError: string;
}>();

const emit = defineEmits<{
  (event: "install"): void;
  (event: "update:packageText", value: string): void;
}>();

const inputGuide = computed(() => {
  switch (props.env?.env_type) {
    case "node":
      return {
        help: "空格或换行分隔，可用 @ 指定版本。",
        placeholder: "包名，支持多行",
        scopeHint: "同一 Node.js 版本的任务共用这些依赖。",
      };
    case "sh":
      return {
        help: "填写 Linux 软件包名，空格或换行分隔。",
        placeholder: "包名，支持多行",
        scopeHint: "软件包将安装到当前 Linux 容器。",
      };
    default:
      return {
        help: "每行一个包，支持版本约束和 extras。",
        placeholder: "包名，每行一个",
        scopeHint: "依赖仅安装到当前 Python 虚拟环境。",
      };
  }
});
const help = computed(() => inputGuide.value.help);
const placeholder = computed(() => inputGuide.value.placeholder);
const scopeHint = computed(() => inputGuide.value.scopeHint);
const canSubmit = computed(() => !props.busy && Boolean(props.env) && props.packageCount > 0 && !props.validationError);
const submit = () => {
  if (canSubmit.value) emit("install");
};
const inputRef = ref<InputInstance>();
defineExpose({
  focus: () => {
    inputRef.value?.textarea?.scrollIntoView({ block: "center" });
    inputRef.value?.focus();
  },
});
</script>

<style scoped>
.dependency-install-form {
  min-width: 0;
  flex: 1;
}

.dependency-install-controls {
  display: flex;
  align-items: flex-start;
  gap: 8px;
}

.dependency-install-input {
  min-width: 0;
  flex: 1;
}

.dependency-install-submit {
  flex-shrink: 0;
}

.dependency-install-input :deep(.el-textarea__inner) {
  height: 44px;
  min-height: 44px !important;
  max-height: 44px;
  overflow-y: auto;
  padding: 0 12px;
  font-family: var(--font-mono);
  font-size: 16px !important;
  line-height: 42px;
}

.dependency-install-input :deep(.el-textarea__inner:focus) {
  border-color: var(--el-color-primary);
  box-shadow: var(--focus-ring) !important;
}

.dependency-install-input :deep(.el-textarea__inner[aria-invalid="true"]) {
  box-shadow: 0 0 0 1px var(--el-color-danger) inset !important;
}

@media (min-width: 769px) {
  .dependency-install-input :deep(.el-textarea__inner) {
    font-size: 13px !important;
  }
}
</style>
