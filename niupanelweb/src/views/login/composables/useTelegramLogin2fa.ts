import { ref } from "vue";
import { useRouter } from "vue-router";
import { ElMessage } from "element-plus";
import { verifyLogin2FA } from "@/api/auth";
import { useUserStore } from "@/stores/user";
import { getApiErrorMessage } from "./authError";

export function useTelegramLogin2fa() {
  const router = useRouter();
  const userStore = useUserStore();

  const show2faDialog = ref(false);
  const loginTicket = ref("");
  const verifyCode = ref("");
  const verifying2fa = ref(false);

  const openTwoFactorDialog = (ticket: string) => {
    loginTicket.value = ticket;
    verifyCode.value = "";
    show2faDialog.value = true;
  };

  const handleVerify2FA = async () => {
    if (verifyCode.value.length !== 6) {
      ElMessage.warning("请输入6位数字验证码");
      return;
    }

    verifying2fa.value = true;
    try {
      const res = await verifyLogin2FA({
        ticket: loginTicket.value,
        code: verifyCode.value,
      });

      userStore.setUserInfo(res.data);
      show2faDialog.value = false;
      ElMessage.success("登录成功");
      router.push({ name: "tasks" });
    } catch (error: unknown) {
      ElMessage.error(getApiErrorMessage(error, "验证码错误或已过期"));
    } finally {
      verifying2fa.value = false;
    }
  };

  return {
    handleVerify2FA,
    openTwoFactorDialog,
    show2faDialog,
    verifyCode,
    verifying2fa,
  };
}
