<template>
  <div
    class="flex h-full min-h-0 flex-col overflow-hidden bg-[#0b0e14]"
  >
    <div
      class="flex h-11 shrink-0 select-none items-center justify-between border-b border-white/10 bg-white/[0.035] px-3 sm:px-4"
    >
      <div class="flex min-w-0 items-center gap-2.5">
        <div class="h-7 w-7 shrink-0 rounded-md bg-white/8 text-slate-200 flex-center">
          <div class="i-carbon-terminal text-[14px]"></div>
        </div>
        <div class="min-w-0">
          <div class="truncate text-xs font-semibold text-slate-100">系统终端</div>
          <div class="truncate text-[10px] text-slate-500">交互式 Shell 会话</div>
        </div>
      </div>

      <div class="flex shrink-0 items-center gap-2">
        <div class="flex h-7 items-center gap-1.5 rounded-md bg-white/6 px-2 text-[11px] font-medium text-slate-300">
          <span
            class="h-1.5 w-1.5 rounded-full"
            :class="connected ? 'bg-emerald-400' : 'bg-rose-400'"
          ></span>
          <span>{{ connected ? "已连接" : "未连接" }}</span>
        </div>
        <button
          type="button"
          class="h-7 w-7 cursor-pointer rounded-md text-slate-400 flex-center transition-colors hover:bg-white/10 hover:text-white"
          title="重新连接"
          aria-label="重新连接终端"
          @click="reconnect"
        >
          <div class="i-ep-refresh text-[14px]"></div>
        </button>
      </div>
    </div>

    <div class="min-h-0 flex-1 overflow-hidden bg-[#0b0e14] p-1.5 sm:p-2">
      <div ref="terminalRef" class="h-full w-full"></div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import "@xterm/xterm/css/xterm.css";
import { useTerminalSession } from "./composables/useTerminalSession";

const terminalRef = ref<HTMLElement | null>(null);

const { connected, reconnect } = useTerminalSession(terminalRef);
</script>

<style>
/* Adjust scrollbar for xterm */
.xterm-viewport::-webkit-scrollbar {
  width: 14px;
}
.xterm-viewport::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.1);
  border-radius: 4px;
}
.xterm-viewport::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.2);
}
</style>
