import { createSharedComposable } from "@vueuse/core";
import { onMounted, onScopeDispose, readonly, ref } from "vue";
import { getSystemMetrics } from "@/api/overview";
import type { SystemMetrics } from "@/types";

// Share one polling loop between the status bar and any open overview windows.
export const useSystemMetrics = createSharedComposable(() => {
  const metrics = ref<SystemMetrics | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let request: AbortController | undefined;
  let disposed = false;

  const refresh = async () => {
    if (disposed || request) return;
    clearTimeout(timer);
    if (document.hidden) {
      metrics.value = null;
      return;
    }

    request = new AbortController();
    try {
      const response = await getSystemMetrics(request.signal);
      if (!disposed && !document.hidden) metrics.value = response.data;
    } catch {
      metrics.value = null;
    } finally {
      request = undefined;
      if (!disposed && !document.hidden) timer = setTimeout(refresh, 3000);
    }
  };

  const handleVisibility = () => {
    if (document.hidden) {
      clearTimeout(timer);
      metrics.value = null;
    } else {
      void refresh();
    }
  };

  onMounted(() => {
    document.addEventListener("visibilitychange", handleVisibility);
    void refresh();
  });

  onScopeDispose(() => {
    disposed = true;
    clearTimeout(timer);
    request?.abort();
    document.removeEventListener("visibilitychange", handleVisibility);
  });

  return { metrics: readonly(metrics) };
});
