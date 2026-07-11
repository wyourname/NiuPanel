import { nextTick, ref, type Ref } from "vue";
import type {
  TaskDetailTab,
  TaskFocusableRef,
  TaskLogUiEvent,
} from "./taskPageTypes";

type UseTaskLogUiStateOptions = {
  activeDetailTab: Ref<TaskDetailTab>;
};

export function useTaskLogUiState({
  activeDetailTab,
}: UseTaskLogUiStateOptions) {
  const logQrCodeData = ref<string | null>(null);
  const expandLogQr = ref(false);
  const logProgressValue = ref<number>(0);
  const showLogProgress = ref(false);
  const showSearch = ref(false);
  const logSearchQuery = ref("");
  const logSearchInputRef = ref<TaskFocusableRef | null>(null);

  const resetLogArtifacts = () => {
    logQrCodeData.value = null;
    expandLogQr.value = false;
    showLogProgress.value = false;
    logProgressValue.value = 0;
  };

  const handleLogUiEvent = (event: TaskLogUiEvent) => {
    if (event.type === "qrcode" && typeof event.data === "string") {
      logQrCodeData.value = event.data;
    }
    if (event.type === "close_qrcode") {
      logQrCodeData.value = null;
      expandLogQr.value = false;
    }
    if (event.type === "progress" && typeof event.data === "number") {
      showLogProgress.value = true;
      logProgressValue.value = event.data;
    }
    if (event.type === "close_progress") {
      showLogProgress.value = false;
      logProgressValue.value = 0;
    }
  };

  const toggleHeaderSearch = () => {
    showSearch.value = !showSearch.value;
    if (showSearch.value) {
      activeDetailTab.value = "log";
      nextTick(() => logSearchInputRef.value?.focus?.());
    }
  };

  return {
    expandLogQr,
    handleLogUiEvent,
    logProgressValue,
    logQrCodeData,
    logSearchInputRef,
    logSearchQuery,
    resetLogArtifacts,
    showLogProgress,
    showSearch,
    toggleHeaderSearch,
  };
}
