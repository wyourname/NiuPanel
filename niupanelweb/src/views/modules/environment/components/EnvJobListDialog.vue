<template>
  <component
    :is="appStore.isMobile ? 'el-drawer' : 'el-dialog'"
    v-model="visible"
    :title="appStore.isMobile ? '环境任务' : ''"
    :size="appStore.isMobile ? '100%' : '600px'"
    :width="appStore.isMobile ? '100%' : '600px'"
    :align-center="!appStore.isMobile"
    direction="btt"
    destroy-on-close
    append-to-body
    class="log-modal"
  >
    <div class="flex-1 flex flex-col bg-[var(--editor-bg)] overflow-hidden">
      <!-- Internal Header for PC/Modern Style -->
      <div
        class="flex items-center justify-between px-4 shrink-0 border-b border-[var(--editor-border)] h-12"
      >
        <div class="flex items-center gap-4 overflow-hidden">
          <button
            v-if="!appStore.isMobile"
            type="button"
            class="h-8 w-8 rounded-md text-muted flex-center transition-colors hover:bg-black/5 hover:text-default dark:hover:bg-white/5"
            title="关闭"
            aria-label="关闭环境任务对话框"
            @click="visible = false"
          >
            <div class="i-ep-close text-lg"></div>
          </button>
          <span class="text-xs font-bold text-[var(--editor-text)] truncate">
            {{ appStore.isMobile ? "" : "环境任务" }}
          </span>
        </div>
        <div class="flex items-center gap-2">
          <button
            type="button"
            class="h-8 w-8 rounded-md text-muted flex-center transition-colors hover:bg-black/5 hover:text-default dark:hover:bg-white/5"
            title="刷新任务"
            aria-label="刷新环境任务"
            @click="fetchJobs"
          >
            <div :class="['i-ep-refresh', loading ? 'animate-spin' : '']"></div>
          </button>
        </div>
      </div>

      <!-- Table Content with consistent padding -->
      <div class="flex-1 overflow-hidden px-4">
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
                {{ row.status }}
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
                v-if="row.status === 'Running'"
                size="small"
                link
                type="danger"
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
  </component>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import { ElMessage } from "element-plus";
import * as jobApi from "../../../../api/jobs";
import { useAppStore } from "../../../../stores/app";
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

watch(
  () => props.modelValue,
  (val: boolean) => {
    visible.value = val;
    if (val) fetchJobs();
  },
);

watch(visible, (val: boolean) => emit("update:modelValue", val));

const fetchJobs = async () => {
  loading.value = true;
  try {
    const res = await jobApi.getJobs();
    jobs.value = res.data || [];
  } catch (e) {
  } finally {
    loading.value = false;
  }
};

const handleCancelJob = async (job: Job) => {
  try {
    await jobApi.cancelJob(job.id);
    ElMessage.success("已发送取消指令");
    fetchJobs();
  } catch (e) {}
};

const getStatusType = (status: string): TagType => {
  return status === "Running"
    ? undefined
    : status === "Failed" || status === "Cancelled"
      ? "danger"
      : "success";
};
</script>
