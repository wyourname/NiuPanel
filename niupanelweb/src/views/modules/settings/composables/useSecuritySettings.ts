import { onMounted, reactive, ref } from "vue";
import { useRouter } from "vue-router";
import {
  ElMessage,
  ElMessageBox,
  type FormInstance,
  type FormItemRule,
} from "element-plus";
import * as settingsApi from "@/api/settings";
import * as telegramApi from "@/api/telegram";
import * as userApi from "@/api/user";
import { useUserStore } from "@/stores/user";
import type { SessionInfo } from "@/types";

const createTelegramConfig = (): telegramApi.TelegramBotConfig => ({
  enabled: false,
  token: "",
  admin_chat_id: "",
  events: [],
  cf_proxy_enabled: false,
  cf_host: "",
  cf_ip: "",
  cf_token: "",
  login_2fa: false,
});

const validateForm = async (form: FormInstance) => {
  try {
    return await form.validate();
  } catch {
    return false;
  }
};

export function useSecuritySettings() {
  const router = useRouter();
  const userStore = useUserStore();

  const profileFormRef = ref<FormInstance | null>(null);
  const savingProfile = ref(false);
  const profileForm = reactive({
    username: "",
    password_confirm: "",
  });

  const profileRules = {
    username: [
      { required: true, message: "请输入用户名", trigger: "blur" },
      { min: 3, max: 20, message: "用户名长度至少 3 个字符", trigger: "blur" },
    ],
    password_confirm: [
      { required: true, message: "请输入当前密码以验证身份", trigger: "blur" },
    ],
  };

  const passFormRef = ref<FormInstance | null>(null);
  const savingPass = ref(false);
  const passForm = reactive({
    old_password: "",
    new_password: "",
    confirm_password: "",
  });

  const validatePasswordConfirm: FormItemRule["validator"] = (
    _rule,
    value: unknown,
    callback,
  ) => {
    if (value !== passForm.new_password) {
      callback(new Error("两次输入密码不一致"));
      return;
    }
    callback();
  };

  const passRules = {
    old_password: [
      { required: true, message: "请输入当前密码", trigger: "blur" },
    ],
    new_password: [
      { required: true, message: "请输入新密码", trigger: "blur" },
      { min: 8, message: "长度至少 8 位", trigger: "blur" },
    ],
    confirm_password: [
      {
        validator: validatePasswordConfirm,
        trigger: "blur",
      },
    ],
  };

  const maxSessions = ref(2);
  const savingSecurity = ref(false);
  const sessions = ref<SessionInfo[]>([]);
  const loadingSessions = ref(false);

  const tgConfig = ref<telegramApi.TelegramBotConfig>(createTelegramConfig());
  const savingTg2FA = ref(false);

  const loadSessions = async () => {
    loadingSessions.value = true;
    try {
      const res = await settingsApi.getActiveSessions();
      sessions.value = res.data;
    } finally {
      loadingSessions.value = false;
    }
  };

  const loadData = async () => {
    const user = await userStore.fetchUserProfile();
    if (user) profileForm.username = user.username || "";

    void loadSessions();
    if (user?.role !== "admin") return;

    const res = await settingsApi.getSettings();
    const setting = res.data.find((item) => item.key === "auth.max_sessions");
    if (setting) maxSessions.value = Number(setting.value);

    try {
      const tgRes = await telegramApi.getTelegramConfig();
      tgConfig.value = tgRes.data;
    } catch {
    }
  };

  const handleUpdateProfile = async () => {
    if (!profileFormRef.value) return;

    const valid = await validateForm(profileFormRef.value);
    if (!valid) return;

    savingProfile.value = true;
    try {
      await userApi.updateProfile({
        username: profileForm.username,
        password_confirm: profileForm.password_confirm,
      });
      ElMessage.success("个人资料已更新");
      profileForm.password_confirm = "";
      await userStore.fetchUserProfile();
    } finally {
      savingProfile.value = false;
    }
  };

  const handleChangePassword = async () => {
    if (!passFormRef.value) return;

    const valid = await validateForm(passFormRef.value);
    if (!valid) return;

    savingPass.value = true;
    try {
      await userApi.changePassword({
        old_password: passForm.old_password,
        new_password: passForm.new_password,
      });
      ElMessage.success("密码修改成功，请重新登录");
      userStore.logout();
      await router.push("/login");
    } finally {
      savingPass.value = false;
    }
  };

  const handleLogout = async () => {
    try {
      await ElMessageBox.confirm("确定要退出当前账号吗？再次确认后将返回登录页。", "退出登录", {
        type: "warning",
        confirmButtonText: "退出登录",
        cancelButtonText: "取消",
        roundButton: true,
      });
      await userStore.logout();
      await router.push("/login");
      ElMessage.success("已退出登录");
    } catch {
    }
  };

  const handleSaveMaxSessions = async () => {
    savingSecurity.value = true;
    try {
      await settingsApi.updateSecuritySettings({
        max_sessions: maxSessions.value,
      });
      ElMessage.success("系统设置已保存");
    } finally {
      savingSecurity.value = false;
    }
  };

  const handleSaveTg2FA = async () => {
    savingTg2FA.value = true;
    try {
      await telegramApi.updateTelegramConfig(tgConfig.value);
      ElMessage.success(
        `Telegram 2FA 已${tgConfig.value.login_2fa ? "启用" : "禁用"}`,
      );
    } catch {
      tgConfig.value.login_2fa = !tgConfig.value.login_2fa;
    } finally {
      savingTg2FA.value = false;
    }
  };

  const handleRevoke = async (id: string) => {
    try {
      await ElMessageBox.confirm("强制下线该设备？", "提示", { type: "warning" });
      await settingsApi.revokeSession(id);
      ElMessage.success("已下线");
      await loadSessions();
    } catch {
    }
  };

  onMounted(() => {
    void loadData();
  });

  return {
    handleChangePassword,
    handleLogout,
    handleRevoke,
    handleSaveMaxSessions,
    handleSaveTg2FA,
    handleUpdateProfile,
    loadSessions,
    loadingSessions,
    maxSessions,
    passForm,
    passFormRef,
    passRules,
    profileForm,
    profileFormRef,
    profileRules,
    savingPass,
    savingProfile,
    savingSecurity,
    savingTg2FA,
    sessions,
    tgConfig,
    userStore,
  };
}
