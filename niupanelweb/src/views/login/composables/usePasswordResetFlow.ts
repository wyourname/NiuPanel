import { computed, onMounted, onScopeDispose, ref, type Ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ElMessage, type FormInstance, type FormRules } from "element-plus";
import {
  forgotPassword,
  identifyReset,
  resetPassword,
  verifyResetCode,
} from "@/api/auth";

export type ResetForm = {
  password: string;
  confirm: string;
};

type ValidatorCallback = (error?: Error) => void;

type UsePasswordResetFlowOptions = {
  loading: Ref<boolean>;
};

export function usePasswordResetFlow({ loading }: UsePasswordResetFlowOptions) {
  const route = useRoute();
  const router = useRouter();

  const showForgotDialog = ref(false);
  const forgotUsername = ref("");
  const forgotEmailPrefix = ref("");
  const forgotEmailSuffix = ref("");
  const forgotEmail = computed(() => forgotEmailPrefix.value.trim());
  const forgotCode = ref("");
  const forgotStep = ref(0);
  const identifying = ref(false);
  const sendingEmail = ref(false);
  const verifyingCode = ref(false);
  const countdown = ref(0);
  const resetToken = ref<string | null>(null);
  const resetFormRef = ref<FormInstance | null>(null);
  const resetForm = ref<ResetForm>({
    password: "",
    confirm: "",
  });

  let countdownTimer: ReturnType<typeof setInterval> | null = null;

  const clearCountdown = () => {
    if (countdownTimer) clearInterval(countdownTimer);
    countdownTimer = null;
  };

  const startCountdown = () => {
    countdown.value = 60;
    clearCountdown();
    countdownTimer = setInterval(() => {
      if (countdown.value > 0) {
        countdown.value--;
      } else {
        clearCountdown();
      }
    }, 1000);
  };

  const resetForgotState = () => {
    showForgotDialog.value = false;
    forgotStep.value = 0;
    forgotUsername.value = "";
    forgotEmailPrefix.value = "";
    forgotEmailSuffix.value = "";
    forgotCode.value = "";
    clearCountdown();
    countdown.value = 0;
  };

  const getForgotTitle = () => {
    if (forgotStep.value === 0) return "第一步：查找账户";
    if (forgotStep.value === 1) return "第二步：核验邮箱";
    return "第三步：验证安全码";
  };

  const handleIdentify = async () => {
    if (!forgotUsername.value) return;
    identifying.value = true;
    try {
      const res = await identifyReset(forgotUsername.value);
      forgotEmailSuffix.value = res.data.suffix;
      forgotStep.value = 1;
    } catch {
    } finally {
      identifying.value = false;
    }
  };

  const handleForgotSubmit = async () => {
    if (!forgotEmailPrefix.value) return;
    sendingEmail.value = true;
    try {
      await forgotPassword(forgotUsername.value, forgotEmail.value);
      ElMessage.success("验证码已发送");
      forgotStep.value = 2;
      startCountdown();
    } catch {
    } finally {
      sendingEmail.value = false;
    }
  };

  const handleVerifyCode = async () => {
    if (forgotCode.value.length !== 6) return;
    verifyingCode.value = true;
    try {
      const res = await verifyResetCode(forgotEmail.value, forgotCode.value);
      ElMessage.success("核验通过");
      resetToken.value = res.data;
      showForgotDialog.value = false;
    } catch {
    } finally {
      verifyingCode.value = false;
    }
  };

  const resetRules: FormRules<ResetForm> = {
    password: [
      { required: true, message: "请输入新密码", trigger: "blur" },
      { min: 8, message: "长度至少为 8 位", trigger: "blur" },
    ],
    confirm: [
      {
        validator: (_rule: unknown, value: string, callback: ValidatorCallback) => {
          if (value !== resetForm.value.password) {
            callback(new Error("两次输入密码不一致"));
          } else {
            callback();
          }
        },
        trigger: "blur",
      },
    ],
  };

  const handleResetSubmit = async (formInstance?: FormInstance | null) => {
    const targetForm = formInstance ?? resetFormRef.value;
    if (!targetForm) return;
    await targetForm.validate(async (valid) => {
      if (!valid) return;
      if (!resetToken.value) {
        ElMessage.error("重置凭证已失效，请重新验证");
        return;
      }

      loading.value = true;
      try {
        await resetPassword({
          token: resetToken.value,
          new_password: resetForm.value.password,
        });
        ElMessage.success("密码更新成功，请登录");
        resetToken.value = null;
        router.replace("/login");
      } catch {
      } finally {
        loading.value = false;
      }
    });
  };

  onMounted(() => {
    if (typeof route.query.token === "string") {
      resetToken.value = route.query.token;
    }
  });

  onScopeDispose(clearCountdown);

  return {
    countdown,
    forgotCode,
    forgotEmailPrefix,
    forgotEmailSuffix,
    forgotStep,
    forgotUsername,
    getForgotTitle,
    handleForgotSubmit,
    handleIdentify,
    handleResetSubmit,
    handleVerifyCode,
    identifying,
    resetForgotState,
    resetForm,
    resetFormRef,
    resetRules,
    resetToken,
    sendingEmail,
    showForgotDialog,
    verifyingCode,
  };
}
