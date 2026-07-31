import { ref } from "vue";
import { ElMessage, ElNotification } from "element-plus";
import * as gitApi from "@/api/git";
import type { DiscoveredTask, GitRepo } from "@/api/git";

export function useGitTaskImport() {
  const scanDialogVisible = ref(false);
  const scanning = ref(false);
  const importing = ref(false);
  const currentScanRepoId = ref<number | null>(null);
  const discoveredTasks = ref<DiscoveredTask[]>([]);
  const selectedDiscoveredTasks = ref<DiscoveredTask[]>([]);
  const selectAll = ref(false);

  const openScan = async (row: GitRepo) => {
    currentScanRepoId.value = row.id;
    scanDialogVisible.value = true;
    scanning.value = true;
    discoveredTasks.value = [];
    selectedDiscoveredTasks.value = [];
    selectAll.value = false;
    try {
      const res = await gitApi.scanRepoTasks(row.id);
      discoveredTasks.value = res.data;
      selectedDiscoveredTasks.value = res.data;
      selectAll.value = res.data.length > 0;
    } catch {
      ElMessage.error("扫描任务失败，请确保仓库已同步");
    } finally {
      scanning.value = false;
    }
  };

  const handleTaskSelectionChange = (val: DiscoveredTask[]) => {
    selectedDiscoveredTasks.value = val;
    selectAll.value =
      val.length === discoveredTasks.value.length &&
      discoveredTasks.value.length > 0;
  };

  const handleSelectAllChange = (val: boolean) => {
    if (val) {
      selectedDiscoveredTasks.value = [...discoveredTasks.value];
    } else {
      selectedDiscoveredTasks.value = [];
    }
    selectAll.value = val;
  };

  const handleImport = async () => {
    if (!currentScanRepoId.value || selectedDiscoveredTasks.value.length === 0) {
      return;
    }

    importing.value = true;
    try {
      await gitApi.importRepoTasks(
        currentScanRepoId.value,
        selectedDiscoveredTasks.value,
      );
      ElNotification({
        title: "导入成功",
        message: `成功导入了 ${selectedDiscoveredTasks.value.length} 个任务`,
        type: "success",
      });
      scanDialogVisible.value = false;
    } catch {
      // Request interceptor shows the error.
    } finally {
      importing.value = false;
    }
  };

  return {
    discoveredTasks,
    handleImport,
    handleSelectAllChange,
    handleTaskSelectionChange,
    importing,
    openScan,
    scanDialogVisible,
    scanning,
    selectAll,
    selectedDiscoveredTasks,
  };
}
