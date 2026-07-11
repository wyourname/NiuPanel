import { onScopeDispose, watch, type Ref } from "vue";
import { useAppStore } from "../stores/app";

type BackCloseResult = {
  handled: boolean;
  remove: boolean;
};

type UseMobileBackCloseActionOptions = {
  appStore: ReturnType<typeof useAppStore>;
  close: () => boolean | void | Promise<boolean | void>;
  visible: Ref<boolean>;
};

export function useMobileBackCloseAction({
  appStore,
  close,
  visible,
}: UseMobileBackCloseActionOptions) {
  let disposeBackAction: (() => void) | null = null;

  const clearBackAction = () => {
    disposeBackAction?.();
    disposeBackAction = null;
  };

  watch(visible, (isVisible) => {
    if (!appStore.isMobile) {
      clearBackAction();
      return;
    }

    if (isVisible && !disposeBackAction) {
      disposeBackAction = appStore.pushBackAction(async () => {
        if (!visible.value) {
          clearBackAction();
          return {
            handled: false,
            remove: true,
          } satisfies BackCloseResult;
        }

        const result = await close();
        const handled = result ?? !visible.value;
        if (handled) {
          disposeBackAction = null;
        }

        return {
          handled: true,
          remove: handled,
        } satisfies BackCloseResult;
      });
    } else if (!isVisible) {
      clearBackAction();
    }
  });

  onScopeDispose(clearBackAction);
}
