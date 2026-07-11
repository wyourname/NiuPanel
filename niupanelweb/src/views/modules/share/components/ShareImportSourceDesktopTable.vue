<template>
  <el-table
    :data="groups"
    class="w-full h-full"
    header-cell-class-name="!bg-base/30 !text-muted text-[11px] font-bold"
  >
    <el-table-column type="expand">
      <template #default="{ row }">
        <div class="px-14 py-3 bg-base/10 shadow-inner">
          <div class="max-h-[400px] overflow-y-auto custom-scrollbar pr-2">
            <el-table
              :data="row.tasks"
              size="small"
              :show-header="false"
              class="!bg-transparent"
            >
              <el-table-column prop="task_name">
                <template #default="{ row: task }">
                  <div class="flex items-center gap-3 py-1">
                    <div class="i-ep-document text-muted/40 text-sm"></div>
                    <span class="text-[13px] font-medium text-default/80">
                      {{ task.task_name }}
                    </span>
                  </div>
                </template>
              </el-table-column>
              <el-table-column prop="note" show-overflow-tooltip>
                <template #default="{ row: task }">
                  <span class="text-xs text-muted/60 italic">
                    {{ task.note || "无描述" }}
                  </span>
                </template>
              </el-table-column>
              <el-table-column width="120" align="right">
                <template #default="{ row: task }">
                  <el-button
                    type="danger"
                    link
                    size="small"
                    class="!text-danger/40 hover:!text-danger transition-colors"
                    @click="emit('delete', task.id, 'task')"
                  >
                    <div class="i-ep-delete text-sm"></div>
                  </el-button>
                </template>
              </el-table-column>
            </el-table>
          </div>
        </div>
      </template>
    </el-table-column>

    <el-table-column label="分享源" min-width="200">
      <template #default="{ row }">
        <div class="flex flex-col py-1">
          <div class="flex items-center gap-1.5 mb-0.5">
            <div class="i-ep-box text-primary shrink-0"></div>
            <span class="font-bold text-default text-sm">
              分享包: {{ row.share_code || "Legacy" }}
            </span>
            <el-tag
              size="small"
              type="info"
              effect="plain"
              class="scale-90 origin-left"
            >
              {{ row.task_count }} 任务
            </el-tag>
          </div>
          <div class="flex items-center gap-1.5 overflow-hidden">
            <div class="i-ep-link text-[10px] text-muted shrink-0"></div>
            <span class="text-[10px] text-muted truncate font-mono opacity-70">
              {{ row.url }}
            </span>
          </div>
        </div>
      </template>
    </el-table-column>

    <el-table-column label="最后更新" width="180">
      <template #default="{ row }">
        <span class="text-[11px] font-mono text-muted">
          {{ formatDate(row.last_updated_at * 1000) }}
        </span>
      </template>
    </el-table-column>

    <el-table-column label="操作" width="200" align="right">
      <template #default="{ row }">
        <div class="flex items-center justify-end gap-2">
          <el-button
            type="primary"
            link
            size="small"
            @click="emit('update', row.url)"
          >
            <div class="i-ep-refresh mr-1"></div>
            整体更新
          </el-button>
          <el-dropdown trigger="click">
            <el-button link size="small">
              <div class="i-ep-more-filled rotate-90"></div>
            </el-button>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item @click="emit('copy-url', row.url)">
                  <div class="i-ep-copy-document mr-2"></div>
                  复制链接
                </el-dropdown-item>
                <el-dropdown-item
                  divided
                  class="!text-danger"
                  @click="requestDeleteGroup(row)"
                >
                  <div class="i-ep-delete mr-2"></div>
                  删除该记录
                </el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </div>
      </template>
    </el-table-column>
  </el-table>
</template>

<script setup lang="ts">
import { formatDate } from "@/utils/format";
import type { ImportSourceGroup } from "@/types";
import type { DeleteTargetType } from "./shareImportHistoryTypes";

defineProps<{
  groups: ImportSourceGroup[];
}>();

const emit = defineEmits<{
  (event: "copy-url", url: string): void;
  (event: "delete", id: number | string, type: DeleteTargetType): void;
  (event: "update", url: string): void;
}>();

const requestDeleteGroup = (group: ImportSourceGroup) => {
  if (group.share_code) {
    emit("delete", group.share_code, "share");
    return;
  }
  emit("delete", group.url, "source");
};
</script>
