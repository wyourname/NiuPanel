<template>
  <ResponsiveDialog
    v-model:visible="visibleValue"
    :title="currentFile?.name"
    width="70%"
    size="100%"
    destroy-on-close
    append-to-body
    custom-class="log-modal"
  >
    <div class="flex-1 flex flex-col bg-[var(--editor-bg)] overflow-hidden">
      <div
        class="flex items-center justify-between px-4 shrink-0 border-b border-[var(--editor-border)] text-[var(--editor-text)] h-12"
      >
        <div class="flex items-center gap-4 overflow-hidden">
          <button
            v-if="!isMobile"
            type="button"
            class="h-8 w-8 rounded-md text-muted flex-center transition-colors hover:bg-black/5 hover:text-default dark:hover:bg-white/5"
            title="关闭编辑器"
            aria-label="关闭文件编辑器"
            @click="visibleValue = false"
          >
            <div class="i-ep-close text-lg"></div>
          </button>
          <span class="truncate text-[10px] font-bold opacity-70">
            {{ currentFile?.path }}
          </span>
        </div>
        <div class="flex items-center gap-2">
          <div class="hidden md:flex items-center gap-2 mr-4 text-[10px] font-bold">
            <span class="opacity-50">语言:</span>
            <span class="text-primary">{{ editorLanguage }}</span>
          </div>
          <el-button
            type="primary"
            size="small"
            class="!h-8 !rounded-md !px-4 font-bold"
            :loading="saving"
            @click="emit('save')"
          >
            <div class="i-ep-check mr-1 font-bold"></div>
            保存
          </el-button>
        </div>
      </div>

      <div class="flex-1 overflow-hidden relative">
        <vue-monaco-editor
          v-if="visible"
          v-model:value="contentValue"
          :theme="isDark ? 'vs-dark' : 'vs'"
          :language="editorLanguage"
          :options="editorOptions"
          class="h-full w-full"
          @mount="handleEditorMount"
        />
      </div>

      <div
        v-if="isMobile"
        class="shrink-0 bg-[var(--editor-toolbar-bg)] border-t border-[var(--editor-border)] flex items-center px-2 py-3 gap-1.5 overflow-x-auto no-scrollbar"
      >
        <button
          type="button"
          class="h-9 shrink-0 rounded-md bg-black/5 px-3 text-[10px] font-bold text-default flex-center transition-colors active:bg-primary dark:bg-white/5"
          @click="triggerCmd('editor.action.indentLines')"
        >
          Tab
        </button>
        <button
          v-for="sym in toolbarKeys"
          :key="sym"
          type="button"
          class="h-9 w-9 shrink-0 rounded-md bg-black/5 font-mono text-sm text-default flex-center transition-colors active:bg-primary dark:bg-white/5"
          @click="insertText(sym)"
        >
          {{ sym }}
        </button>
      </div>
    </div>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { computed, defineAsyncComponent, ref } from "vue";
import type * as Monaco from "monaco-editor";
import type { FileItem } from "../../../../composables/useFileOperations";
import {
  getLanguageConfig,
  getLanguageFromFilename,
} from "../../../../utils/editor";
import ResponsiveDialog from "../../../../components/common/ResponsiveDialog.vue";

const VueMonacoEditor = defineAsyncComponent(() =>
  import("@guolao/vue-monaco-editor").then((mod) => mod.VueMonacoEditor),
);

const props = defineProps<{
  content: string;
  currentFile: FileItem | null;
  isDark: boolean;
  isMobile: boolean;
  saving: boolean;
  visible: boolean;
}>();

const emit = defineEmits<{
  (event: "save"): void;
  (event: "update:content", content: string): void;
  (event: "update:visible", visible: boolean): void;
}>();

const toolbarKeys = [
  "=",
  ":",
  "(",
  ")",
  "{",
  "}",
  "[",
  "]",
  '"',
  "'",
  "-",
  "_",
  "$",
];

const editorInstance = ref<Monaco.editor.IStandaloneCodeEditor | null>(null);

const visibleValue = computed({
  get: () => props.visible,
  set: (visible: boolean) => emit("update:visible", visible),
});

const contentValue = computed({
  get: () => props.content,
  set: (content: string) => emit("update:content", content),
});

const editorLanguage = computed(() => {
  if (!props.currentFile) return "plaintext";
  return getLanguageFromFilename(props.currentFile.name);
});

const editorOptions = computed<Monaco.editor.IStandaloneEditorConstructionOptions>(() => {
  const config = getLanguageConfig(editorLanguage.value);
  return {
    automaticLayout: true,
    minimap: { enabled: false },
    fontSize: props.isMobile ? 15 : 14,
    scrollBeyondLastLine: false,
    wordWrap: props.isMobile ? "off" : "on",
    fontFamily:
      "'JetBrains Mono', 'Fira Code', 'Consolas', 'Monaco', 'Andale Mono', 'Ubuntu Mono', monospace",
    lineNumbersMinChars: props.isMobile ? 3 : 5,
    padding: { top: 10, bottom: 10 },
    renderLineHighlight: "line" as const,
    folding: !props.isMobile,
    tabSize: config.tabSize,
    insertSpaces: config.insertSpaces,
    cursorBlinking: "smooth" as const,
    smoothScrolling: true,
  };
});

const handleEditorMount = (
  editor: Monaco.editor.IStandaloneCodeEditor,
  monaco: typeof import("monaco-editor"),
) => {
  editorInstance.value = editor;
  editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
    emit("save");
  });
};

const insertText = (text: string) => {
  if (!editorInstance.value) return;
  const editor = editorInstance.value;
  const selection = editor.getSelection();
  if (!selection) return;
  editor.executeEdits("toolbar", [
    {
      range: selection,
      text,
      forceMoveMarkers: true,
    },
  ]);
  editor.focus();
};

const triggerCmd = (cmd: string) => {
  if (!editorInstance.value) return;
  editorInstance.value.trigger("keyboard", cmd, null);
  editorInstance.value.focus();
};
</script>
