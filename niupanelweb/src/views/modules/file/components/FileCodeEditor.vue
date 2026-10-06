<template>
  <FileMobileCodeEditor v-if="isMobile" v-model:content="contentValue" :file-name="fileName" :is-dark="isDark" @save="emit('save')" />
  <div v-else class="flex h-full min-h-0 flex-1 flex-col overflow-hidden bg-[var(--editor-bg)]">
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

  </div>
</template>

<script setup lang="ts">
import { computed, defineAsyncComponent, onBeforeUnmount, shallowRef } from "vue";
import type * as Monaco from "monaco-editor";
import { getLanguageConfig, getLanguageFromFilename } from "@/utils/editor";
import { createAsyncMonacoEditor } from "@/utils/monaco";

const VueMonacoEditor = createAsyncMonacoEditor();
const FileMobileCodeEditor = defineAsyncComponent(() => import("./FileMobileCodeEditor.vue"));

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

const editorInstance = shallowRef<Monaco.editor.IStandaloneCodeEditor | null>(null);

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

onBeforeUnmount(() => {
  editorInstance.value = null;
});
</script>
