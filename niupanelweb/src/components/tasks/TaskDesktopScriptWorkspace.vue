<template>
  <div
    class="full flex min-h-0 flex-col overflow-hidden bg-[#0b111c]"
    v-loading="loading"
  >
    <div
      class="flex h-10 shrink-0 items-center justify-between gap-3 border-b border-white/8 bg-[#0f1724] px-3"
    >
      <div class="flex min-w-0 items-center gap-2">
        <div
          class="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg border border-sky-400/20 bg-sky-400/10 text-sky-300"
        >
          <div class="i-ep-document text-sm"></div>
        </div>
        <div class="min-w-0">
          <div class="truncate text-[13px] font-bold text-slate-100">
            {{ editorTitle }}
          </div>
          <div
            v-if="editorMeta"
            class="truncate font-mono text-[10px] font-semibold text-slate-400"
          >
            {{ editorMeta }}
          </div>
        </div>
      </div>

      <div class="flex shrink-0 items-center gap-2">
        <span
          class="rounded-md border border-white/8 bg-white/6 px-1.5 py-0.5 font-mono text-[10px] font-bold text-slate-300"
        >
          {{ language }}
        </span>
        <el-button
          type="primary"
          size="small"
          class="!h-7 !rounded-lg !px-3 !text-[12px] font-bold"
          :disabled="loading"
          :loading="saving"
          @click="emit('save')"
        >
          <div class="i-ep-check mr-1 text-xs"></div>
          保存
        </el-button>
      </div>
    </div>

    <div class="min-h-0 flex-1 overflow-hidden bg-[#0b111c]">
      <VueMonacoEditor
        :key="'editor-' + language"
        v-model:value="contentValue"
        theme="vs-dark"
        :language="language"
        :options="options"
        class="h-full w-full"
        @mount="emit('editor-mount', $event)"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { Task } from "@/types";
import { createAsyncMonacoEditor } from "@/utils/monaco";
import type {
  TaskEditorOptions,
  TaskScriptEditorRef,
} from "../../composables/taskPageTypes";

const VueMonacoEditor = createAsyncMonacoEditor();

const props = defineProps<{
  content: string;
  language: string;
  loading: boolean;
  options: TaskEditorOptions;
  saving: boolean;
  task?: Task | null;
}>();

const emit = defineEmits<{
  (event: "editor-mount", editor: TaskScriptEditorRef): void;
  (event: "save"): void;
  (event: "update:content", content: string): void;
}>();

const contentValue = computed({
  get: () => props.content,
  set: (value: string) => emit("update:content", value),
});

const editorTitle = computed(() => props.task?.name || "脚本");

const editorMeta = computed(() => {
  const path = props.task?.path?.trim();
  if (path) return path;
  return props.task?.env_type || "";
});
</script>
