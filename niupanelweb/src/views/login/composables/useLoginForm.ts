import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { ElMessage, type FormInstance, type FormRules } from "element-plus";
import { login as apiLogin, register } from "@/api/auth";
import { useUserStore } from "@/stores/user";
import type { LoginResponse, UserInfo } from "@/types";
import { getApiErrorMessage, getApiErrorStatus } from "./authError";

export type LoginForm = {
  username: string;
  password: string;
  confirm_password: string;
  mail_host: string;
  mail_username: string;
  mail_password: string;
};

type ValidatorCallback = (error?: Error) => void;

type UseLoginFormOptions = {
  onTwoFactorRequired: (ticket: string) => void;
};

const createInitialForm = (): LoginForm => ({
  username: "",
  password: "",
  confirm_password: "",
  mail_host: "",
  mail_username: "",
  mail_password: "",
});

const isTwoFactorLoginResponse = (data: LoginResponse): data is { ticket: string } => {
  return "ticket" in data;
};

const isUserInfoResponse = (data: LoginResponse): data is UserInfo => {
  return !isTwoFactorLoginResponse(data);
};

export function useLoginForm({ onTwoFactorRequired }: UseLoginFormOptions) {
  const router = useRouter();
  const userStore = useUserStore();

  const formRef = ref<FormInstance | null>(null);
  const loading = ref(false);
  const form = ref<LoginForm>(createInitialForm());
  const showSmtpConfig = ref(false);
  const isInitialized = computed(() => userStore.isInitialized);

  const rules: FormRules<LoginForm> = {
    username: [
      { required: true, message: "请输入用户名", trigger: "blur" },
      { min: 3, max: 20, message: "用户名长度至少3个字符", trigger: "blur" },
    ],
    password: [
      { required: true, message: "请输入密码", trigger: "blur" },
      { min: 6, message: "密码长度至少 6 个字符", trigger: "blur" },
    ],
    confirm_password: [
      {
        validator: (_rule: unknown, value: string, callback: ValidatorCallback) => {
          if (!isInitialized.value && value !== form.value.password) {
            callback(new Error("两次输入密码不一致"));
          } else {
            callback();
          }
        },
        trigger: "blur",
      },
    ],
    mail_username: [
      {
        validator: (_rule: unknown, value: string, callback: ValidatorCallback) => {
          if (showSmtpConfig.value && value) {
            if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value)) {
              callback(new Error("邮箱格式不正确"));
            } else {
              callback();
            }
          } else {
            callback();
          }
        },
        trigger: "blur",
      },
    ],
  };

  const checkStatus = async () => {
    try {
      await userStore.checkSystemStatus();
    } catch {
      // Keep the login page reachable when status probing fails.
    }
  };

  const resetForm = () => {
    form.value = createInitialForm();
  };

  const handleSubmit = async (formInstance?: FormInstance | null) => {
    const targetForm = formInstance ?? formRef.value;
    if (!targetForm) return;

    await targetForm.validate(async (valid: boolean) => {
      if (!valid) return;

      loading.value = true;
      try {
        if (isInitialized.value) {
          const res = await apiLogin({
            username: form.value.username,
            password: form.value.password,
          });

          if (isTwoFactorLoginResponse(res.data)) {
            onTwoFactorRequired(res.data.ticket);
          } else if (isUserInfoResponse(res.data)) {
            userStore.setUserInfo(res.data);
            ElMessage.success("登录成功");
            router.push({ name: "tasks" });
          }
        } else {
          await register({
            ...form.value,
            email: form.value.mail_username,
          });
          ElMessage.success("系统初始化成功，请登录");
          await checkStatus();
          resetForm();
        }
      } catch (error: unknown) {
        if (getApiErrorStatus(error) === 401) {
          ElMessage.error(getApiErrorMessage(error, "登录失败"));
        } else {
          ElMessage.error(getApiErrorMessage(error, "连接失败，请检查网络或服务器地址配置"));
        }
      } finally {
        loading.value = false;
      }
    });
  };

  onMounted(() => {
    void checkStatus();
  });

  return {
    checkStatus,
    form,
    formRef,
    handleSubmit,
    isInitialized,
    loading,
    rules,
    showSmtpConfig,
  };
}
