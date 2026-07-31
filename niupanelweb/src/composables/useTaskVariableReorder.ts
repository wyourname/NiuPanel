import {
  computed,
  onBeforeUnmount,
  ref,
  type ComputedRef,
  type Ref,
} from "vue";
import { ElMessage } from "element-plus";
import * as variableApi from "../api/variable";
import {
  collectPersistedTaskVariableIds,
  type TaskVariableRow,
} from "./taskVariableEditorHelpers";

type UseTaskVariableReorderOptions = {
  hasDraftRows: ComputedRef<boolean>;
  loading: Ref<boolean>;
  saving: Ref<boolean>;
  taskId: ComputedRef<number>;
  variables: Ref<TaskVariableRow[]>;
};

const moveItem = <T,>(list: T[], from: number, to: number) => {
  const next = [...list];
  const [item] = next.splice(from, 1);
  next.splice(to, 0, item);
  return next;
};

const isValidIndex = (index: number, length: number) => {
  return Number.isInteger(index) && index >= 0 && index < length;
};

export function useTaskVariableReorder({
  hasDraftRows,
  loading,
  saving,
  taskId,
  variables,
}: UseTaskVariableReorderOptions) {
  const dragIndex = ref<number | null>(null);
  const dragOverIndex = ref<number | null>(null);
  const isReordering = ref(false);

  const canSort = computed(
    () =>
      !saving.value &&
      !loading.value &&
      !isReordering.value &&
      !hasDraftRows.value,
  );

  const clearDragState = () => {
    dragIndex.value = null;
    dragOverIndex.value = null;
  };

  const removePointerListeners = () => {
    window.removeEventListener("pointermove", handlePointerMove);
    window.removeEventListener("pointerup", handlePointerUp);
    window.removeEventListener("pointercancel", handlePointerUp);
  };

  const handlePointerMove = (event: PointerEvent) => {
    const target = document
      .elementFromPoint(event.clientX, event.clientY)
      ?.closest<HTMLElement>("[data-variable-index]");

    if (!target) return;

    const nextIndex = Number(target.dataset.variableIndex);

    if (isValidIndex(nextIndex, variables.value.length)) {
      dragOverIndex.value = nextIndex;
    }
  };

  const persistReorder = async (from: number, to: number) => {
    const previousItems = [...variables.value];
    const nextItems = moveItem(previousItems, from, to);
    const orderedIds = collectPersistedTaskVariableIds(nextItems);

    if (orderedIds === null) {
      ElMessage.warning("请先保存新增变量，再进行拖拽排序");
      return;
    }

    variables.value = nextItems;
    isReordering.value = true;

    try {
      await variableApi.reorderVariables({
        task_id: taskId.value,
        ids: orderedIds,
      });
      ElMessage.success("变量顺序已保存");
    } catch {
      variables.value = previousItems;
      ElMessage.error("变量顺序保存失败");
    } finally {
      isReordering.value = false;
    }
  };

  const handlePointerUp = async () => {
    removePointerListeners();

    const from = dragIndex.value;
    const to = dragOverIndex.value;
    clearDragState();

    if (
      from === null ||
      to === null ||
      from === to ||
      !isValidIndex(from, variables.value.length) ||
      !isValidIndex(to, variables.value.length)
    ) {
      return;
    }

    await persistReorder(from, to);
  };

  const startDrag = (index: number, event: PointerEvent) => {
    if (!canSort.value) {
      if (hasDraftRows.value) {
        ElMessage.warning("请先保存新增变量，再进行拖拽排序");
      }
      return;
    }

    event.preventDefault();
    dragIndex.value = index;
    dragOverIndex.value = index;

    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp);
    window.addEventListener("pointercancel", handlePointerUp);
  };

  onBeforeUnmount(removePointerListeners);

  return {
    canSort,
    dragIndex,
    dragOverIndex,
    isReordering,
    startDrag,
  };
}
