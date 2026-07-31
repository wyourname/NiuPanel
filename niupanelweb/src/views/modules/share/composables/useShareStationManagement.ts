import { ref } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import useClipboard from "vue-clipboard3";
import * as shareApi from "@/api/share";
import type {
  StationConfigPayload,
  StationFile,
  StationStats,
} from "@/types";

const DEFAULT_STATION_URL = "https://station.365676.xyz";

export function useShareStationManagement() {
  const { toClipboard } = useClipboard();
  const stationList = ref<StationFile[]>([]);
  const stationStats = ref<StationStats | null>(null);
  const loadingList = ref(false);
  const currentShare = ref<StationFile | undefined>(undefined);
  const editDialogVisible = ref(false);
  const savingConfig = ref(false);
  const configForm = ref<StationConfigPayload>({
    url: DEFAULT_STATION_URL,
    token: "",
  });

  const fetchStationList = async () => {
    loadingList.value = true;
    try {
      const statsRes = await shareApi.getStationStats();
      stationStats.value = statsRes.data;

      if (stationStats.value.isConfigured) {
        const res = await shareApi.listStationFiles();
        stationList.value = res.data;
      }
    } catch (e) {
      console.error("Failed to fetch station data", e);
    } finally {
      loadingList.value = false;
    }
  };

  const handleSaveConfig = async () => {
    if (!configForm.value.token) {
      return ElMessage.warning("请输入管理员密码");
    }
    savingConfig.value = true;
    try {
      await shareApi.saveStationConfig(configForm.value);
      ElMessage.success("配置已保存");
      await fetchStationList();
    } catch (e) {
    } finally {
      savingConfig.value = false;
    }
  };

  const showStationConfig = () => {
    if (!stationStats.value) return;
    stationStats.value = {
      ...stationStats.value,
      isConfigured: false,
    };
  };

  const openEditDialog = (row: StationFile) => {
    currentShare.value = row;
    editDialogVisible.value = true;
  };

  const handleDelete = (row: StationFile) => {
    ElMessageBox.confirm("确认从中转站彻底删除该资源?", "彻底删除", {
      type: "warning",
      confirmButtonText: "确定删除",
      confirmButtonClass: "el-button--danger",
    }).then(async () => {
      await shareApi.deleteStationFile(row.token);
      void fetchStationList();
      ElMessage.success("已从云端和本地记录中移除");
    });
  };

  const handleUpdateContent = async (row: StationFile) => {
    ElMessageBox.confirm(
      "确定要重新打包当前任务并更新到中转站吗？此操作将覆盖云端文件。",
      "更新内容",
      {
        type: "warning",
        confirmButtonText: "确定更新",
      },
    ).then(async () => {
      const loading = ElMessage({
        type: "info",
        message: "正在打包并上传...",
        duration: 0,
      });
      try {
        await shareApi.updateStationContent(row.token);
        loading.close();
        ElMessage.success("内容更新成功");
        void fetchStationList();
      } catch (error) {
        loading.close();
        ElMessage.error(error instanceof Error ? error.message : "更新失败");
      }
    });
  };

  const copyLink = (token: string) => {
    const baseUrl = configForm.value.url || DEFAULT_STATION_URL;
    const url = `${baseUrl.replace(/\/$/, "")}/share/${token}`;
    toClipboard(url)
      .then(() => ElMessage.success("中转链接已复制"))
      .catch(() => ElMessage.error("复制失败"));
  };

  return {
    configForm,
    copyLink,
    currentShare,
    editDialogVisible,
    fetchStationList,
    handleDelete,
    handleSaveConfig,
    handleUpdateContent,
    loadingList,
    openEditDialog,
    savingConfig,
    showStationConfig,
    stationList,
    stationStats,
  };
}
