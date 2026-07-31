import { computed, type ComputedRef, type Ref } from "vue";
import { useAppStore } from "../stores/app";
import { getLanguageConfig, getLanguageFromFilename } from "../utils/editor";
import type { Task } from "@/types";
import { ENV_LANGUAGE_MAP } from "./useTaskPresentation";

type UseTaskScriptEditorConfigOptions = {
  currentScriptTask: Ref<Task | null>;
  currentTask: ComputedRef<Task | undefined>;
  editorWordWrap: Ref<boolean>;
  isFileMode: Ref<boolean>;
};

export function useTaskScriptEditorConfig({
  currentScriptTask,
  currentTask,
  editorWordWrap,
  isFileMode,
}: UseTaskScriptEditorConfigOptions) {
  const appStore = useAppStore();

  const scriptLanguage = computed(() => {
    if (isFileMode.value && currentTask.value?.path) {
      return getLanguageFromFilename(currentTask.value.path);
    }
    const envType = currentTask.value?.env_type;
    return (envType && ENV_LANGUAGE_MAP[envType]) || "shell";
  });

  const scriptEditorOptions = computed(() => {
    const config = getLanguageConfig(scriptLanguage.value);
    return {
      automaticLayout: true,
      minimap: { enabled: false },
      fontSize: appStore.isMobile ? 14 : 14,
      fontFamily: "'JetBrains Mono', 'Fira Code', 'Consolas', monospace",
      lineNumbers: "on" as const,
      lineNumbersMinChars: appStore.isMobile ? 3 : 5,
      renderLineHighlight: "line" as const,
      scrollBeyondLastLine: false,
      wordWrap: "on" as const,
      padding: {
        top: appStore.isMobile ? 8 : 12,
        bottom: appStore.isMobile ? 8 : 12,
      },
      tabSize: config.tabSize,
      insertSpaces: config.insertSpaces,
      folding: !appStore.isMobile,
      cursorBlinking: "smooth" as const,
      smoothScrolling: true,
    };
  });

  const dialogEditorLanguage = computed(() => {
    if (currentScriptTask.value?.path) {
      return getLanguageFromFilename(currentScriptTask.value.path);
    }
    const envType = currentScriptTask.value?.env_type;
    return (envType && ENV_LANGUAGE_MAP[envType]) || "shell";
  });

  const dialogEditorOptions = computed(() => ({
    automaticLayout: true,
    minimap: { enabled: false },
    fontSize: appStore.isMobile ? 13 : 14,
    fontFamily: "'JetBrains Mono', 'Fira Code', 'Consolas', monospace",
    lineNumbers: "on",
    lineNumbersMinChars: appStore.isMobile ? 3 : 3,
    scrollBeyondLastLine: false,
    wordWrap: editorWordWrap.value ? "on" : "off",
    padding: {
      top: appStore.isMobile ? 12 : 8,
      bottom: appStore.isMobile ? 80 : 8,
    },
    folding: false,
    renderLineHighlight: appStore.isMobile ? "gutter" : "line",
    cursorBlinking: "smooth",
    smoothScrolling: true,
    tabSize: dialogEditorLanguage.value === "python" ? 4 : 2,
    insertSpaces: true,
    lineHeight: appStore.isMobile ? 22 : 20,
    letterSpacing: 0.3,
  }));

  return {
    dialogEditorLanguage,
    dialogEditorOptions,
    scriptEditorOptions,
    scriptLanguage,
  };
}
