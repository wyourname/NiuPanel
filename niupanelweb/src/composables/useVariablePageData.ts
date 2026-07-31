import { onMounted, onScopeDispose, ref, watch, type Ref } from "vue";
import { useRoute } from "vue-router";
import { ElMessage } from "element-plus";
import { debounce } from "lodash-es";
import * as variableApi from "../api/variable";
import type { VariableSummary } from "@/types";
import type { useHaptics } from "./useHaptics";

export type VariablePageRow = VariableSummary & {
  value?: string;
  task_ids?: number[];
};

type VariablePageTask = {
  id: number;
  name: string;
};

type UseVariablePageDataOptions = {
  clearSelection: () => void;
  haptics: ReturnType<typeof useHaptics>;
  onReset?: () => void;
  variables: Ref<VariablePageRow[]>;
};

export function useVariablePageData({
  clearSelection,
  haptics,
  onReset,
  variables,
}: UseVariablePageDataOptions) {
  const route = useRoute();
  const activeTab = ref("Script");
  const loading = ref(false);
  const tasks = ref<VariablePageTask[]>([]);
  const tasksLoading = ref(false);
  const searchQuery = ref("");
  const currentPage = ref(1);
  const pageSize = ref(24);
  const hasMore = ref(true);

  const getScopedTaskId = () => {
    if (activeTab.value !== "Script") return null;
    const raw = route.query.scope_id;
    if (typeof raw !== "string" || !/^[1-9]\d*$/.test(raw)) return null;
    const parsed = Number(raw);
    return Number.isSafeInteger(parsed) ? parsed : null;
  };

  const fetchTasks = async () => {
    tasksLoading.value = true;
    try {
      const res = await variableApi.getAllTasksSimple();
      tasks.value = res.data || [];
    } finally {
      tasksLoading.value = false;
    }
  };

  const loadData = async (isLoadMore = false) => {
    if (loading.value) return;
    const scopedTaskId = getScopedTaskId();

    if (!isLoadMore) {
      onReset?.();
      currentPage.value = 1;
      variables.value = [];
      hasMore.value = true;
      clearSelection();
    }

    loading.value = true;
    try {
      const res = await variableApi.getVariables({
        scope: activeTab.value,
        scope_id: scopedTaskId ?? undefined,
        key: searchQuery.value || undefined,
        page: currentPage.value,
        page_size: scopedTaskId ? 1000 : pageSize.value,
      });

      const newItems = (res.data.items || []) as VariablePageRow[];
      variables.value = isLoadMore
        ? [...variables.value, ...newItems]
        : newItems;

      hasMore.value = variables.value.length < res.data.total;
      if (hasMore.value) currentPage.value++;
    } catch {
      ElMessage.error("Load failed");
    } finally {
      loading.value = false;
    }
  };

  const handleSearch = debounce(() => {
    loadData();
  }, 300);

  onScopeDispose(() => {
    handleSearch.cancel();
  });

  const loadMore = () => {
    if (!loading.value && hasMore.value) {
      loadData(true);
    }
  };

  const handleTabChange = () => {
    haptics.impact();
    loadData();
  };

  const getTaskNames = (taskIds?: number[]) => {
    if (!taskIds || taskIds.length === 0) return "未绑定任务";
    return taskIds
      .map((id) => tasks.value.find((task) => task.id === id)?.name || `#${id}`)
      .join(", ");
  };

  watch(
    () => route.query.q,
    (newQ) => {
      if (typeof newQ === "string") {
        searchQuery.value = newQ;
        loadData();
      }
    },
  );

  watch(
    () => route.query.scope_id,
    () => {
      loadData();
    },
  );

  onMounted(() => {
    if (typeof route.query.q === "string") searchQuery.value = route.query.q;
    loadData();
    fetchTasks();
  });

  return {
    activeTab,
    currentPage,
    fetchTasks,
    getScopedTaskId,
    getTaskNames,
    handleSearch,
    handleTabChange,
    hasMore,
    loadData,
    loading,
    loadMore,
    pageSize,
    searchQuery,
    tasks,
    tasksLoading,
  };
}
