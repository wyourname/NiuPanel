import { onMounted, ref } from "vue";
import { ElMessage } from "element-plus";
import { useAppStore } from "@/stores/app";

export function useServerSettings() {
  const appStore = useAppStore();
  const showServerSettings = ref(false);
  const serverUrl = ref("");

  const saveServerSettings = () => {
    let url = serverUrl.value.trim();
    if (url) {
      if (!url.startsWith("http://") && !url.startsWith("https://")) {
        url = `http://${url}`;
      }
      appStore.setServerUrl(url);
    } else {
      appStore.setServerUrl("");
    }

    ElMessage.success("设置已保存");
    setTimeout(() => window.location.reload(), 500);
  };

  onMounted(() => {
    serverUrl.value = appStore.serverUrl;
  });

  return {
    saveServerSettings,
    serverUrl,
    showServerSettings,
  };
}
