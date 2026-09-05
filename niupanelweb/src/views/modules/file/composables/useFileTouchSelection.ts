import { ref, type Ref } from "vue";
import { useHaptics } from "@/composables/useHaptics";
import type { FileItem } from "@/composables/useFileOperations";

type UseFileTouchSelectionOptions = {
  handleItemClick: (row: FileItem) => void;
  isMobile: () => boolean;
  selectedFiles: Ref<FileItem[]>;
  toggleSelection: (row: FileItem) => void;
};

export function useFileTouchSelection(options: UseFileTouchSelectionOptions) {
  const haptics = useHaptics();
  const pressTimer = ref<ReturnType<typeof setTimeout> | null>(null);
  const clickSuppressionTimer = ref<ReturnType<typeof setTimeout> | null>(null);
  const touchMoved = ref(false);
  const longPressTriggered = ref(false);

  const clearPressTimer = () => {
    if (pressTimer.value) {
      clearTimeout(pressTimer.value);
      pressTimer.value = null;
    }
  };

  const clearClickSuppressionTimer = () => {
    if (clickSuppressionTimer.value) {
      clearTimeout(clickSuppressionTimer.value);
      clickSuppressionTimer.value = null;
    }
  };

  const scheduleClickSuppressionReset = () => {
    clearClickSuppressionTimer();
    clickSuppressionTimer.value = setTimeout(() => {
      longPressTriggered.value = false;
      clickSuppressionTimer.value = null;
    }, 350);
  };

  const handleTouchStart = (row: FileItem) => {
    if (!options.isMobile()) return;
    clearPressTimer();
    clearClickSuppressionTimer();
    touchMoved.value = false;
    longPressTriggered.value = false;
    pressTimer.value = setTimeout(() => {
      longPressTriggered.value = true;
      haptics.notification();
      options.toggleSelection(row);
    }, 600);
  };

  const handleTouchEnd = () => {
    clearPressTimer();
    if (longPressTriggered.value) scheduleClickSuppressionReset();
  };

  const handleTouchMove = () => {
    clearPressTimer();
    touchMoved.value = true;
  };

  const handleItemClickMobile = (row: FileItem) => {
    if (longPressTriggered.value) {
      longPressTriggered.value = false;
      clearClickSuppressionTimer();
      return;
    }
    if (touchMoved.value) return;

    if (options.selectedFiles.value.length > 0) {
      haptics.selectionChanged();
      options.toggleSelection(row);
      return;
    }

    haptics.impact();
    options.handleItemClick(row);
  };

  return {
    handleItemClickMobile,
    handleTouchEnd,
    handleTouchMove,
    handleTouchStart,
  };
}
