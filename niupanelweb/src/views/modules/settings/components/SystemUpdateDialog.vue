<template>
  <el-dialog
    v-model="visibleValue"
    :title="title"
    :width="width"
    append-to-body
    align-center
    :show-close="!executingUpdate"
    :close-on-click-modal="!executingUpdate"
    class="release-notes-dialog custom-dialog"
  >
    <div class="p-1">
      <template v-if="!executingUpdate && !updateFailed">
        <div
          class="mb-5 flex items-center justify-between rounded-md border p-4"
          :class="
            updateInfo?.update_available
              ? 'bg-emerald-50 dark:bg-emerald-900/20 border-emerald-100 dark:border-emerald-800'
              : 'bg-gray-50 dark:bg-dark-800 border-base'
          "
        >
          <div class="flex flex-col">
            <span
              class="text-[11px] font-bold"
              :class="
                updateInfo?.update_available
                  ? 'text-emerald-600 dark:text-emerald-400'
                  : 'text-muted'
              "
            >
              {{ updateInfo?.update_available ? "发现新版本" : "当前已是最新版本" }}
            </span>
            <div class="flex items-baseline gap-2">
              <span
                class="text-[20px] font-bold"
                :class="
                  updateInfo?.update_available
                    ? 'text-emerald-800 dark:text-emerald-200'
                    : 'text-default'
                "
              >
                {{ updateInfo?.tag_name }}
              </span>
              <span
                class="rounded px-2 py-0.5 text-[10px] font-bold"
                :class="
                  updateInfo?.channel === 'preview'
                    ? 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-300'
                    : 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-300'
                "
              >
                {{ updateInfo?.channel === "preview" ? "预览" : "正式" }}
              </span>
              <span
                v-if="updateInfo?.size"
                class="text-xs font-bold opacity-60"
                :class="
                  updateInfo?.update_available
                    ? 'text-emerald-700 dark:text-emerald-300'
                    : 'text-muted'
                "
              >
                ({{ formatFileSize(updateInfo.size) }})
              </span>
            </div>
          </div>
          <div
            class="h-10 w-10 rounded-md text-[20px] flex-center"
            :class="
              updateInfo?.update_available
                ? 'bg-emerald-500/20 text-emerald-500'
                : 'bg-base text-muted'
            "
          >
            <div
              :class="
                updateInfo?.update_available
                  ? 'i-carbon-rocket'
                  : 'i-ep-circle-check'
              "
            ></div>
          </div>
        </div>

        <div class="mb-5">
          <h4
            class="mb-2 text-[11px] font-bold text-muted"
          >
            Core 更新日志
          </h4>
          <div
            class="max-h-[300px] overflow-y-auto rounded-md border border-light bg-subtle p-3 custom-scrollbar"
          >
            <div
              class="whitespace-pre-wrap text-[12px] font-medium leading-5 text-default"
            >
              {{ updateInfo?.body || "该版本没有提供详细说明。" }}
            </div>
          </div>
        </div>

        <div class="flex flex-col gap-3">
          <el-button
            v-if="updateInfo?.update_available"
            type="primary"
            size="large"
            class="!h-10 !w-full !rounded-md font-bold"
            @click="emit('start-update')"
          >
            立即更新 Core
          </el-button>
          <div v-else class="flex flex-col gap-3">
            <div class="text-center py-1 text-xs text-muted font-bold opacity-60">
              您当前使用的是最新{{ updateInfo?.channel === "preview" ? "预览" : "正式" }}版本
            </div>
            <el-button
              type="warning"
              plain
              size="large"
              class="!h-10 !w-full !rounded-md font-bold"
              @click="emit('force-update')"
            >
              <div class="i-ep-refresh mr-2"></div>
              重新安装 Core
            </el-button>
          </div>
          <el-button
            link
            class="text-muted text-xs"
            @click="visibleValue = false"
          >
            关闭窗口
          </el-button>
        </div>
      </template>

      <template v-else-if="executingUpdate">
        <div class="py-6 px-4">
          <el-progress
            :percentage="updateProgress"
            :status="updateProgress === 100 ? 'success' : ''"
            striped
            striped-flow
            :stroke-width="12"
          />
          <p
            class="mt-6 text-center text-[11px] font-bold text-primary"
          >
            {{ updateStatusMessage }}
          </p>
          <div v-if="canCancel" class="flex justify-center mt-8">
            <el-button
              type="danger"
              plain
              :loading="cancellingUpdate"
              size="small"
              class="!rounded-md"
              @click="emit('cancel-update')"
            >
              取消下载
            </el-button>
          </div>
        </div>
      </template>

      <template v-else-if="updateFailed">
        <div class="text-center py-6">
          <div class="text-red-500 text-5xl mb-4 flex justify-center">
            <div class="i-ep-circle-close-filled" />
          </div>
          <p class="mb-2 text-default font-bold">更新失败</p>
          <p class="text-xs text-muted mb-6 px-4 break-all">
            {{ updateStatusMessage }}
          </p>
          <div class="flex justify-center gap-2">
            <el-button @click="visibleValue = false">关闭</el-button>
            <el-button type="primary" @click="emit('retry-update')">
              重试
            </el-button>
          </div>
        </div>
      </template>
    </div>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { UpdateInfo } from "@/types";
import { formatFileSize } from "@/utils/format";

const props = defineProps<{
  canCancel: boolean;
  cancellingUpdate: boolean;
  executingUpdate: boolean;
  title: string;
  updateFailed: boolean;
  updateInfo: UpdateInfo | null;
  updateProgress: number;
  updateStatusMessage: string;
  visible: boolean;
  width: string;
}>();

const emit = defineEmits<{
  (event: "cancel-update"): void;
  (event: "force-update"): void;
  (event: "retry-update"): void;
  (event: "start-update"): void;
  (event: "update:visible", visible: boolean): void;
}>();

const visibleValue = computed({
  get: () => props.visible,
  set: (visible: boolean) => emit("update:visible", visible),
});
</script>

<style scoped>
.release-notes-dialog :deep(.el-dialog__body) {
  padding-top: 0 !important;
}
</style>
