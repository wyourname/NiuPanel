import type { Ref } from "vue";
import type { TaskLogViewerRef } from "./taskPageTypes";

export function useTaskLogViewerBridge(
  logViewerRef: Ref<TaskLogViewerRef | null>,
): Required<TaskLogViewerRef> {
  return {
    clear: () => {
      logViewerRef.value?.clear?.();
    },
    init: (loader) => {
      logViewerRef.value?.init?.(loader);
    },
    reset: () => {
      logViewerRef.value?.reset?.();
    },
    scrollToBottom: () => {
      logViewerRef.value?.scrollToBottom?.();
    },
    setSearch: (query, jumpToNext) => {
      logViewerRef.value?.setSearch?.(query, jumpToNext);
    },
    toggleWrap: () => {
      logViewerRef.value?.toggleWrap?.();
    },
    write: (data) => {
      logViewerRef.value?.write?.(data);
    },
    writeln: (data) => {
      logViewerRef.value?.writeln?.(data);
    },
  };
}
