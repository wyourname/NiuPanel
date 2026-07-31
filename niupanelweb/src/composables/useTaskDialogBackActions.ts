import { type Ref } from "vue";
import { useAppStore } from "../stores/app";
import { useMobileBackCloseAction } from "./useMobileBackCloseAction";

type UseTaskDialogBackActionsOptions = {
  appStore: ReturnType<typeof useAppStore>;
  logVisible: Ref<boolean>;
  shareVisible: Ref<boolean>;
  variableEditorVisible: Ref<boolean>;
  wizardVisible: Ref<boolean>;
};

export function useTaskDialogBackActions({
  appStore,
  logVisible,
  shareVisible,
  variableEditorVisible,
  wizardVisible,
}: UseTaskDialogBackActionsOptions) {
  useMobileBackCloseAction({
    appStore,
    visible: wizardVisible,
    close: () => {
      wizardVisible.value = false;
    },
  });
  useMobileBackCloseAction({
    appStore,
    visible: logVisible,
    close: () => {
      logVisible.value = false;
    },
  });
  useMobileBackCloseAction({
    appStore,
    visible: shareVisible,
    close: () => {
      shareVisible.value = false;
    },
  });
  useMobileBackCloseAction({
    appStore,
    visible: variableEditorVisible,
    close: () => {
      variableEditorVisible.value = false;
    },
  });
}
