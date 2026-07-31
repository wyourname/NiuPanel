import { onBeforeUnmount, ref, type Ref } from "vue";
import { ElMessage } from "element-plus";
import * as variableApi from "../api/variable";
import { moveItem } from "../utils/persistedOrder";
import type { useHaptics } from "./useHaptics";
import type { VariablePageRow } from "./useVariablePageData";

type UseVariableReorderOptions = {
  activeTab: Ref<string>;
  getScopedTaskId: () => number | null;
  hasMore: Ref<boolean>;
  haptics: ReturnType<typeof useHaptics>;
  searchQuery: Ref<string>;
  variables: Ref<VariablePageRow[]>;
};

export function useVariableReorder({
  activeTab,
  getScopedTaskId,
  hasMore,
  haptics,
  searchQuery,
  variables,
}: UseVariableReorderOptions) {
  const dragIndex = ref<number | null>(null);
  const dragOverIndex = ref<number | null>(null);
  const touchDragIndex = ref<number | null>(null);
  const longPressTimer = ref<ReturnType<typeof setTimeout> | null>(null);
  const isLongPressActive = ref(false);
  const touchOriginalItems = ref<VariablePageRow[] | null>(null);

  const persistOrder = async () => {
    const taskId = getScopedTaskId();
    await variableApi.reorderVariables({
      task_id: taskId ?? undefined,
      scope: activeTab.value,
      ids: variables.value.map((item) => item.id),
    });
    ElMessage.success(taskId ? "任务内部排序已保存" : "全局排序已保存");
  };

  const clearDragState = () => {
    dragIndex.value = null;
    dragOverIndex.value = null;
  };

  const clearLongPressTimer = () => {
    if (longPressTimer.value) {
      clearTimeout(longPressTimer.value);
      longPressTimer.value = null;
    }
  };

  const handleDragStart = (index: number) => {
    if (searchQuery.value || hasMore.value) {
      if (hasMore.value) ElMessage.warning("请先加载全部变量，再进行排序");
      return;
    }
    dragIndex.value = index;
  };

  const handleDragOver = (index: number) => {
    if (searchQuery.value) return;
    dragOverIndex.value = index;
  };

  const handleDrop = async (index: number) => {
    if (dragIndex.value === null) return;

    const previousItems = [...variables.value];
    variables.value = moveItem(variables.value, dragIndex.value, index);
    clearDragState();

    try {
      await persistOrder();
    } catch {
      variables.value = previousItems;
      ElMessage.error("排序保存失败");
    }
  };

  const handleDragEnd = () => {
    clearDragState();
  };

  const handleTouchStart = (_event: TouchEvent, index: number) => {
    if (searchQuery.value || hasMore.value) {
      if (hasMore.value) ElMessage.warning("请先加载全部变量，再进行排序");
      return;
    }

    isLongPressActive.value = false;
    touchOriginalItems.value = [...variables.value];
    clearLongPressTimer();
    longPressTimer.value = setTimeout(() => {
      isLongPressActive.value = true;
      touchDragIndex.value = index;
      haptics.impact();
    }, 350);
  };

  const handleTouchMove = (event: TouchEvent) => {
    if (!isLongPressActive.value) {
      clearLongPressTimer();
      return;
    }
    if (touchDragIndex.value === null) return;
    if (event.cancelable) event.preventDefault();

    const touch = event.touches[0];
    const element = document.elementFromPoint(touch.clientX, touch.clientY);
    if (!element) return;

    const card = element.closest(".variable-card");
    if (!card) return;

    const indexAttr = card.getAttribute("data-index");
    if (indexAttr === null) return;

    const overIndex = parseInt(indexAttr, 10);
    if (overIndex === touchDragIndex.value) return;

    const rect = card.getBoundingClientRect();
    const middleY = rect.top + rect.height / 2;
    const isMovingDown = overIndex > touchDragIndex.value;

    if (isMovingDown && touch.clientY < middleY) return;
    if (!isMovingDown && touch.clientY > middleY) return;

    variables.value = moveItem(
      variables.value,
      touchDragIndex.value,
      overIndex,
    );
    touchDragIndex.value = overIndex;
  };

  const handleTouchEnd = async () => {
    clearLongPressTimer();

    if (isLongPressActive.value && touchDragIndex.value !== null) {
      try {
        await persistOrder();
      } catch {
        if (touchOriginalItems.value) {
          variables.value = touchOriginalItems.value;
        }
        ElMessage.error("排序保存失败");
      }
    }

    touchDragIndex.value = null;
    touchOriginalItems.value = null;
    isLongPressActive.value = false;
  };

  onBeforeUnmount(clearLongPressTimer);

  return {
    dragOverIndex,
    handleDragEnd,
    handleDragOver,
    handleDragStart,
    handleDrop,
    handleTouchEnd,
    handleTouchMove,
    handleTouchStart,
    touchDragIndex,
  };
}
