<template>
  <component
    :is="appStore.isMobile ? 'el-drawer' : 'el-dialog'"
    v-model="visible"
    title="中转站上传"
    :size="appStore.isMobile ? 'auto' : '500px'"
    :width="appStore.isMobile ? '90%' : '500px'"
    :align-center="!appStore.isMobile"
    direction="btt"
    append-to-body
    class="task-wizard-dialog"
  >
    <div class="p-4 flex flex-col h-full">
      <div v-if="step === 0" class="flex flex-col gap-4">
        <div
          class="bg-base p-3 rounded text-sm text-secondary border border-base"
        >
          上传到公共中转站生成临时下载链接。
        </div>
        <el-form :model="form" label-position="top">
          <el-form-item label="有效期 (小时)">
            <el-input-number
              v-model="form.expire_hours"
              :min="1"
              :max="720"
              class="!w-full"
              controls-position="right"
            />
          </el-form-item>
          <el-form-item label="提取密码">
            <el-input v-model="form.password" placeholder="留空无密码" />
          </el-form-item>
        </el-form>
        <div class="flex justify-end pt-4">
          <el-button @click="visible = false">取消</el-button>
          <el-button type="primary" @click="handleStart" :loading="loading"
            >开始上传</el-button
          >
        </div>
      </div>
      <div v-else class="flex flex-col items-center justify-center py-8">
        <el-progress
          type="circle"
          :percentage="status?.progress || 0"
          :status="
            status?.state === 'error'
              ? 'exception'
              : status?.state === 'success'
                ? 'success'
                : ''
          "
        />
        <p class="mt-4 text-muted">{{ status?.message || "处理中..." }}</p>
        <div
          v-if="status?.state === 'success' && status?.download_url"
          class="mt-6 w-full"
        >
          <el-input v-model="status.download_url" readonly>
            <template #append>
              <el-button @click="copyText(status.download_url)">复制</el-button>
            </template>
          </el-input>
        </div>
      </div>
    </div>
  </component>
</template>

<script setup lang="ts">
import { ref, reactive, watch, onUnmounted } from "vue";
import { ElMessage } from "element-plus";
import * as shareApi from "../../../../api/share";
import { useAppStore } from "../../../../stores/app";
import useClipboard from "vue-clipboard3";
import type { TransferStatus } from "@/types";

const props = withDefaults(
  defineProps<{
    modelValue?: boolean;
    shareId?: string;
  }>(),
  {
    modelValue: false,
    shareId: undefined,
  },
);

const emit = defineEmits<{
  (event: "update:modelValue", visible: boolean): void;
}>();
const appStore = useAppStore();
const { toClipboard } = useClipboard();

const visible = ref(false);
const loading = ref(false);
const step = ref(0);
const status = ref<TransferStatus | null>(null);
const form = reactive({
  expire_hours: 24,
  password: "",
  burn_after_reading: false,
});
let pollTimer: ReturnType<typeof setInterval> | null = null;

const clearPollTimer = () => {
  if (!pollTimer) return;
  clearInterval(pollTimer);
  pollTimer = null;
};

watch(
  () => props.modelValue,
  (val: boolean) => {
    visible.value = val;
    if (val) {
      step.value = 0;
      status.value = null;
    } else {
      clearPollTimer();
    }
  },
);

watch(visible, (val: boolean) => emit("update:modelValue", val));

const handleStart = async () => {
  if (!props.shareId) return;
  loading.value = true;
  try {
    await shareApi.uploadToTransferStation(props.shareId, {
      ...form,
      password: form.password || null,
    });
    step.value = 1;
    startPolling();
  } catch (error) {
    ElMessage.error(error instanceof Error ? error.message : "上传失败");
  } finally {
    loading.value = false;
  }
};

const startPolling = () => {
  clearPollTimer();
  pollTimer = setInterval(async () => {
    if (!props.shareId) return;
    try {
      const res = await shareApi.getTransferStatus(props.shareId);
      status.value = res.data;
      if (res.data.state === "success" || res.data.state === "error") {
        clearPollTimer();
      }
    } catch {
      clearPollTimer();
    }
  }, 1000);
};

const copyText = (txt: string) => {
  toClipboard(txt)
    .then(() => ElMessage.success("已复制"))
    .catch(() => ElMessage.error("复制失败"));
};

onUnmounted(() => {
  clearPollTimer();
});
</script>
