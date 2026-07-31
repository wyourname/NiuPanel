import { reactive } from "vue";
import { ElMessage } from "element-plus";
import useClipboard from "vue-clipboard3";
import type { useHaptics } from "./useHaptics";

type UseVariableValueVisibilityOptions = {
  haptics: ReturnType<typeof useHaptics>;
  resolveValue: (id: number) => Promise<string>;
};

export function useVariableValueVisibility({
  haptics,
  resolveValue,
}: UseVariableValueVisibilityOptions) {
  const visibleValues = reactive<Record<number, boolean>>({});
  const loadingValues = reactive<Record<number, boolean>>({});
  const { toClipboard } = useClipboard();

  const isValueVisible = (id: number) => !!visibleValues[id];
  const isValueLoading = (id: number) => !!loadingValues[id];
  const resetValueVisibility = () => {
    for (const id of Object.keys(visibleValues)) {
      delete visibleValues[Number(id)];
    }
    for (const id of Object.keys(loadingValues)) {
      delete loadingValues[Number(id)];
    }
  };

  const toggleValueVisibility = async (id: number) => {
    haptics.impact();
    if (loadingValues[id]) return;
    if (!visibleValues[id]) {
      loadingValues[id] = true;
      try {
        await resolveValue(id);
      } catch {
        ElMessage.error("变量值加载失败");
        return;
      } finally {
        delete loadingValues[id];
      }
    }
    visibleValues[id] = !visibleValues[id];
  };

  const copyValue = async (id: number) => {
    try {
      haptics.impact();
      const value = await resolveValue(id);
      await toClipboard(value);
      ElMessage.success("已复制");
    } catch {
      ElMessage.error("复制失败");
    }
  };

  return {
    copyValue,
    isValueLoading,
    isValueVisible,
    resetValueVisibility,
    toggleValueVisibility,
    visibleValues,
  };
}
