<template>
  <div
    v-show="visible"
    ref="menuRef"
    class="fixed z-[90] min-w-[184px] select-none rounded-lg border border-light bg-card p-1 shadow-md"
    role="menu"
    :style="{ top: `${position.y}px`, left: `${position.x}px` }"
    @contextmenu.prevent
  >
    <div v-for="(item, index) in items" :key="index">
      <div v-if="item.type === 'divider'" class="mx-2 my-1 h-px bg-light"></div>
      <div v-else class="group relative">
        <button
          type="button"
          class="flex h-8 w-full cursor-pointer items-center justify-between gap-3 rounded-md px-2.5 text-left text-[12px] font-semibold text-default transition-colors hover:bg-soft"
          role="menuitem"
          :class="item.class"
          @click.stop="handleAction(item)"
        >
          <span class="flex min-w-0 items-center gap-2.5">
            <div v-if="item.icon" :class="item.icon" class="text-[15px] opacity-80"></div>
            <span class="truncate">{{ item.label }}</span>
          </span>
          <div
            v-if="hasChildren(item)"
            class="i-ep-arrow-right text-[13px] text-muted"
          ></div>
        </button>

        <div
          v-if="hasChildren(item)"
          class="pointer-events-none invisible absolute top-[-0.25rem] z-10 min-w-[176px] rounded-lg border border-light bg-card p-1 opacity-0 shadow-md transition-opacity group-hover:pointer-events-auto group-hover:visible group-hover:opacity-100"
          role="menu"
          :class="submenuPanelClass"
        >
          <div v-for="(child, childIndex) in item.children" :key="childIndex">
            <div v-if="child.type === 'divider'" class="mx-2 my-1 h-px bg-light"></div>
            <button
              v-else
              type="button"
              class="flex h-8 w-full cursor-pointer items-center gap-2.5 rounded-md px-2.5 text-left text-[12px] font-semibold text-default transition-colors hover:bg-soft"
              role="menuitem"
              :class="child.class"
              @click.stop="handleAction(child)"
            >
              <div v-if="child.icon" :class="child.icon" class="text-[15px] opacity-80"></div>
              <span class="truncate">{{ child.label }}</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from "vue";
import { onClickOutside } from "@vueuse/core";
import type {
  ContextMenuItem,
  ContextMenuPosition,
} from "./contextMenuTypes";

type ContextMenuProps = {
  visible?: boolean;
  position?: ContextMenuPosition;
  items?: ContextMenuItem[];
};

const props = withDefaults(defineProps<ContextMenuProps>(), {
  visible: false,
  position: () => ({ x: 0, y: 0 }),
  items: () => [],
});

const emit = defineEmits<{
  (event: "update:visible", value: boolean): void;
  (event: "select", action: string): void;
}>();
const menuRef = ref<HTMLElement | null>(null);
const submenuPanelClass = computed(() => {
  if (typeof window === "undefined") return "left-[calc(100%-0.25rem)]";
  return props.position.x > window.innerWidth - 420
    ? "right-[calc(100%-0.25rem)]"
    : "left-[calc(100%-0.25rem)]";
});

onClickOutside(menuRef, () => {
  if (props.visible) {
    emit("update:visible", false);
  }
});

const hasChildren = (item: ContextMenuItem) =>
  Array.isArray(item.children) && item.children.length > 0;

const handleAction = (item: ContextMenuItem) => {
  if (hasChildren(item) && !item.action) return;

  if (item.action) {
    emit("select", item.action);
  }
  emit("update:visible", false);
};

// Close on scroll or window resize
const close = () => emit("update:visible", false);

onMounted(() => {
  window.addEventListener("scroll", close, true);
  window.addEventListener("resize", close);
});

onUnmounted(() => {
  window.removeEventListener("scroll", close, true);
  window.removeEventListener("resize", close);
});
</script>
