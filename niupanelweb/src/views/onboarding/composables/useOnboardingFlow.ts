import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { ElMessage } from "element-plus";
import * as envApi from "@/api/environment";
import request from "@/utils/request";

export type OnboardingFeature = {
  color: string;
  desc: string;
  icon: string;
  title: string;
};

export const ONBOARDING_STEPS = [0, 1, 2, 3] as const;
const FALLBACK_NODE_LTS_VERSION = "22.23.1";

export const ONBOARDING_FEATURES: OnboardingFeature[] = [
  {
    icon: "i-ep-cpu",
    color: "text-primary",
    title: "Python 虚拟环境",
    desc: "完全隔离，不污染全局",
  },
  {
    icon: "i-ep-monitor",
    color: "text-green-500",
    title: "Node.js 沙盒",
    desc: "pnpm runtime 驱动，多版本并存",
  },
  {
    icon: "i-ep-lock",
    color: "text-orange-500",
    title: "依赖隔离",
    desc: "每个环境独立 node_modules",
  },
  {
    icon: "i-ep-finished",
    color: "text-purple-500",
    title: "开箱即用",
    desc: "无需手动配置环境变量",
  },
];

const getApiErrorMessage = (error: unknown, fallback: string) => {
  if (typeof error !== "object" || error === null || !("response" in error)) {
    return fallback;
  }
  return (
    error as { response?: { data?: { message?: string } } }
  ).response?.data?.message || fallback;
};

const markOnboardingDone = async () => {
  try {
    await request.post("/settings/onboarding/done");
  } catch {
  }
};

export function useOnboardingFlow() {
  const router = useRouter();

  const step = ref(0);
  const checkingEnv = ref(false);
  const hasPython = ref(false);
  const hasNode = ref(false);

  const pythonVersion = ref("3.11");
  const nodeVersion = ref(FALLBACK_NODE_LTS_VERSION);
  const recommendedNodeVersion = ref(FALLBACK_NODE_LTS_VERSION);

  const pythonJobId = ref<string | null>(null);
  const pythonLoading = ref(false);
  const pythonDone = ref(false);
  const pythonStatus = ref("");

  const nodeJobId = ref<string | null>(null);
  const nodeLoading = ref(false);
  const nodeDone = ref(false);
  const nodeStatus = ref("");

  const checkExistingEnvironments = async () => {
    checkingEnv.value = true;
    try {
      const res = await envApi.getEnvironments();
      const envs = res.data || [];
      hasPython.value = envs.some((env) => env.env_type === "python");
      hasNode.value = envs.some((env) => env.env_type === "node");
      if (hasPython.value) pythonDone.value = true;
      if (hasNode.value) nodeDone.value = true;
    } finally {
      checkingEnv.value = false;
    }
  };

  const loadRecommendedNodeVersion = async () => {
    try {
      const res = await envApi.getAvailableVersions();
      const payload = JSON.parse(res.data || "{}") as {
        node_recommended_lts?: unknown;
      };
      if (typeof payload.node_recommended_lts !== "string") return;

      const recommended = payload.node_recommended_lts.trim().replace(/^v/, "");
      if (!/^\d+\.\d+\.\d+$/.test(recommended)) return;

      recommendedNodeVersion.value = recommended;
      if (nodeVersion.value === FALLBACK_NODE_LTS_VERSION) {
        nodeVersion.value = recommended;
      }
    } catch {
      // 镜像索引不可用时保留兼容 ARMv7 的 Node 22 LTS 兜底版本。
    }
  };

  const startInitialization = () => {
    if (hasPython.value && hasNode.value) {
      step.value = 3;
      return;
    }
    if (hasPython.value) {
      step.value = 2;
      return;
    }
    step.value = 1;
  };

  const goNextFromPython = () => {
    if (hasNode.value) {
      step.value = 3;
      return;
    }
    step.value = 2;
  };

  const goBackFromNode = () => {
    if (hasPython.value) {
      step.value = 0;
      return;
    }
    step.value = 1;
  };

  const createPythonEnv = async () => {
    if (!pythonVersion.value.trim()) return;

    pythonLoading.value = true;
    pythonStatus.value = `正在创建 Python ${pythonVersion.value} 虚拟环境...`;
    try {
      const res = await envApi.createEnvironment(
        { version: pythonVersion.value },
        "python",
      );
      pythonJobId.value = res.data;
      pythonDone.value = true;
      pythonStatus.value = `任务 #${pythonJobId.value} 已提交，正在后台安装...`;
    } catch (error: unknown) {
      ElMessage.error(getApiErrorMessage(error, "创建失败，请稍后重试"));
      pythonStatus.value = "创建失败";
    } finally {
      pythonLoading.value = false;
    }
  };

  const createNodeEnv = async () => {
    if (!nodeVersion.value.trim()) return;

    nodeLoading.value = true;
    nodeStatus.value = `正在创建 Node.js ${nodeVersion.value} 环境...`;
    try {
      const res = await envApi.createEnvironment(
        { version: nodeVersion.value },
        "node",
      );
      nodeJobId.value = res.data;
      nodeDone.value = true;
      nodeStatus.value = `任务 #${nodeJobId.value} 已提交，正在后台安装...`;
    } catch (error: unknown) {
      ElMessage.error(getApiErrorMessage(error, "创建失败，请稍后重试"));
      nodeStatus.value = "创建失败";
    } finally {
      nodeLoading.value = false;
    }
  };

  const handleFinish = async () => {
    step.value = 3;
    await markOnboardingDone();
  };

  const goToDashboard = () => {
    void router.push({ name: "tasks" });
  };

  const skipOnboarding = async () => {
    await markOnboardingDone();
    void router.push({ name: "tasks" });
  };

  onMounted(() => {
    void checkExistingEnvironments();
    void loadRecommendedNodeVersion();
  });

  return {
    checkingEnv,
    createNodeEnv,
    createPythonEnv,
    features: ONBOARDING_FEATURES,
    goBackFromNode,
    goNextFromPython,
    goToDashboard,
    handleFinish,
    hasNode,
    hasPython,
    nodeDone,
    nodeJobId,
    nodeLoading,
    nodeStatus,
    nodeVersion,
    recommendedNodeVersion,
    pythonDone,
    pythonJobId,
    pythonLoading,
    pythonStatus,
    pythonVersion,
    skipOnboarding,
    startInitialization,
    step,
    steps: ONBOARDING_STEPS,
  };
}
