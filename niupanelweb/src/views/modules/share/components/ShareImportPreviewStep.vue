<template>
  <div class="flex flex-col gap-4">
    <div
      class="flex items-center justify-between rounded-md border border-base bg-base/50 p-3"
    >
      <div class="flex flex-col min-w-0">
        <span class="text-xs text-muted mb-0.5">资源名称</span>
        <span class="font-bold text-default truncate">
          {{ packageInfo.note || "未命名分享" }}
        </span>
      </div>
      <div class="flex items-center gap-3 shrink-0 ml-4">
        <div class="text-right">
          <div class="text-[10px] text-muted leading-tight">版本</div>
          <div class="font-mono text-xs font-bold">
            v{{ packageInfo.version }}
          </div>
        </div>
        <div class="w-px h-6 bg-base"></div>
        <div class="text-right">
          <div class="text-[10px] text-muted leading-tight">任务</div>
          <div class="font-bold text-xs">
            {{ packageInfo.tasks.length }}
          </div>
        </div>
      </div>
    </div>

    <div class="flex flex-col gap-2">
      <div class="flex justify-between items-center">
        <span class="text-xs font-bold text-default">
          待导入内容 ({{ selectedTasks.length }})
        </span>
        <el-button link type="primary" size="small" @click="emit('toggle-all')">
          {{
            selectedTasks.length === packageInfo.tasks.length
              ? "取消全选"
              : "全选全部"
          }}
        </el-button>
      </div>

      <div class="max-h-[200px] overflow-y-auto rounded-md border border-base bg-card">
        <div
          v-for="task in packageInfo.tasks"
          :key="task.meta.name"
          class="flex items-center p-3 border-b border-base last:border-0 hover:bg-hover cursor-pointer transition-colors group"
          @click="emit('toggle-task', task.meta.name)"
        >
          <el-checkbox
            :model-value="selectedTasks.includes(task.meta.name)"
            class="!mr-3"
            @click.stop="emit('toggle-task', task.meta.name)"
          />
          <div class="flex-1 min-w-0 mr-4">
            <div class="flex items-center gap-2">
              <span class="font-bold text-xs text-default truncate">
                {{ task.meta.name }}
              </span>
              <el-tooltip
                v-if="task.meta.variables"
                content="包含变量配置"
                placement="top"
              >
                <div class="i-ep-key text-amber-500 text-[10px]"></div>
              </el-tooltip>
            </div>
            <div class="text-[10px] text-muted truncate mt-0.5">
              {{ task.meta.description || "无任务描述" }}
            </div>
          </div>
          <div class="text-[10px] font-mono text-muted whitespace-nowrap">
            {{ task.files.length }} 文件
          </div>
        </div>
      </div>
    </div>

    <div class="flex gap-3 mt-2">
      <el-button class="flex-1 !h-9 !rounded-md" @click="emit('reset')">
        上一步
      </el-button>
      <el-button
        type="primary"
        class="flex-[2] !h-9 !rounded-md font-bold"
        :loading="importing"
        :disabled="selectedTasks.length === 0"
        @click="emit('confirm')"
      >
        开始导入所选内容
      </el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { NiuPackage } from "@/types";

defineProps<{
  importing: boolean;
  packageInfo: NiuPackage;
  selectedTasks: string[];
}>();

const emit = defineEmits<{
  (event: "confirm"): void;
  (event: "reset"): void;
  (event: "toggle-all"): void;
  (event: "toggle-task", taskName: string): void;
}>();
</script>
