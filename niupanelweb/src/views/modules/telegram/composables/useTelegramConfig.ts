import { reactive, ref } from "vue";
import { ElMessage } from "element-plus";
import * as telegramApi from "@/api/telegram";
import type { TelegramBotConfig } from "@/api/telegram";

const createDefaultTelegramConfig = (): TelegramBotConfig => ({
  enabled: false,
  token: "",
  admin_chat_id: "",
  proxy_url: "",
  api_base_url: "https://api.telegram.org",
  events: [],
  cf_proxy_enabled: false,
  cf_host: "",
  cf_ip: "104.16.0.0",
  cf_token: "",
  login_2fa: false,
});

const getErrorMessage = (error: unknown, fallback: string) => {
  return error instanceof Error && error.message ? error.message : fallback;
};

export function useTelegramConfig() {
  const config = reactive<TelegramBotConfig>(createDefaultTelegramConfig());
  const form = reactive<TelegramBotConfig>(createDefaultTelegramConfig());
  const showSettings = ref(false);
  const saving = ref(false);
  const testing = ref(false);
  const latency = ref(0);

  const loadConfig = async () => {
    try {
      const res = await telegramApi.getTelegramConfig();
      if (res.data) {
        Object.assign(config, res.data);
        Object.assign(form, res.data);
      }
    } catch {
      ElMessage.error("获取配置失败");
    }
  };

  const handleSave = async () => {
    saving.value = true;
    try {
      await telegramApi.updateTelegramConfig({ ...form });
      Object.assign(config, form);
      ElMessage.success("保存成功");
      showSettings.value = false;
      await refreshLatency();
    } finally {
      saving.value = false;
    }
  };

  const handleTest = async () => {
    testing.value = true;
    try {
      await telegramApi.testTelegram({ ...form });
      ElMessage.success("测试消息已发送");
    } catch (error: unknown) {
      ElMessage.error(getErrorMessage(error, "测试失败"));
    } finally {
      testing.value = false;
    }
  };

  const refreshLatency = async () => {
    try {
      const res = await telegramApi.getLatency();
      latency.value = res.data || 0;
    } catch {
      latency.value = 0;
    }
  };

  return {
    config,
    form,
    handleSave,
    handleTest,
    latency,
    loadConfig,
    refreshLatency,
    saving,
    showSettings,
    testing,
  };
}
