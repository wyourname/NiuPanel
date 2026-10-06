<template>
  <ResponsiveDialog
    v-model:visible="visible"
    title="环境任务"
    desktop-size="lg"
    content-preset="list"
    mobile-mode="fullscreen"
    destroy-on-close
    append-to-body
  >
    <template #header-actions>
      <button
        type="button"
        class="mobile-touch-target h-9 w-9 cursor-pointer rounded-md text-secondary flex-center transition-colors hover:bg-soft hover:text-default"
        title="刷新任务"
        aria-label="刷新环境任务"
        @click="fetchJobs"
      >
        <span :class="['i-ep-refresh', loading ? 'animate-spin' : '']"></span>
      </button>
    </template>

    <div class="flex min-h-[420px] flex-1 flex-col overflow-hidden md:h-[min(600px,70vh)]">
      <div class="min-h-0 flex-1 overflow-hidden">
        <el-table
          :data="jobs"
          v-loading="loading"
          height="100%"
          style="width: 100%; --el-table-bg-color: transparent; --el-table-tr-bg-color: transparent;"
          stripe
          class="modern-table"
        >
          <el-table-column
            prop="status"
            label="状态"
            :width="appStore.isMobile ? 70 : 90"
          >
            <template #default="{ row }">
              <el-tag :type="getStatusType(row.status)" size="small" effect="plain" class="!border-none !bg-opacity-10">
                {{ statusLabel(row.status) }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column
            prop="name"
            label="任务名称"
            :min-width="appStore.isMobile ? 150 : 200"
            show-overflow-tooltip
          >
            <template #default="{ row }">
              <span class="text-xs font-medium text-[var(--editor-text)] opacity-90">{{ row.name }}</span>
            </template>
          </el-table-column>
          <el-table-column
            label="操作"
            :width="appStore.isMobile ? 80 : 100"
            align="right"
          >
            <template #default="{ row }">
              <el-button
                v-if="row.status === 'Running' || row.status === 'Pending'"
                size="small"
                link
                type="danger"
                :disabled="cancelling === row.id"
                @click="handleCancelJob(row)"
                >取消</el-button
              >
              <el-button
                size="small"
                link
                type="primary"
                @click="emit('show-log', row.id, row.name)"
                >日志</el-button
              >
            </template>
          </el-table-column>
        </el-table>
      </div>
    </div>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { useMobileBackCloseAction } from "@/composables/useMobileBackCloseAction";
import { ref, watch, onUnmounted } from "vue";
import { ElMessage } from "element-plus";
import * as jobApi from "../../../../api/jobs";
import { useAppStore } from "../../../../stores/app";
import ResponsiveDialog from "../../../../components/common/ResponsiveDialog.vue";
import type { Job } from "@/types";

type TagType = "success" | "danger" | undefined;

const props = withDefaults(
  defineProps<{
    modelValue?: boolean;
  }>(),
  {
    modelValue: false,
  },
);

const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
  (event: "show-log", id: number | string, name: string): void;
}>();
const appStore = useAppStore();

const visible = ref(false);
const loading = ref(false);
const jobs = ref<Job[]>([]);
const cancelling = ref<number | null>(null);
let timer: ReturnType<typeof setTimeout> | undefined;
let disposed = false;
const statusLabel = (status: string) => ({ Pending: "排队中", Running: "执行中", Success: "成功", Finished: "完成", Failed: "失败", Cancelled: "已取消" } as Record<string, string>)[status] || status;


watch(
  () => props.modelValue,
  (val: boolean) => {
    visible.value = val;
    if (val) void fetchJobs();
    else clearTimeout(timer);
  },
);

watch(visible, (val: boolean) => emit("update:modelValue", val));

const fetchJobs = async () => {
  if (loading.value) return;
  clearTimeout(timer);
  loading.value = true;
  try {
    const res = await jobApi.getJobs();
    jobs.value = res.data || [];
  } catch (e) {
  } finally {
    loading.value = false;
    if (!disposed && visible.value) timer = setTimeout(fetchJobs, 3000);
  }
};

const handleCancelJob = async (job: Job) => {
  if (cancelling.value !== null) return;
  cancelling.value = job.id;
  try {
    await jobApi.cancelJob(job.id);
    ElMessage.success("已发送取消指令");
    fetchJobs();
  } catch (e) {} finally { cancelling.value = null; }
};

onUnmounted(() => { disposed = true; clearTimeout(timer); });

const getStatusType = (status: string): TagType => {
  return status === "Running"
    ? undefined
    : status === "Failed" || status === "Cancelled"
      ? "danger"
      : "success";
};
useMobileBackCloseAction({ appStore: useAppStore(), visible, close: () => { visible.value = false; } });
</script>
