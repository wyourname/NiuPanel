import { computed, ref, type Ref } from "vue";
import { useDraggable } from "@vueuse/core";

type UseTaskMobileLogDrawerStateOptions = {
  modelValue: Ref<boolean>;
  onRefreshTimeline: () => void;
  onSelectTimeline: (runId: number | null) => void;
  onUpdateVisible: (visible: boolean) => void;
};

export function useTaskMobileLogDrawerState({
  modelValue,
  onRefreshTimeline,
  onSelectTimeline,
  onUpdateVisible,
}: UseTaskMobileLogDrawerStateOptions) {
  const drawerVisible = computed({
    get: () => modelValue.value,
    set: onUpdateVisible,
  });

  const showTimeline = ref(false);
  const mobileWidgetRef = ref<HTMLElement | null>(null);
  const { x: mobileWidgetX, y: mobileWidgetY } = useDraggable(
    mobileWidgetRef,
    {
      initialValue: { x: 16, y: 16 },
      preventDefault: true,
    },
  );

  const toggleTimeline = () => {
    showTimeline.value = !showTimeline.value;

    if (showTimeline.value) {
      onRefreshTimeline();
    }
  };

  const handleTimelineSelect = (runId: number | null) => {
    onSelectTimeline(runId);
    showTimeline.value = false;
  };

  return {
    drawerVisible,
    handleTimelineSelect,
    mobileWidgetRef,
    mobileWidgetX,
    mobileWidgetY,
    showTimeline,
    toggleTimeline,
  };
}
