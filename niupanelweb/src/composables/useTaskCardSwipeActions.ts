import { onBeforeUnmount, ref, type Ref } from "vue";
import { useSwipe } from "@vueuse/core";
import { useHaptics } from "./useHaptics";
import type { Task } from "@/types";

export type TaskCardSwipeAction =
  | "delete"
  | "disable"
  | "enable"
  | "more"
  | "run"
  | "stop";

type UseTaskCardSwipeActionsOptions = {
  isSelected: Ref<boolean>;
  onAction: (action: TaskCardSwipeAction, task: Task) => void;
  onEnterSelection: (task: Task) => void;
  onOpenLogs: (task: Task) => void;
  onSelectionChange: (task: Task, selected: boolean) => void;
  selectionMode: Ref<boolean>;
  task: Ref<Task>;
};

export function useTaskCardSwipeActions({
  isSelected,
  onAction,
  onEnterSelection,
  onOpenLogs,
  onSelectionChange,
  selectionMode,
  task,
}: UseTaskCardSwipeActionsOptions) {
  const haptics = useHaptics();
  const cardRef = ref<HTMLElement | null>(null);
  const offset = ref(0);
  const isSwiping = ref(false);
  const swipeDirection = ref<"horizontal" | "vertical" | null>(null);
  const longPressTimer = ref<ReturnType<typeof setTimeout> | null>(null);

  const maxLeft = 80;
  const maxRight = 210;

  const { lengthX, lengthY } = useSwipe(cardRef, {
    passive: true,
    onSwipeStart() {
      isSwiping.value = true;
      swipeDirection.value = null;
    },
    onSwipe() {
      const absX = Math.abs(lengthX.value);
      const absY = Math.abs(lengthY.value);

      if (!swipeDirection.value && (absX > 10 || absY > 10)) {
        swipeDirection.value = absX > absY ? "horizontal" : "vertical";
      }

      if (swipeDirection.value === "horizontal") {
        const x = -lengthX.value;
        if (x > 0) {
          offset.value = x > maxLeft ? maxLeft + (x - maxLeft) * 0.15 : x;
        } else {
          offset.value =
            x < -maxRight ? -maxRight + (x + maxRight) * 0.15 : x;
        }
      } else {
        offset.value = 0;
      }
    },
    onSwipeEnd() {
      isSwiping.value = false;

      if (swipeDirection.value === "horizontal") {
        if (offset.value > 40) offset.value = maxLeft;
        else if (offset.value < -50) offset.value = -maxRight;
        else offset.value = 0;

        if (offset.value !== 0) haptics.impact();
      } else {
        offset.value = 0;
      }

      swipeDirection.value = null;
    },
  });

  const handleAction = (action: TaskCardSwipeAction) => {
    haptics.impact();
    const actionTask = task.value;
    offset.value = 0;

    setTimeout(() => {
      onAction(action, actionTask);
    }, 400);
  };

  const startLongPress = () => {
    if (selectionMode.value || offset.value !== 0) return;

    longPressTimer.value = setTimeout(() => {
      haptics.notification();
      onEnterSelection(task.value);
    }, 600);
  };

  const cancelLongPress = () => {
    if (longPressTimer.value) {
      clearTimeout(longPressTimer.value);
      longPressTimer.value = null;
    }
  };

  onBeforeUnmount(cancelLongPress);

  const handleSelectionChange = (selected: boolean) => {
    haptics.selectionChanged();
    onSelectionChange(task.value, selected);
  };

  const handleCardClick = () => {
    if (offset.value !== 0) {
      offset.value = 0;
      return;
    }

    if (selectionMode.value) {
      handleSelectionChange(!isSelected.value);
    } else {
      haptics.impact();
      onOpenLogs(task.value);
    }
  };

  return {
    cancelLongPress,
    cardRef,
    handleAction,
    handleCardClick,
    handleSelectionChange,
    isSwiping,
    maxLeft,
    maxRight,
    offset,
    startLongPress,
  };
}
