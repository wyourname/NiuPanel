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
  const touchMoved = ref(false);

  const clearPressTimer = () => {
    if (pressTimer.value) {
      clearTimeout(pressTimer.value);
      pressTimer.value = null;
    }
  };

  const handleTouchStart = (row: FileItem) => {
    if (!options.isMobile()) return;
    clearPressTimer();
    touchMoved.value = false;
    pressTimer.value = setTimeout(() => {
      haptics.notification();
      options.toggleSelection(row);
    }, 600);
  };

  const handleTouchEnd = () => {
    clearPressTimer();
  };

  const handleTouchMove = () => {
    clearPressTimer();
    touchMoved.value = true;
  };

  const handleItemClickMobile = (row: FileItem) => {
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
