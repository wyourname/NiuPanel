import {
  computed,
  nextTick,
  ref,
  watch,
  type ComputedRef,
  type Ref,
} from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { useAppStore } from "../stores/app";
import { useMobileBackCloseAction } from "./useMobileBackCloseAction";
import { useTaskScriptEditorConfig } from "./useTaskScriptEditorConfig";
import {
  isTaskScriptFileMode,
  readTaskScriptContent,
  writeTaskScriptContent,
} from "../utils/taskScriptContent";
import type { TaskScriptEditorRef } from "./taskPageTypes";
import type { Task } from "@/types";

type UseTaskScriptWorkspaceOptions = {
  currentScriptTask: Ref<Task | null>;
  currentTask: ComputedRef<Task | undefined>;
  isFileMode: Ref<boolean>;
  saveMobileScript: () => Promise<void>;
  scriptEditorContent: Ref<string>;
  scriptEditorVisible: Ref<boolean>;
};

export function useTaskScriptWorkspace({
  currentScriptTask,
  currentTask,
  isFileMode,
  saveMobileScript,
  scriptEditorContent,
  scriptEditorVisible,
}: UseTaskScriptWorkspaceOptions) {
  const appStore = useAppStore();

  const scriptContent = ref("");
  const scriptLoading = ref(false);
  const scriptSaving = ref(false);
  const scriptEditorInstance = ref<TaskScriptEditorRef | null>(null);
  const editorWordWrap = ref(true);
  const scriptEditorReady = ref(false);
  const originalScriptContent = ref("");

  const {
    dialogEditorLanguage,
    dialogEditorOptions,
    scriptEditorOptions,
    scriptLanguage,
  } = useTaskScriptEditorConfig({
    currentScriptTask,
    currentTask,
    editorWordWrap,
    isFileMode,
  });

  const isScriptModified = computed(
    () =>
      originalScriptContent.value !== "" &&
      originalScriptContent.value !== scriptEditorContent.value,
  );

  const onScriptDrawerOpened = () => {
    nextTick(() => {
      setTimeout(() => {
        scriptEditorReady.value = true;
      }, 100);
    });
  };

  const resetScriptEditorReady = () => {
    scriptEditorReady.value = false;
  };

  const closeScriptEditor = async () => {
    if (isScriptModified.value) {
      try {
        await ElMessageBox.confirm("脚本已修改，是否保存？", "提示", {
          confirmButtonText: "保存",
          cancelButtonText: "不保存",
          distinguishCancelAndClose: true,
          type: "warning",
        });
        await saveMobileScript();
      } catch (action) {
        if (action === "close") return;
      }
    }
    scriptEditorVisible.value = false;
  };

  useMobileBackCloseAction({
    appStore,
    visible: scriptEditorVisible,
    close: async () => {
      await closeScriptEditor();
      return !scriptEditorVisible.value;
    },
  });

  watch(scriptEditorVisible, (visible) => {
    if (visible) {
      originalScriptContent.value = scriptEditorContent.value;
    } else {
      originalScriptContent.value = "";
    }
  });

  const toggleEditorWordWrap = () => {
    editorWordWrap.value = !editorWordWrap.value;
    scriptEditorInstance.value?.updateOptions?.({
      wordWrap: editorWordWrap.value ? "on" : "off",
    });
  };

  const handleScriptEditorMount = (editor: TaskScriptEditorRef) => {
    scriptEditorInstance.value = editor;
  };

  const loadScriptContent = async () => {
    if (!currentTask.value) return;
    scriptLoading.value = true;
    isFileMode.value = isTaskScriptFileMode(currentTask.value);
    try {
      scriptContent.value = await readTaskScriptContent(currentTask.value);
    } catch {
      scriptContent.value = "";
    } finally {
      scriptLoading.value = false;
    }
  };

  const saveScriptContent = async () => {
    if (!currentTask.value) return;
    scriptSaving.value = true;
    try {
      const content = scriptContent.value.replace(/\r\n/g, "\n");
      await writeTaskScriptContent(currentTask.value, content, isFileMode.value);
      ElMessage.success("Saved");
    } finally {
      scriptSaving.value = false;
    }
  };

  return {
    dialogEditorLanguage,
    dialogEditorOptions,
    editorWordWrap,
    handleScriptEditorMount,
    loadScriptContent,
    onScriptDrawerOpened,
    resetScriptEditorReady,
    saveScriptContent,
    scriptContent,
    scriptEditorInstance,
    scriptEditorOptions,
    scriptEditorReady,
    scriptLanguage,
    scriptLoading,
    scriptSaving,
    closeScriptEditor,
    toggleEditorWordWrap,
  };
}
