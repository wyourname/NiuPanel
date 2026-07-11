import { ref } from "vue";
import { ElMessage, ElMessageBox, ElNotification } from "element-plus";
import * as gitApi from "@/api/git";
import type { GitRepo } from "@/api/git";

export function useGitRepos() {
  const loading = ref(false);
  const repos = ref<GitRepo[]>([]);
  const syncingId = ref<number | null>(null);

  const loadData = async () => {
    loading.value = true;
    try {
      const res = await gitApi.listGitRepos();
      repos.value = res.data;
    } catch {
    } finally {
      loading.value = false;
    }
  };

  const getStatusColor = (status: string | null) => {
    if (status === "Success") return "bg-emerald-500";
    if (status === "Failed") return "bg-rose-500";
    return "bg-gray-400";
  };

  const handleDelete = async (row: GitRepo) => {
    try {
      await ElMessageBox.confirm(
        `确定删除仓库 "${row.name}" 吗？本地同步的文件将保持不变。`,
        "删除确认",
        {
          type: "warning",
          confirmButtonClass: "el-button--danger",
        },
      );
      await gitApi.deleteGitRepo(row.id);
      ElMessage.success("删除成功");
      await loadData();
    } catch {
    }
  };

  const handleSync = async (row: GitRepo) => {
    syncingId.value = row.id;
    try {
      const res = await gitApi.syncGitRepo(row.id);
      if (res.data.success) {
        ElNotification({
          title: "同步成功",
          message: `仓库 [${row.name}] ${res.data.message}`,
          type: "success",
        });
      } else {
        ElMessage.error(res.data.message);
      }
      await loadData();
    } catch {
    } finally {
      syncingId.value = null;
    }
  };

  return {
    getStatusColor,
    handleDelete,
    handleSync,
    loadData,
    loading,
    repos,
    syncingId,
  };
}
