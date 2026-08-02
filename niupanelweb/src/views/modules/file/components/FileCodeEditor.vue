<template>
  <div class="flex h-full min-h-0 flex-1 flex-col overflow-hidden bg-[var(--editor-bg)]">
    <div class="relative min-h-0 flex-1 overflow-hidden">
      <vue-monaco-editor
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
      class="file-editor-mobile-toolbar shrink-0 border-t border-[var(--editor-border)] bg-[var(--editor-toolbar-bg)] px-2 pt-3 flex items-center gap-1.5 overflow-x-auto no-scrollbar"
    >
      <button
        type="button"
        class="h-11 shrink-0 cursor-pointer rounded-md bg-black/5 px-3 text-[10px] font-bold text-default flex-center transition-colors active:bg-primary dark:bg-white/5"
        @click="triggerCommand('editor.action.indentLines')"
      >
        Tab
      </button>
      <button
        v-for="symbol in toolbarKeys"
        :key="symbol"
        type="button"
        class="h-11 w-11 shrink-0 cursor-pointer rounded-md bg-black/5 font-mono text-sm text-default flex-center transition-colors active:bg-primary dark:bg-white/5"
        @click="insertText(symbol)"
      >
        {{ symbol }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, defineAsyncComponent, onBeforeUnmount, ref } from "vue";
import type * as Monaco from "monaco-editor";
import { getLanguageConfig, getLanguageFromFilename } from "@/utils/editor";

const VueMonacoEditor = defineAsyncComponent(() =>
  import("@guolao/vue-monaco-editor").then((module) => module.VueMonacoEditor),
);

const props = defineProps<{
  content: string;
  fileName: string;
  isDark: boolean;
  isMobile: boolean;
}>();

const emit = defineEmits<{
  (event: "save"): void;
  (event: "update:content", content: string): void;
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

const contentValue = computed({
  get: () => props.content,
  set: (content: string) => emit("update:content", content),
});

const editorLanguage = computed(() => getLanguageFromFilename(props.fileName));

const editorOptions = computed<Monaco.editor.IStandaloneEditorConstructionOptions>(
  () => {
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
      renderLineHighlight: "line",
      folding: !props.isMobile,
      tabSize: config.tabSize,
      insertSpaces: config.insertSpaces,
      cursorBlinking: "smooth",
      smoothScrolling: true,
    };
  },
);

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
  const editor = editorInstance.value;
  const selection = editor?.getSelection();
  if (!editor || !selection) return;

  editor.executeEdits("file-editor-toolbar", [
    {
      range: selection,
      text,
      forceMoveMarkers: true,
    },
  ]);
  editor.focus();
};

const triggerCommand = (command: string) => {
  const editor = editorInstance.value;
  if (!editor) return;

  editor.trigger("keyboard", command, null);
  editor.focus();
};

onBeforeUnmount(() => {
  editorInstance.value = null;
});
</script>

<style scoped>
.file-editor-mobile-toolbar {
  padding-bottom: calc(12px + env(safe-area-inset-bottom, 0px));
}
</style>
