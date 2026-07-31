<template>
  <div class="flex items-center justify-between shrink-0 min-h-[32px] gap-2">
    <div
      class="flex shrink-0 gap-0.5 rounded-md bg-gray-100 p-1 dark:bg-dark-900"
      :class="isMobile ? 'flex-1 min-w-0' : 'w-[200px]'"
    >
      <button
        v-for="item in tabItems"
        :key="item.value"
        type="button"
        class="flex-1 rounded py-1.5 font-bold transition-colors duration-200"
        :class="[
          activeTab === item.value
            ? 'bg-white dark:bg-gray-800 text-primary shadow-sm'
            : 'text-gray-500 hover:text-gray-700 dark:text-gray-400',
          isMobile ? 'text-[11px]' : 'text-xs',
        ]"
        @click="activeTab = item.value"
      >
        {{ item.label }}
      </button>
    </div>

    <div class="flex items-center gap-1.5 shrink-0">
      <div
        class="flex h-9 items-center gap-1.5 rounded-md bg-gray-100 px-2.5 font-mono text-[11px] font-bold dark:bg-dark-900"
      >
        <span
          v-if="enabled"
          class="w-1.5 h-1.5 rounded-full bg-green-500 animate-pulse"
        ></span>
        <span v-else class="w-1.5 h-1.5 rounded-full bg-red-500"></span>
        <span v-if="!enabled" class="text-red-500">离线</span>
        <span v-else class="text-green-600 dark:text-green-400">
          {{ latencyLabel }}
        </span>
      </div>

      <el-dropdown trigger="click" @command="$emit('fileAction', $event)">
        <button
          type="button"
          class="h-9 w-9 rounded-md bg-orange-500/10 text-orange-600 flex-center transition-colors hover:bg-orange-500/15"
          title="发送文件"
          aria-label="发送文件"
        >
          <div class="i-ep-share text-base"></div>
        </button>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item command="local">
              <div class="i-ep-upload-filled mr-2 text-primary"></div>
              <span class="text-xs font-bold">发送本地文件</span>
            </el-dropdown-item>
            <el-dropdown-item command="server">
              <div class="i-ep-folder mr-2 text-orange-500"></div>
              <span class="text-xs font-bold">发送服务器文件</span>
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>

      <button
        type="button"
        class="h-9 rounded-md bg-primary text-[11px] font-bold text-white flex-center gap-1.5 transition-colors hover:bg-primary/90"
        title="机器人配置"
        aria-label="机器人配置"
        :class="isMobile ? 'w-9' : 'px-5'"
        @click="$emit('openSettings')"
      >
        <div class="i-ep-setting text-base"></div>
        <span v-if="!isMobile">配置</span>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";

export type TelegramTabItem = {
  label: string;
  value: string;
};

const props = defineProps<{
  enabled: boolean;
  isMobile: boolean;
  latency: number;
  tabItems: readonly TelegramTabItem[];
}>();

defineEmits<{
  (e: "fileAction", command: unknown): void;
  (e: "openSettings"): void;
}>();

const activeTab = defineModel<string>("activeTab", { required: true });

const latencyLabel = computed(() => {
  if (props.latency <= 0) return "在线";
  if (props.isMobile && props.latency > 999) return "···";
  return `${props.latency}ms`;
});
</script>
