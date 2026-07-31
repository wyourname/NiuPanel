<template>
  <div
    v-if="isMobile"
    class="relative flex items-center gap-3 min-h-[72px] px-3 py-2.5"
  >
    <div v-if="selectionMode" class="shrink-0 flex-center">
      <el-checkbox
        :model-value="isSelected"
        class="!mr-0"
        @change="emit('selection-change', !!$event)"
        @click.stop
      />
    </div>

    <span
      class="h-2 w-2 shrink-0 rounded-full"
      :class="statusDotClass"
      aria-hidden="true"
    ></span>

    <div class="min-w-0 flex-1">
      <div class="flex min-w-0 items-center gap-2">
        <span
          class="min-w-0 flex-1 truncate text-[13px] font-bold"
          :class="task.enabled ? 'text-default' : 'text-muted'"
        >
          {{ task.name }}
        </span>
        <span v-if="task.is_pinned" class="i-ep-top shrink-0 text-[10px] text-amber-500"></span>
        <span
          v-if="task.status !== 'Failed'"
          class="shrink-0 text-[10px] font-semibold"
          :class="statusTextClass"
        >
          {{ statusText }}
        </span>
      </div>

      <div class="mt-1 flex min-w-0 items-center gap-1.5 text-[10px] text-muted">
        <span class="shrink-0">{{ scheduleInfo }}</span>
        <span class="opacity-50">·</span>
        <span class="truncate">{{ environmentLabel }}</span>
        <template v-if="task.status === 'Failed'">
          <span class="shrink-0 opacity-50">·</span>
          <span class="shrink-0 text-[10px] font-semibold text-rose-600 dark:text-rose-400">
            {{ statusText }}
          </span>
        </template>
      </div>

      <div class="mt-1 flex min-w-0 items-center justify-between gap-2 text-[10px]">
        <div v-if="task.status === 'Running'" class="min-w-0 flex-1 truncate font-mono text-secondary">
          <span v-if="task.cpu_usage !== undefined">CPU {{ task.cpu_usage.toFixed(1) }}%</span>
          <span v-if="task.cpu_usage !== undefined && task.memory_usage !== undefined" class="mx-1 opacity-40">·</span>
          <span v-if="task.memory_usage !== undefined">内存 {{ (task.memory_usage / 1024 / 1024).toFixed(1) }} MB</span>
          <span v-if="task.cpu_usage === undefined && task.memory_usage === undefined" class="text-primary">正在启动</span>
        </div>
        <span v-else class="min-w-0 flex-1 truncate text-muted/80">
          上次 {{ formatLastTime }}
        </span>

        <span
          v-if="nextRunText"
          class="inline-flex shrink-0 items-center gap-1 font-medium text-[var(--accent-subtle-text)]"
          :title="`下次运行 ${nextRunText}`"
        >
          <span class="i-ep-clock text-[10px]"></span>
          下次 {{ nextRunText }}
        </span>
      </div>
    </div>

    <div v-if="!selectionMode" class="shrink-0 flex items-center gap-0.5">
      <button
        type="button"
        class="h-8 rounded-md px-2 text-[11px] font-semibold text-primary transition-colors hover:bg-soft"
        :title="primaryActionLabel"
        @click.stop="handlePrimaryAction"
      >
        <span :class="primaryActionIcon" class="mr-1 align-[-1px]"></span>
        {{ primaryActionLabel }}
      </button>
      <button
        type="button"
        class="h-8 w-8 rounded-md text-muted flex-center transition-colors hover:bg-soft hover:text-default"
        title="更多操作"
        @click.stop="emit('more-actions')"
      >
        <span class="i-ep-more-filled"></span>
      </button>
    </div>
  </div>

  <!-- 桌面端:信息收拢 + 状态胶囊 + 环境图标 -->
  <div
    v-else
    class="task-desktop-row group/row"
  >
    <div class="flex shrink-0 items-center gap-2.5">
      <div v-if="selectionMode" class="shrink-0 flex-center">
        <el-checkbox
          :model-value="isSelected"
          class="!mr-0"
          @change="emit('selection-change', !!$event)"
          @click.stop
        />
      </div>

      <div class="relative h-9 w-9 shrink-0 rounded-md border border-light bg-subtle text-secondary flex-center">
        <span :class="getEnvIcon" class="text-[17px]"></span>
        <span
          class="absolute -right-1 -top-1 h-2.5 w-2.5 rounded-full border-2 border-card"
          :class="statusDotClass"
          aria-hidden="true"
        ></span>
      </div>
    </div>

    <div class="min-w-0 flex-1">
      <div class="flex min-w-0 items-center gap-1.5">
        <span
          class="truncate text-[13px] font-semibold"
          :class="task.enabled ? 'text-default' : 'text-muted'"
        >
          {{ task.name }}
        </span>
        <span v-if="task.is_pinned" class="i-ep-top shrink-0 text-[10px] text-amber-500"></span>
      </div>
      <div class="mt-1 flex min-w-0 items-center gap-2 font-mono text-[10px] text-muted">
        <span class="min-w-0 flex-1 truncate">
          {{ scheduleInfo }} · {{ environmentLabel }}
        </span>
        <span class="shrink-0 text-[10px] text-muted/80">
          {{ desktopActivityText }}
        </span>
      </div>
    </div>

    <div
      class="task-desktop-row__trailing"
      :class="{
        'has-actions': !selectionMode,
        'is-active': isSelected && !selectionMode,
      }"
    >
      <span
        class="task-desktop-row__status max-w-full truncate rounded px-1.5 py-0.5 text-[10px] font-semibold"
        :class="statusPillClass"
      >
        {{ statusText }}
      </span>

      <div
        v-if="!selectionMode"
        class="task-desktop-row__actions"
      >
        <button
          type="button"
          class="h-8 w-8 rounded-md text-primary flex-center transition-colors hover:bg-soft"
          :title="primaryActionLabel"
          :aria-label="primaryActionLabel"
          @click.stop="handlePrimaryAction"
        >
          <span :class="primaryActionIcon"></span>
        </button>
        <button
          type="button"
          class="h-8 w-8 rounded-md text-muted flex-center transition-colors hover:bg-soft hover:text-default"
          title="更多操作"
          aria-label="更多操作"
          @click.stop="openMenuFromButton"
        >
          <span class="i-ep-more-filled"></span>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, toRef } from "vue";
import { useTaskCardPresentation } from "../../composables/useTaskCardPresentation";
import type { Task } from "@/types";

const props = defineProps<{
  isMobile: boolean;
  isSelected: boolean;
  selectionMode: boolean;
  task: Task;
}>();

const emit = defineEmits<{
  (event: "more-actions"): void;
  (event: "run"): void;
  (event: "selection-change", selected: boolean): void;
  (event: "stop"): void;
}>();

const {
  environmentLabel,
  formatLastTime,
  getEnvIcon,
  nextRunText,
  primaryActionIcon,
  primaryActionLabel,
  scheduleInfo,
  statusDotClass,
  statusText,
} = useTaskCardPresentation(toRef(props, "task"));

// 移动端沿用文字色;桌面端用设计系统的 *-subtle 状态胶囊
const statusTextClass = computed(() => {
  if (!props.task.enabled) return "text-muted";
  if (props.task.status === "Running" || props.task.status === "Finished") return "text-emerald-600 dark:text-emerald-400";
  if (props.task.status === "Failed") return "text-rose-600 dark:text-rose-400";
  if (props.task.status === "Paused") return "text-amber-600 dark:text-amber-400";
  return "text-muted";
});

const statusPillClass = computed(() => {
  if (!props.task.enabled) return "bg-subtle text-muted";
  switch (props.task.status) {
    case "Running":
    case "Finished":
      return "success-subtle";
    case "Failed":
      return "danger-subtle";
    case "Paused":
      return "warning-subtle";
    default:
      return "bg-subtle text-secondary";
  }
});

const desktopActivityText = computed(() => {
  if (props.task.status !== "Running") return formatLastTime.value;
  if (props.task.cpu_usage !== undefined) {
    return `CPU ${props.task.cpu_usage.toFixed(1)}%`;
  }
  if (props.task.memory_usage !== undefined) {
    return `内存 ${(props.task.memory_usage / 1024 / 1024).toFixed(1)}M`;
  }
  return "正在启动";
});

const handlePrimaryAction = () => {
  if (props.task.status === "Running" || props.task.status === "Paused" || !props.task.enabled) {
    emit("more-actions");
    return;
  }
  emit("run");
};

// 桌面端“···”:在按钮位置派发原生 contextmenu,复用行的右键下拉菜单,
// 不持有任何组件实例,避免卸载后访问导致的报错
const openMenuFromButton = (event: MouseEvent) => {
  if (props.isMobile) {
    emit("more-actions");
    return;
  }
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
  (event.currentTarget as HTMLElement).dispatchEvent(
    new MouseEvent("contextmenu", {
      bubbles: true,
      cancelable: true,
      clientX: rect.left + rect.width / 2,
      clientY: rect.bottom,
    }),
  );
};
</script>

<style scoped>
.task-desktop-row {
  position: relative;
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) 68px;
  align-items: center;
  column-gap: 12px;
  min-height: 68px;
  padding: 10px 12px;
}

.task-desktop-row__trailing {
  position: relative;
  display: flex;
  width: 68px;
  height: 32px;
  align-items: center;
  justify-content: flex-end;
}

.task-desktop-row__status,
.task-desktop-row__actions {
  transition: opacity 0.16s ease;
}

.task-desktop-row__actions {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 2px;
  opacity: 0;
  pointer-events: none;
}

.task-desktop-row:hover .task-desktop-row__trailing.has-actions .task-desktop-row__status,
.task-desktop-row:focus-within .task-desktop-row__trailing.has-actions .task-desktop-row__status,
.task-desktop-row__trailing.has-actions.is-active .task-desktop-row__status {
  opacity: 0;
}

.task-desktop-row:hover .task-desktop-row__trailing.has-actions .task-desktop-row__actions,
.task-desktop-row:focus-within .task-desktop-row__trailing.has-actions .task-desktop-row__actions,
.task-desktop-row__trailing.has-actions.is-active .task-desktop-row__actions {
  opacity: 1;
  pointer-events: auto;
}

@media (prefers-reduced-motion: reduce) {
  .task-desktop-row__status,
  .task-desktop-row__actions {
    transition: none;
  }
}
</style>
