<template>
  <section
    class="fixed overflow-hidden bg-card transition-[border-color,box-shadow] duration-150"
    :class="[
      window.maximized ? 'rounded-md' : 'rounded-lg',
      active
        ? 'border border-[var(--accent-subtle-border)] shadow-[0_16px_42px_rgba(15,23,42,0.19)] dark:shadow-[0_18px_46px_rgba(0,0,0,0.44)]'
        : 'border border-base shadow-[0_8px_24px_rgba(15,23,42,0.10)] opacity-[0.98] dark:shadow-[0_10px_28px_rgba(0,0,0,0.28)]',
    ]"
    :style="windowStyle"
    @pointerdown="emit('focus')"
  >
    <header
      class="relative flex h-9 shrink-0 items-center border-b border-light px-2.5 select-none"
      :class="active ? 'bg-[var(--accent-subtle-bg)]' : 'bg-subtle dark:bg-white/[0.035]'"
      @pointerdown="startDrag"
    >
      <div class="z-10 flex w-[86px] shrink-0 items-center">
        <el-dropdown trigger="click" @command="handlePlacementCommand">
          <button
            type="button"
            class="h-7 w-7 rounded-md text-muted flex-center transition-colors hover:bg-card hover:text-default focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/35 dark:hover:bg-white/10"
            aria-label="窗口布局"
            @click.stop
            @pointerdown.stop
          >
            <div class="i-ep-more-filled text-[13px]"></div>
          </button>
          <template #dropdown>
            <el-dropdown-menu class="modern-dropdown">
              <el-dropdown-item command="left">
                <div class="flex items-center gap-2">
                  <div class="i-ep-d-arrow-left"></div>
                  左半屏
                </div>
              </el-dropdown-item>
              <el-dropdown-item command="right">
                <div class="flex items-center gap-2">
                  <div class="i-ep-d-arrow-right"></div>
                  右半屏
                </div>
              </el-dropdown-item>
              <el-dropdown-item command="center">
                <div class="flex items-center gap-2">
                  <div class="i-ep-rank"></div>
                  居中
                </div>
              </el-dropdown-item>
              <el-dropdown-item divided command="restore">
                <div class="flex items-center gap-2">
                  <div class="i-ep-refresh-left"></div>
                  还原大小
                </div>
              </el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
      </div>

      <div class="pointer-events-none absolute inset-x-24 top-0 flex h-full items-center justify-center gap-2">
        <div :class="window.icon" class="shrink-0 text-[13px] text-primary"></div>
        <div class="min-w-0 truncate text-[12px] font-bold text-default">
          {{ window.title }}
        </div>
      </div>

      <div class="z-10 ml-auto flex w-[96px] shrink-0 items-center justify-end">
        <div class="flex items-center gap-0.5 rounded-md border border-light bg-card p-0.5 dark:bg-white/[0.045]">
          <button
            type="button"
            class="h-[26px] w-[26px] rounded-sm text-muted flex-center transition-colors hover:bg-soft hover:text-default focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/35 dark:hover:bg-white/10"
            aria-label="最小化窗口"
            @click.stop="emit('minimize')"
            @pointerdown.stop
          >
            <div class="i-ep-minus text-[12px]"></div>
          </button>
          <button
            type="button"
            class="h-[26px] w-[26px] rounded-sm flex-center transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/35"
            :class="
              window.maximized
                ? 'accent-subtle'
                : 'text-muted hover:bg-soft hover:text-default dark:hover:bg-white/10'
            "
            aria-label="最大化窗口"
            @click.stop="emit('toggle-maximize')"
            @pointerdown.stop
          >
            <div class="i-ep-full-screen text-[12px]"></div>
          </button>
          <button
            type="button"
            class="h-[26px] w-[26px] rounded-sm text-muted flex-center transition-colors hover:bg-rose-500/10 hover:text-rose-600 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-rose-400/40 dark:hover:bg-rose-500/15 dark:hover:text-rose-300"
            aria-label="关闭窗口"
            @click.stop="emit('close')"
            @pointerdown.stop
          >
            <div class="i-ep-close text-[13px]"></div>
          </button>
        </div>
      </div>
    </header>

    <div class="h-[calc(100%-36px)] min-h-0 overflow-hidden">
      <slot></slot>
    </div>

    <template v-if="!window.maximized">
      <div
        v-for="handle in resizeHandles"
        :key="handle.direction"
        :class="handle.class"
        @pointerdown.stop="startResize($event, handle.direction)"
      ></div>
      <div class="pointer-events-none absolute bottom-1 right-1 h-2.5 w-2.5 rounded-br-md border-b border-r border-muted/40"></div>
    </template>
  </section>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type {
  WorkspaceWindow,
  WorkspaceWindowPlacement,
} from "@/types/workspace";

type ResizeDirection =
  | "n"
  | "s"
  | "e"
  | "w"
  | "ne"
  | "nw"
  | "se"
  | "sw";

type ResizeHandle = {
  direction: ResizeDirection;
  class: string;
};

const props = defineProps<{
  active: boolean;
  window: WorkspaceWindow;
}>();

const emit = defineEmits<{
  (event: "close"): void;
  (event: "focus"): void;
  (event: "minimize"): void;
  (event: "place", placement: WorkspaceWindowPlacement): void;
  (event: "toggle-maximize"): void;
  (event: "update-bounds", bounds: Partial<WorkspaceWindow["bounds"]>): void;
}>();

const windowStyle = computed(() => ({
  left: `${props.window.bounds.x}px`,
  top: `${props.window.bounds.y}px`,
  width: `${props.window.bounds.width}px`,
  height: `${props.window.bounds.height}px`,
  zIndex: props.window.zIndex,
}));

const resizeHandles: ResizeHandle[] = [
  {
    direction: "n",
    class: "absolute left-4 right-4 top-0 h-[3px] cursor-ns-resize",
  },
  {
    direction: "s",
    class: "absolute bottom-0 left-4 right-4 h-[3px] cursor-ns-resize",
  },
  {
    direction: "e",
    class: "absolute bottom-4 right-0 top-4 w-[3px] cursor-ew-resize",
  },
  {
    direction: "w",
    class: "absolute bottom-4 left-0 top-4 w-[3px] cursor-ew-resize",
  },
  {
    direction: "ne",
    class: "absolute right-0 top-0 h-3 w-3 cursor-nesw-resize",
  },
  {
    direction: "nw",
    class: "absolute left-0 top-0 h-3 w-3 cursor-nwse-resize",
  },
  {
    direction: "se",
    class: "absolute bottom-0 right-0 h-4 w-4 cursor-nwse-resize",
  },
  {
    direction: "sw",
    class: "absolute bottom-0 left-0 h-4 w-4 cursor-nesw-resize",
  },
];

const isWindowPlacement = (value: unknown): value is WorkspaceWindowPlacement =>
  value === "left" ||
  value === "right" ||
  value === "center" ||
  value === "restore";

const handlePlacementCommand = (command: unknown) => {
  if (isWindowPlacement(command)) emit("place", command);
};

const startDrag = (event: PointerEvent) => {
  if (props.window.maximized) return;
  const target = event.target as HTMLElement | null;
  if (target?.closest("button")) return;

  emit("focus");
  const startX = event.clientX;
  const startY = event.clientY;
  const { x, y } = props.window.bounds;

  const move = (moveEvent: PointerEvent) => {
    emit("update-bounds", {
      x: Math.max(8, x + moveEvent.clientX - startX),
      y: Math.max(8, y + moveEvent.clientY - startY),
    });
  };

  const stop = () => {
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", stop);
  };

  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", stop);
};

const startResize = (event: PointerEvent, direction: ResizeDirection) => {
  event.preventDefault();
  emit("focus");
  const startX = event.clientX;
  const startY = event.clientY;
  const startBounds = { ...props.window.bounds };
  const minWidth = 480;
  const minHeight = 360;

  const move = (moveEvent: PointerEvent) => {
    const dx = moveEvent.clientX - startX;
    const dy = moveEvent.clientY - startY;
    const next = { ...startBounds };

    if (direction.includes("e")) {
      next.width = Math.max(minWidth, startBounds.width + dx);
    }

    if (direction.includes("s")) {
      next.height = Math.max(minHeight, startBounds.height + dy);
    }

    if (direction.includes("w")) {
      const proposedWidth = startBounds.width - dx;
      if (proposedWidth >= minWidth) {
        next.x = Math.max(8, startBounds.x + dx);
        next.width = startBounds.width + startBounds.x - next.x;
      } else {
        next.x = startBounds.x + startBounds.width - minWidth;
        next.width = minWidth;
      }
    }

    if (direction.includes("n")) {
      const proposedHeight = startBounds.height - dy;
      if (proposedHeight >= minHeight) {
        next.y = Math.max(8, startBounds.y + dy);
        next.height = startBounds.height + startBounds.y - next.y;
      } else {
        next.y = startBounds.y + startBounds.height - minHeight;
        next.height = minHeight;
      }
    }

    emit("update-bounds", next);
  };

  const stop = () => {
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", stop);
  };

  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", stop);
};
</script>
