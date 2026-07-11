import { reactive } from "vue";
import { ElMessage } from "element-plus";
import useClipboard from "vue-clipboard3";
import type { useHaptics } from "./useHaptics";

type UseVariableValueVisibilityOptions = {
  haptics: ReturnType<typeof useHaptics>;
};

export function useVariableValueVisibility({
  haptics,
}: UseVariableValueVisibilityOptions) {
  const visibleValues = reactive<Record<number, boolean>>({});
  const { toClipboard } = useClipboard();

  const isValueVisible = (id: number) => !!visibleValues[id];

  const toggleValueVisibility = (id: number) => {
    haptics.impact();
    visibleValues[id] = !visibleValues[id];
  };

  const copyValue = async (value: string) => {
    try {
      haptics.impact();
      await toClipboard(value);
      ElMessage.success("Copied");
    } catch {
      // Clipboard failures are surfaced by the browser; keep UI unchanged.
    }
  };

  return {
    copyValue,
    isValueVisible,
    toggleValueVisibility,
    visibleValues,
  };
}
