<template>
  <div
    v-if="workspace.windows.length > 0"
    class="fixed inset-0 z-40 pointer-events-none"
  >
    <template v-if="!appStore.isMobile">
      <WorkspaceWindow
        v-for="item in workspace.visibleWindows"
        :key="item.id"
        :window="item"
        :active="workspace.activeWindowId === item.id"
        class="pointer-events-auto"
        @close="workspace.requestCloseWindow(item.id)"
        @focus="workspace.focusWindow(item.id)"
        @minimize="workspace.minimizeWindow(item.id)"
        @place="workspace.placeWindow(item.id, $event)"
        @toggle-maximize="workspace.toggleMaximizeWindow(item.id)"
        @update-bounds="workspace.updateWindowBounds(item.id, $event)"
      >
        <TaskLogWorkspaceWindow
          v-if="item.appId === 'task-log'"
          :payload="taskLogPayload(item.payload)"
        />
        <TaskEditorWorkspaceWindow
          v-else-if="item.appId === 'task-editor'"
          :payload="taskEditorPayload(item.payload)"
          :window-id="item.id"
        />
        <FileEditorWorkspaceWindow
          v-else-if="item.appId === 'file-editor'"
          :payload="fileEditorPayload(item.payload)"
          :window-id="item.id"
        />
        <PluginHostView
          v-else-if="isPluginWindow(item.appId)"
          :plugin-id="pluginPayload(item.payload).pluginId"
          :route-path="pluginPayload(item.payload).routePath"
          :route-query="pluginPayload(item.payload).routeQuery"
          @navigate="workspace.navigatePluginWindow(item.id, $event)"
        />
        <component
          :is="appComponent(item.appId)"
          v-else-if="appComponent(item.appId)"
        />
      </WorkspaceWindow>
    </template>

    <template v-else>
      <section
        v-if="workspace.activeWindow"
        class="mobile-dock-safe pointer-events-auto fixed inset-0 z-40 flex flex-col bg-base"
      >
        <header class="h-14 shrink-0 border-b border-light bg-card px-3 flex items-center gap-3">
          <button
            type="button"
            class="accent-subtle h-9 w-9 rounded-md flex-center transition-colors"
            aria-label="关闭窗口"
            @click="workspace.requestCloseWindow(workspace.activeWindow.id)"
          >
            <div class="i-ep-arrow-left text-base"></div>
          </button>
          <div class="min-w-0 flex-1">
            <div class="truncate text-sm font-bold text-default">
              {{ workspace.activeWindow.title }}
            </div>
            <div v-if="workspace.activeWindow.subtitle" class="truncate text-[10px] font-medium text-muted">
              {{ workspace.activeWindow.subtitle }}
            </div>
          </div>
        </header>

        <div
          v-if="workspace.windows.length > 1"
          class="shrink-0 border-b border-light bg-card px-2 py-2 flex gap-1 overflow-x-auto no-scrollbar"
        >
          <button
            v-for="item in workspace.windows"
            :key="item.id"
            type="button"
            class="h-8 shrink-0 rounded-lg px-3 text-[11px] font-bold transition-colors"
            :class="
              workspace.activeWindowId === item.id
                ? 'accent-subtle'
                : 'text-muted hover:bg-soft hover:text-default'
            "
            @click="workspace.focusWindow(item.id)"
          >
            {{ item.title }}
          </button>
        </div>

        <div class="min-h-0 flex-1 overflow-hidden">
          <TaskLogWorkspaceWindow
            v-if="workspace.activeWindow.appId === 'task-log'"
            :payload="taskLogPayload(workspace.activeWindow.payload)"
            is-mobile
          />
          <TaskEditorWorkspaceWindow
            v-else-if="workspace.activeWindow.appId === 'task-editor'"
            :payload="taskEditorPayload(workspace.activeWindow.payload)"
            :window-id="workspace.activeWindow.id"
          />
          <FileEditorWorkspaceWindow
            v-else-if="workspace.activeWindow.appId === 'file-editor'"
            :payload="fileEditorPayload(workspace.activeWindow.payload)"
            :window-id="workspace.activeWindow.id"
          />
          <PluginHostView
            v-else-if="isPluginWindow(workspace.activeWindow.appId)"
            :plugin-id="pluginPayload(workspace.activeWindow.payload).pluginId"
            :route-path="pluginPayload(workspace.activeWindow.payload).routePath"
            :route-query="pluginPayload(workspace.activeWindow.payload).routeQuery"
            @navigate="workspace.navigatePluginWindow(workspace.activeWindow.id, $event)"
          />
          <component
            :is="appComponent(workspace.activeWindow.appId)"
            v-else-if="appComponent(workspace.activeWindow.appId)"
          />
        </div>
      </section>
    </template>
  </div>
</template>

<script setup lang="ts">
import { useAppStore } from "@/stores/app";
import { useWorkspaceStore } from "@/stores/workspace";
import type {
  FileEditorWindowPayload,
  PluginWorkspaceWindowPayload,
  TaskEditorWindowPayload,
  TaskLogWindowPayload,
  WorkspaceAppId,
  WorkspaceWindowPayload,
} from "@/types/workspace";
import { workspaceAppComponents } from "@/workspace/components";
import PluginHostView from "@/views/plugins/PluginHostView.vue";
import FileEditorWorkspaceWindow from "./FileEditorWorkspaceWindow.vue";
import TaskEditorWorkspaceWindow from "./TaskEditorWorkspaceWindow.vue";
import TaskLogWorkspaceWindow from "./TaskLogWorkspaceWindow.vue";
import WorkspaceWindow from "./WorkspaceWindow.vue";

const appStore = useAppStore();
const workspace = useWorkspaceStore();

const taskLogPayload = (payload: WorkspaceWindowPayload) =>
  payload as TaskLogWindowPayload;

const taskEditorPayload = (payload: WorkspaceWindowPayload) =>
  payload as TaskEditorWindowPayload;

const fileEditorPayload = (payload: WorkspaceWindowPayload) =>
  payload as FileEditorWindowPayload;

const pluginPayload = (payload: WorkspaceWindowPayload) =>
  payload as PluginWorkspaceWindowPayload;

const isPluginWindow = (appId: WorkspaceAppId) => appId.startsWith("plugin:");

const appComponent = (appId: WorkspaceAppId) => workspaceAppComponents[appId];
</script>
