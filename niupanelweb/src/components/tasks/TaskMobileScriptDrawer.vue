<template>
  <OverlayDrawer
    v-model:visible="visibleValue"
    :title="task?.name || '编辑脚本'"
    variant="workspace"
    content-preset="workspace"
    custom-class="editor-overlay"
    destroy-on-close
    append-to-body
    :lock-scroll="false"
    :close-on-header="false"
    @opened="emit('opened')"
    @close="emit('drawer-close')"
    @request-close="emit('request-close')"
  >
    <template #title>
      <div class="min-w-0">
        <div class="truncate text-[13px] font-bold leading-tight text-default">
          {{ task?.name || "编辑脚本" }}
        </div>
        <div class="mt-0.5 truncate font-mono text-[10px] leading-tight text-muted">
          {{ task?.path || task?.env_type || "shell" }}
        </div>
      </div>
    </template>

    <template #header-actions>
      <span class="rounded-md bg-soft px-2 py-1 font-mono text-[10px] font-bold text-secondary">
        {{ language }}
      </span>
      <el-button
        type="primary"
        :loading="loading"
        class="!min-h-11 !rounded-md !px-3 !text-xs font-bold"
        @click="emit('save')"
      >
        <span class="i-ep-check mr-1 text-xs"></span>
        保存
      </el-button>
    </template>

    <div class="flex flex-col h-full bg-[var(--editor-bg)] overflow-hidden">
      <div class="flex-1 overflow-hidden relative">
        <VueMonacoEditor
          v-if="ready && !loading"
          :key="'script-editor-' + (task?.id || 'new')"
          v-model:value="contentValue"
          theme="vs-dark"
          :language="language"
          :options="options"
          class="h-full w-full"
          @mount="emit('editor-mount', $event)"
        />
        <div v-else class="absolute inset-0 flex-center bg-[var(--editor-bg)]">
          <div class="i-ep-loading animate-spin text-primary text-2xl"></div>
        </div>
      </div>

      <div
        class="task-mobile-script-toolbar shrink-0 bg-[var(--editor-toolbar-bg)] border-t border-[var(--editor-border)] flex items-center justify-between px-3 pt-2"
      >
        <div class="flex items-center gap-1">
          <button
            type="button"
            class="h-11 w-11 cursor-pointer rounded-md text-[var(--editor-text)] opacity-50 flex-center transition-colors hover:bg-[var(--editor-border)] hover:opacity-90"
            title="撤销"
            @click="emit('editor-command', 'undo')"
          >
            <div class="i-ep-refresh-left text-sm"></div>
          </button>
          <button
            type="button"
            class="h-11 w-11 cursor-pointer rounded-md text-[var(--editor-text)] opacity-50 flex-center transition-colors hover:bg-[var(--editor-border)] hover:opacity-90"
            title="重做"
            @click="emit('editor-command', 'redo')"
          >
            <div class="i-ep-refresh-right text-sm"></div>
          </button>
          <div class="w-px h-4 bg-[var(--editor-border)] mx-1"></div>
          <button
            type="button"
            class="h-11 w-11 cursor-pointer rounded-md text-[var(--editor-text)] opacity-50 flex-center transition-colors hover:bg-[var(--editor-border)] hover:opacity-90"
            title="格式化"
            @click="emit('editor-command', 'format')"
          >
            <div class="i-ep-magic-stick text-sm"></div>
          </button>
          <button
            type="button"
            class="h-11 w-11 cursor-pointer rounded-md text-[var(--editor-text)] opacity-50 flex-center transition-colors hover:bg-[var(--editor-border)] hover:opacity-90"
            :class="{ '!text-primary': wordWrap }"
            title="换行"
            @click="emit('toggle-word-wrap')"
          >
            <div class="i-ep-notebook text-sm"></div>
          </button>
        </div>
        <div class="flex items-center gap-1">
          <span class="text-[10px] text-[var(--editor-text)] opacity-30 font-mono tabular-nums">
            {{ content.split("\n").length }} 行
          </span>
        </div>
      </div>
    </div>
  </OverlayDrawer>
</template>

<script setup lang="ts">
import { computed, defineAsyncComponent } from "vue";
import type { Task } from "@/types";
import OverlayDrawer from "../common/OverlayDrawer.vue";
import type {
  TaskEditorOptions,
  TaskMobileScriptEditorCommand,
  TaskScriptEditorRef,
} from "../../composables/taskPageTypes";

const VueMonacoEditor = defineAsyncComponent({
  loader: () =>
    import("@guolao/vue-monaco-editor").then((mod) => mod.VueMonacoEditor),
  loadingComponent: {
    template: '<div class="flex-center h-full"><div class="i-ep-loading animate-spin text-2xl"></div></div>',
  },
  delay: 200,
  timeout: 10000,
});

const props = defineProps<{
  content: string;
  language: string;
  loading: boolean;
  options: TaskEditorOptions;
  ready: boolean;
  task?: Task | null;
  visible: boolean;
  wordWrap: boolean;
}>();

const emit = defineEmits<{
  (event: "drawer-close"): void;
  (event: "editor-command", command: TaskMobileScriptEditorCommand): void;
  (event: "editor-mount", editor: TaskScriptEditorRef): void;
  (event: "opened"): void;
  (event: "request-close"): void;
  (event: "save"): void;
  (event: "toggle-word-wrap"): void;
  (event: "update:content", content: string): void;
  (event: "update:visible", visible: boolean): void;
}>();

const visibleValue = computed({
  get: () => props.visible,
  set: (value: boolean) => emit("update:visible", value),
});

const contentValue = computed({
  get: () => props.content,
  set: (value: string) => emit("update:content", value),
});
</script>

<style scoped>
.task-mobile-script-toolbar {
  padding-bottom: calc(8px + env(safe-area-inset-bottom, 0px));
}
</style>
