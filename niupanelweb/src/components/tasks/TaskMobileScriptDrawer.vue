<template>
  <el-drawer
    v-model="visibleValue"
    size="100%"
    :with-header="false"
    direction="btt"
    destroy-on-close
    append-to-body
    class="log-modal"
    :lock-scroll="false"
    @opened="emit('opened')"
    @close="emit('drawer-close')"
  >
    <div class="flex flex-col h-full bg-[var(--editor-bg)] overflow-hidden">
      <div
        class="shrink-0 px-3 h-12 flex items-center justify-between border-b border-[var(--editor-border)] z-10"
      >
        <div class="flex items-center gap-2 min-w-0 flex-1">
          <button
            type="button"
            class="h-9 w-9 shrink-0 rounded-md border-none bg-transparent text-[var(--editor-text)] opacity-60 flex-center transition-colors hover:bg-[var(--editor-border)] touch-manipulation"
            aria-label="关闭脚本编辑器"
            @click="emit('request-close')"
          >
            <div class="i-ep-back text-[22px]"></div>
          </button>
          <div class="flex flex-col min-w-0 justify-center">
            <span class="text-[15px] font-bold text-[var(--editor-text)] truncate leading-tight">
              {{ task?.name || "编辑脚本" }}
            </span>
            <span
              class="text-[11px] text-[var(--editor-text)] opacity-40 truncate leading-tight mt-0.5 font-mono"
            >
              {{ task?.path || task?.env_type || "shell" }}
            </span>
          </div>
        </div>
        <div class="flex items-center gap-1.5 pr-1 shrink-0">
          <span
            class="rounded-md bg-[var(--editor-border)] px-2 py-0.5 font-mono text-[10px] font-bold text-purple-300"
          >
            {{ language }}
          </span>
          <el-button
            type="primary"
            size="small"
            :loading="loading"
            class="!h-7 !rounded-md !px-4 text-xs font-bold"
            @click="emit('save')"
          >
            <div class="i-ep-check mr-1 text-xs"></div>
            保存
          </el-button>
        </div>
      </div>

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
        class="shrink-0 bg-[var(--editor-toolbar-bg)] border-t border-[var(--editor-border)] flex items-center justify-between px-3 py-2"
      >
        <div class="flex items-center gap-1">
          <button
            type="button"
            class="h-8 w-8 rounded-md text-[var(--editor-text)] opacity-50 flex-center transition-colors hover:bg-[var(--editor-border)] hover:opacity-90"
            title="撤销"
            @click="emit('editor-command', 'undo')"
          >
            <div class="i-ep-refresh-left text-sm"></div>
          </button>
          <button
            type="button"
            class="h-8 w-8 rounded-md text-[var(--editor-text)] opacity-50 flex-center transition-colors hover:bg-[var(--editor-border)] hover:opacity-90"
            title="重做"
            @click="emit('editor-command', 'redo')"
          >
            <div class="i-ep-refresh-right text-sm"></div>
          </button>
          <div class="w-px h-4 bg-[var(--editor-border)] mx-1"></div>
          <button
            type="button"
            class="h-8 w-8 rounded-md text-[var(--editor-text)] opacity-50 flex-center transition-colors hover:bg-[var(--editor-border)] hover:opacity-90"
            title="格式化"
            @click="emit('editor-command', 'format')"
          >
            <div class="i-ep-magic-stick text-sm"></div>
          </button>
          <button
            type="button"
            class="h-8 w-8 rounded-md text-[var(--editor-text)] opacity-50 flex-center transition-colors hover:bg-[var(--editor-border)] hover:opacity-90"
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
  </el-drawer>
</template>

<script setup lang="ts">
import { computed, defineAsyncComponent } from "vue";
import type { Task } from "@/types";
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
