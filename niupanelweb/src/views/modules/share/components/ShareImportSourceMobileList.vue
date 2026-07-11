<template>
  <div class="h-full overflow-y-auto bg-white dark:bg-[#1c2431] pt-1 custom-scrollbar">
    <div
      v-for="row in groups"
      :key="getImportSourceGroupKey(row)"
      class="px-4 py-3 flex flex-col border-b border-light/30 last:border-b-0 active:bg-black/5 dark:active:bg-white/5 transition-colors group"
    >
      <div
        class="flex items-start justify-between w-full"
        @click="emit('toggle-group', row)"
      >
        <div class="flex flex-col gap-1 flex-1 overflow-hidden pr-2">
          <div class="flex items-center gap-1.5">
            <div class="i-ep-box text-primary shrink-0 text-sm"></div>
            <span class="text-[14px] font-semibold leading-snug text-default">
              {{ row.share_code || "历史资源包" }}
            </span>
            <span class="text-[10px] px-1.5 py-0.5 rounded text-primary font-bold">
              {{ row.task_count }} 任务
            </span>
            <div
              class="text-muted/30 ml-auto mr-2"
              :class="getGroupExpandIcon(row)"
            ></div>
          </div>
          <div class="flex items-center gap-1.5 opacity-70 mb-0.5 mt-0.5">
            <div class="i-ep-link text-[11px] text-muted shrink-0"></div>
            <span class="text-[11px] text-muted truncate font-mono">
              {{ row.url }}
            </span>
          </div>
          <div class="text-[10px] text-muted opacity-80 mt-1">
            最后更新:
            {{ formatImportHistoryDateCompact(row.last_updated_at * 1000) }}
          </div>
        </div>

        <div class="flex items-center shrink-0 h-10">
          <button
            type="button"
            class="mr-1 h-8 w-8 rounded-md text-primary flex-center transition-colors hover:bg-soft"
            title="重新导入"
            aria-label="重新导入资源"
            @click.stop="emit('update', row.url)"
          >
            <div class="i-ep-refresh"></div>
          </button>
          <el-dropdown trigger="click" @click.stop>
            <button
              type="button"
              class="h-8 w-8 rounded-md text-muted outline-none flex-center transition-colors hover:bg-soft hover:text-default"
              title="更多操作"
              aria-label="更多资源操作"
            >
              <div class="i-ep-more-filled"></div>
            </button>
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
      </div>

      <div
        v-if="expandedGroups.has(getImportSourceGroupKey(row))"
        class="mb-1 mt-3 flex max-h-[280px] flex-col gap-1.5 overflow-y-auto rounded-md border-t border-light/10 bg-base/5 p-2 pt-2 custom-scrollbar"
      >
        <div
          v-for="task in row.tasks"
          :key="task.id"
          class="flex items-center justify-between py-1 px-2.5 bg-white/40 dark:bg-black/20 rounded-md border border-light/5"
        >
          <div class="flex items-center gap-2 overflow-hidden flex-1">
            <div class="i-ep-document text-[10px] text-muted/40"></div>
            <span class="text-[11px] font-medium truncate opacity-90">
              {{ task.task_name }}
            </span>
          </div>
          <button
            type="button"
            class="ml-2 h-6 w-6 shrink-0 rounded-md text-danger/50 flex-center transition-colors hover:bg-danger/10 hover:text-danger"
            title="删除任务记录"
            aria-label="删除任务记录"
            @click.stop="emit('delete', task.id, 'task')"
          >
            <div class="i-ep-delete text-[10px]"></div>
          </button>
        </div>
        <div
          class="mt-1 py-1 text-center text-[9px] font-bold opacity-40"
          @click.stop="emit('toggle-group', row)"
        >
          点击上方收起
        </div>
      </div>

      <div
        v-else
        class="mt-2 flex flex-wrap gap-1.5 opacity-80"
        @click="emit('toggle-group', row)"
      >
        <div
          v-for="task in row.tasks.slice(0, 3)"
          :key="task.id"
          class="rounded border border-light/20 bg-base px-2 py-0.5 text-[10px] text-muted"
        >
          {{ task.task_name }}
        </div>
        <div
          v-if="row.tasks.length > 3"
          class="text-[10px] px-2 py-0.5 text-muted italic"
        >
          + {{ row.tasks.length - 3 }} 更多...
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { ImportSourceGroup } from "@/types";
import {
  formatImportHistoryDateCompact,
  getImportSourceGroupKey,
  type DeleteTargetType,
} from "./shareImportHistoryTypes";

const props = defineProps<{
  expandedGroups: Set<string>;
  groups: ImportSourceGroup[];
}>();

const emit = defineEmits<{
  (event: "copy-url", url: string): void;
  (event: "delete", id: number | string, type: DeleteTargetType): void;
  (event: "toggle-group", group: ImportSourceGroup): void;
  (event: "update", url: string): void;
}>();

const getGroupExpandIcon = (group: ImportSourceGroup) =>
  props.expandedGroups.has(getImportSourceGroupKey(group))
    ? "i-ep-arrow-up"
    : "i-ep-arrow-down";

const requestDeleteGroup = (group: ImportSourceGroup) => {
  if (group.share_code) {
    emit("delete", group.share_code, "share");
    return;
  }
  emit("delete", group.url, "source");
};
</script>
