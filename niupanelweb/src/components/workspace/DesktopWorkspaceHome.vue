<template>
  <section
    class="desktop-bg relative h-full overflow-hidden bg-base"
    @contextmenu.prevent="openDesktopContextMenu"
    @dragenter.prevent="handleDesktopDragEnter"
    @dragleave.prevent="handleDesktopDragLeave"
    @dragover.prevent="handleDesktopDragOver"
    @drop.prevent="handleDesktopDrop"
  >
    <div class="absolute inset-0 z-10 overflow-y-auto no-scrollbar">
      <div class="mx-auto flex min-h-full w-full max-w-[1180px] flex-col gap-5 px-6 pb-24 pt-8">
        <!-- 签名元素:今日时间轴 -->
        <DesktopTimeline
          :stations="stations"
          :clock="clock"
          :day-start="dayStart"
          :loading="taskStore.loading && !taskStore.tasks.length"
          @open-log="openTaskLog"
          @run="(task) => handleTaskAction('run', task)"
          @create="workspace.openTaskCreateWindow()"
        />

        <!-- 小组件区 -->
        <div class="grid grid-cols-1 gap-5 lg:grid-cols-3">
          <DesktopWidget title="正在运行" :count="runningTasks.length" content-class="overflow-hidden">
            <div v-if="!runningTasks.length" class="flex h-full min-h-[96px] flex-col items-center justify-center text-center">
              <span class="i-ep-cpu text-[22px] text-muted opacity-60"></span>
              <p class="mt-2 text-[11px] text-muted">暂无后台任务</p>
            </div>
            <DesktopTaskPager v-else :items="runningTasks" label="正在运行的任务">
              <template #item="{ task }">
                <div
                  class="group flex min-w-0 items-center gap-2 rounded-md px-2 py-1.5 transition-colors hover:bg-soft focus-within:bg-soft"
                >
                  <span class="h-2 w-2 shrink-0 rounded-full bg-emerald-500"></span>
                  <div class="min-w-0 flex-1">
                    <button
                      type="button"
                      class="block w-full cursor-pointer truncate text-left text-[11px] font-semibold leading-4 text-default hover:text-primary focus-visible:outline-none"
                      @click="openTaskLog(task)"
                    >{{ task.name }}</button>
                    <span class="block truncate font-mono text-[9px] leading-3 tabular-nums text-muted">{{ cpuUsage(task) }} · {{ memoryUsage(task) }}</span>
                  </div>
                  <button
                    type="button"
                    class="h-7 w-7 shrink-0 cursor-pointer rounded-md danger-subtle opacity-0 flex-center transition-opacity group-hover:opacity-100 group-focus-within:opacity-100 focus-visible:opacity-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-rose-400/40"
                    :aria-label="`停止任务 ${task.name}`"
                    title="停止"
                    @click="handleTaskAction('stop', task)"
                  >
                    <span class="i-ep-switch-button text-[12px]"></span>
                  </button>
                </div>
              </template>
            </DesktopTaskPager>
          </DesktopWidget>

          <DesktopWidget title="需要处理" :count="attentionTasks.length" tone="warning" content-class="overflow-hidden">
            <div v-if="!attentionTasks.length" class="flex h-full min-h-[96px] flex-col items-center justify-center text-center">
              <span class="i-ep-circle-check text-[22px] text-emerald-500 opacity-70"></span>
              <p class="mt-2 text-[11px] text-muted">一切正常，没有失败任务</p>
            </div>
            <DesktopTaskPager v-else :items="attentionTasks" label="需要处理的任务">
              <template #item="{ task }">
                <div
                  class="group flex min-w-0 items-center gap-2 rounded-md px-2 py-1.5 transition-colors hover:bg-soft focus-within:bg-soft"
                >
                  <span class="h-2 w-2 shrink-0 rounded-full" :class="getStatusDotClass(task)"></span>
                  <div class="min-w-0 flex-1">
                    <button
                      type="button"
                      class="block w-full cursor-pointer truncate text-left text-[11px] font-semibold leading-4 text-default hover:text-primary focus-visible:outline-none"
                      @click="openTaskLog(task)"
                    >{{ task.name }}</button>
                    <span class="block truncate text-[9px] font-semibold leading-3 text-muted">{{ getStatusLabel(task.status) }}</span>
                  </div>
                  <button
                    type="button"
                    class="h-7 w-7 shrink-0 cursor-pointer rounded-md accent-subtle opacity-0 flex-center transition-opacity group-hover:opacity-100 group-focus-within:opacity-100 focus-visible:opacity-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40"
                    :aria-label="`重新运行任务 ${task.name}`"
                    title="重新运行"
                    @click="handleTaskAction('run', task)"
                  >
                    <span class="i-ep-refresh-right text-[12px]"></span>
                  </button>
                </div>
              </template>
            </DesktopTaskPager>
          </DesktopWidget>

          <DesktopWidget title="快捷操作" content-class="overflow-y-auto no-scrollbar">
            <div class="flex min-h-[96px] flex-col">
              <div class="grid grid-cols-3 gap-2">
                <button
                  type="button"
                  class="group flex min-w-0 cursor-pointer flex-col items-center gap-1.5 rounded-lg border border-light bg-card px-2 py-2.5 text-secondary transition-colors hover:border-primary/35 hover:bg-soft hover:text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40"
                  @click="workspace.openTaskCreateWindow()"
                >
                  <span class="h-7 w-7 rounded-md accent-subtle flex-center">
                    <span class="i-ep-plus text-[14px]"></span>
                  </span>
                  <span class="truncate text-[10px] font-semibold">新建任务</span>
                </button>
                <button
                  type="button"
                  class="group flex min-w-0 cursor-pointer flex-col items-center gap-1.5 rounded-lg border border-light bg-card px-2 py-2.5 text-secondary transition-colors hover:border-primary/35 hover:bg-soft hover:text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40"
                  @click="openQuickCreate()"
                >
                  <span class="h-7 w-7 rounded-md accent-subtle flex-center">
                    <span class="i-ep-link text-[14px]"></span>
                  </span>
                  <span class="truncate text-[10px] font-semibold">从 URL 导入</span>
                </button>
                <button
                  type="button"
                  class="group flex min-w-0 cursor-pointer flex-col items-center gap-1.5 rounded-lg border border-light bg-card px-2 py-2.5 text-secondary transition-colors hover:border-primary/35 hover:bg-soft hover:text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40"
                  @click="workspace.openAppWindow('tasks')"
                >
                  <span class="h-7 w-7 rounded-md accent-subtle flex-center">
                    <span class="i-ep-list text-[14px]"></span>
                  </span>
                  <span class="truncate text-[10px] font-semibold">任务管理</span>
                </button>
              </div>

              <section v-if="workspacePluginApps.length" class="border-t border-light/70 pt-2">
                <div class="mb-1.5 flex items-center justify-between px-0.5">
                  <h3 class="text-[10px] font-semibold text-muted">已安装扩展</h3>
                  <span class="rounded-full bg-subtle px-1.5 py-0.5 text-[9px] font-bold text-muted">{{ workspacePluginApps.length }}</span>
                </div>
                <div class="grid gap-1.5">
                  <button
                    v-for="app in workspacePluginApps"
                    :key="app.plugin_id"
                    type="button"
                    class="group flex min-w-0 cursor-pointer items-center gap-2 rounded-lg border border-light bg-card p-2 text-left text-secondary transition-colors hover:border-primary/35 hover:bg-soft hover:text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40"
                    :title="`打开扩展 ${primaryPluginRoute(app)?.title ?? app.name}`"
                    @click="workspace.openPluginAppWindow(app)"
                  >
                    <span class="h-8 w-8 shrink-0 rounded-md accent-subtle flex-center">
                      <span :class="primaryPluginRoute(app)?.icon ?? 'i-ep-box'" class="text-[15px]"></span>
                    </span>
                    <span class="min-w-0 flex-1">
                      <span class="block truncate text-[11px] font-semibold">{{ primaryPluginRoute(app)?.title ?? app.name }}</span>
                      <span class="block truncate text-[9px] leading-4 text-muted group-hover:text-secondary">{{ primaryPluginRoute(app)?.description ?? app.description ?? '打开扩展' }}</span>
                    </span>
                    <span class="i-ep-arrow-right shrink-0 text-[12px] text-muted transition-transform duration-200 group-hover:translate-x-0.5 motion-reduce:transition-none"></span>
                  </button>
                </div>
              </section>
              <p v-else class="text-center text-[10px] leading-4 text-muted">也可将 Python、Node 或 Shell 脚本直接拖到桌面</p>
            </div>
          </DesktopWidget>
        </div>

        <!-- 置顶任务:桌面图标 -->
        <section v-if="pinnedTasks.length" class="shrink-0">
          <h2 class="mb-2 px-1 text-[11px] font-semibold text-muted">已固定</h2>
          <div class="flex flex-wrap gap-2">
            <button
              v-for="task in pinnedTasks"
              :key="task.id"
              type="button"
              class="group flex w-[92px] cursor-pointer flex-col items-center gap-2 rounded-lg px-2 py-3 transition-colors hover:bg-soft"
              @dblclick="openTaskLog(task)"
              @contextmenu.prevent.stop="openTaskContextMenu($event, task)"
            >
              <div class="relative h-11 w-11 rounded-lg border border-light bg-card text-secondary shadow-sm flex-center">
                <span :class="getEnvIcon(task)" class="text-[20px]"></span>
                <span class="absolute -right-1 -top-1 h-3 w-3 rounded-full border-2 border-card" :class="getStatusDotClass(task)"></span>
              </div>
              <span class="w-full truncate text-center text-[11px] font-medium text-secondary group-hover:text-default">{{ task.name }}</span>
            </button>
          </div>
        </section>
      </div>
    </div>

    <div v-if="isDraggingUpload" class="pointer-events-none absolute inset-6 z-30 rounded-lg border border-dashed border-primary/55 bg-card/95 flex-center">
      <div class="text-center">
        <span class="i-ep-upload-filled text-[30px] text-primary"></span>
        <div class="mt-3 text-[13px] font-bold text-default">拖放脚本创建任务</div>
        <div class="mt-1 text-[11px] text-muted">支持 Python、Node、Shell 脚本</div>
      </div>
    </div>
    <ContextMenu v-model:visible="contextMenuVisible" :items="contextMenuItems" :position="contextMenuPosition" @select="handleContextMenuSelect" />
    <TaskQuickCreateDialog v-model:visible="quickCreateVisible" v-model:url="quickCreateForm.url" :creating="quickCreating" @submit="handleQuickCreate" />
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import type { Task } from "@/types";
import ContextMenu from "@/components/common/ContextMenu.vue";
import type { ContextMenuItem, ContextMenuPosition } from "@/components/common/contextMenuTypes";
import TaskQuickCreateDialog from "@/components/tasks/TaskQuickCreateDialog.vue";
import DesktopTimeline from "@/components/workspace/DesktopTimeline.vue";
import DesktopTaskPager from "@/components/workspace/DesktopTaskPager.vue";
import DesktopWidget from "@/components/workspace/DesktopWidget.vue";
import { isSupportedScriptFileName } from "@/composables/taskWizardHelpers";
import { useTaskQuickCreate } from "@/composables/useTaskQuickCreate";
import { getEnvIcon, getStatusDotClass, getStatusLabel } from "@/composables/useTaskPresentation";
import { useTaskDeleteConfirmation } from "@/composables/useTaskDeleteConfirmation";
import { useTodaySchedule } from "@/composables/useTodaySchedule";
import { primaryPluginRoute, usePluginAppsStore } from "@/stores/pluginApps";
import { useTaskStore } from "@/stores/tasks";
import { useWorkspaceStore } from "@/stores/workspace";
import { formatFileSize } from "@/utils/format";

type Action = "new_task" | "quick_create_task" | "open_tasks" | "refresh_tasks" | "next_window" | "tile_windows" | "cascade_windows" | "minimize_all" | "close_all_windows" | "open_log" | "run" | "pause" | "resume" | "stop" | "edit" | "script" | "variables" | "cron" | "pin" | "unpin" | "enable" | "disable" | "delete";

const taskStore = useTaskStore();
const pluginApps = usePluginAppsStore();
const workspace = useWorkspaceStore();
const { quickCreateVisible, quickCreating, quickCreateForm, openQuickCreate, handleQuickCreate } = useTaskQuickCreate(taskStore.refreshTasks);
const { clock, dayStart, stations } = useTodaySchedule(() => taskStore.tasks);
const contextMenuVisible = ref(false);
const contextMenuPosition = ref<ContextMenuPosition>({ x: 0, y: 0 });
const contextTask = ref<Task | null>(null);
const selectedTaskId = ref<number | null>(null);
const selectedIds = ref<number[]>([]);
const dragDepth = ref(0);
const isDraggingUpload = ref(false);
const { handleDelete } = useTaskDeleteConfirmation({ clearAllSelection: () => { selectedIds.value = []; }, selectedIds, taskStore });

const timeValue = (task: Task) => new Date(task.updated_at || task.created_at || 0).getTime() || 0;
const runningTasks = computed(() => taskStore.tasks.filter((task) => task.status === "Running").sort((a, b) => timeValue(b) - timeValue(a)));
const attentionTasks = computed(() => taskStore.tasks.filter((task) => task.status === "Failed" || task.status === "Paused"));
const pinnedTasks = computed(() => taskStore.tasks.filter((task) => task.is_pinned));
const workspacePluginApps = computed(() => pluginApps.workspaceApps);

const cpuUsage = (task: Task) => typeof task.cpu_usage === "number" ? `${task.cpu_usage.toFixed(1)}%` : "--";
const memoryUsage = (task: Task) => typeof task.memory_usage === "number" ? formatFileSize(task.memory_usage).replace(/\s+/g, "") : "--";
const openTaskLog = (task: Task) => { selectedTaskId.value = task.id; workspace.openTaskLogWindow(task); };
const openTaskContextMenu = (event: MouseEvent, task: Task) => { selectedTaskId.value = task.id; contextTask.value = task; contextMenuPosition.value = { x: Math.min(event.clientX, window.innerWidth - 224), y: Math.min(event.clientY, window.innerHeight - 380) }; contextMenuVisible.value = true; };
const openDesktopContextMenu = (event: MouseEvent) => { contextTask.value = null; contextMenuPosition.value = { x: Math.min(event.clientX, window.innerWidth - 224), y: Math.min(event.clientY, window.innerHeight - 380) }; contextMenuVisible.value = true; };

const desktopItems = computed<ContextMenuItem[]>(() => [{ label: "新建任务", action: "new_task", icon: "i-ep-plus" }, { label: "从 URL 导入", action: "quick_create_task", icon: "i-ep-link" }, { label: "打开任务管理", action: "open_tasks", icon: "i-ep-list" }, { type: "divider" }, { label: "刷新工作台", action: "refresh_tasks", icon: "i-ep-refresh" }, ...(workspace.windows.length ? [{ type: "divider" as const }, { label: "切换窗口", action: "next_window", icon: "i-ep-sort" }, { label: "平铺窗口", action: "tile_windows", icon: "i-ep-grid" }, { label: "整理窗口", action: "cascade_windows", icon: "i-ep-copy-document" }, { label: "最小化所有窗口", action: "minimize_all", icon: "i-ep-minus" }, { label: "关闭所有窗口", action: "close_all_windows", icon: "i-ep-close", class: "text-rose-600 dark:text-rose-400" }] : [])]);
const taskItems = computed<ContextMenuItem[]>(() => { const task = contextTask.value; if (!task) return desktopItems.value; const actions: ContextMenuItem[] = [{ label: "查看日志", action: "open_log", icon: "i-ep-document-copy" }]; if (task.status === "Running") actions.push({ label: "暂停任务", action: "pause", icon: "i-ep-video-pause" }, { label: "停止任务", action: "stop", icon: "i-ep-switch-button" }); else if (task.status === "Paused") actions.push({ label: "恢复任务", action: "resume", icon: "i-ep-refresh" }, { label: "停止任务", action: "stop", icon: "i-ep-switch-button" }); else actions.push({ label: "立即运行", action: "run", icon: "i-ep-video-play" }); actions.push({ type: "divider" }, { label: "任务配置", icon: "i-ep-setting", children: [{ label: "编辑任务", action: "edit", icon: "i-ep-edit" }, { label: "编辑脚本", action: "script", icon: "i-ep-document" }, { label: "变量", action: "variables", icon: "i-ep-key" }, { label: "定时规则", action: "cron", icon: "i-ep-clock" }] }, { type: "divider" }, { label: task.is_pinned ? "取消固定" : "固定到桌面", action: task.is_pinned ? "unpin" : "pin", icon: "i-ep-collection-tag" }, { label: task.enabled ? "禁用任务" : "启用任务", action: task.enabled ? "disable" : "enable", icon: task.enabled ? "i-ep-close" : "i-ep-check" }, { label: "删除任务", action: "delete", icon: "i-ep-delete", class: "text-rose-600 dark:text-rose-400" }); return actions; });
const contextMenuItems = computed(() => contextTask.value ? taskItems.value : desktopItems.value);
const handleTaskAction = async (action: Action, task: Task) => { if (action === "open_log") openTaskLog(task); else if (action === "run") { await taskStore.runTask(task.id); openTaskLog(task); } else if (action === "pause") await taskStore.pauseTask(task.id); else if (action === "resume") await taskStore.resumeTask(task.id); else if (action === "stop") await taskStore.stopTask(task.id); else if (["edit", "script", "variables", "cron"].includes(action)) workspace.openTaskEditorWindow(task, action as "edit" | "script" | "variables" | "cron"); else if (action === "pin") await taskStore.pinTask(task.id); else if (action === "unpin") await taskStore.unpinTask(task.id); else if (action === "enable") await taskStore.toggleEnable(task, true); else if (action === "disable") await taskStore.toggleEnable(task, false); else if (action === "delete") handleDelete(task.id); };
const handleDesktopAction = async (action: Action) => { if (action === "new_task") workspace.openTaskCreateWindow(); else if (action === "quick_create_task") openQuickCreate(); else if (action === "open_tasks") workspace.openAppWindow("tasks"); else if (action === "refresh_tasks") await taskStore.refreshTasks(); else if (action === "next_window") workspace.focusNextWindow(); else if (action === "tile_windows") workspace.tileVisibleWindows(); else if (action === "cascade_windows") workspace.cascadeWindows(); else if (action === "minimize_all") workspace.minimizeAll(); else if (action === "close_all_windows") await workspace.requestCloseAll(); };
const handleContextMenuSelect = (action: string) => { const typed = action as Action; contextTask.value ? void handleTaskAction(typed, contextTask.value) : void handleDesktopAction(typed); };
const isFileDragEvent = (event: DragEvent) => Array.from(event.dataTransfer?.types ?? []).includes("Files");
const handleDesktopDragEnter = (event: DragEvent) => { if (isFileDragEvent(event)) { dragDepth.value += 1; isDraggingUpload.value = true; } };
const handleDesktopDragLeave = (event: DragEvent) => { if (isFileDragEvent(event)) { dragDepth.value = Math.max(0, dragDepth.value - 1); if (!dragDepth.value) isDraggingUpload.value = false; } };
const handleDesktopDragOver = (event: DragEvent) => { if (isFileDragEvent(event)) { if (event.dataTransfer) event.dataTransfer.dropEffect = "copy"; isDraggingUpload.value = true; } };
const handleDesktopDrop = async (event: DragEvent) => { if (!isFileDragEvent(event)) return; dragDepth.value = 0; isDraggingUpload.value = false; const scripts = Array.from(event.dataTransfer?.files ?? []).filter((file) => isSupportedScriptFileName(file.name)); if (!scripts.length) return void ElMessage.warning("仅支持 Python、Node、Shell 脚本文件"); const file = scripts[0]; try { await ElMessageBox.confirm(`检测到脚本文件 ${file.name}，是否创建任务？`, "创建任务", { type: "info", confirmButtonText: "创建任务", cancelButtonText: "取消" }); workspace.openTaskCreateWindow({ uploadedFile: file }); } catch { /* User cancelled. */ } };
onMounted(() => {
  void taskStore.init();
  void pluginApps.loadApps().catch(() => {});
});
onUnmounted(() => taskStore.stopStatusStream());
</script>
