<template>
  <el-drawer
    v-model="internalVisible"
    v-bind="$attrs"
    :title="title"
    :size="resolvedSize"
    :direction="resolvedDirection"
    :destroy-on-close="destroyOnClose"
    :append-to-body="appendToBody"
    :lock-scroll="lockScroll"
    :show-close="false"
    :with-header="showHeader"
    :class="[
      'overlay-drawer',
      `overlay-drawer--${variant}`,
      `overlay-drawer--content-${contentPreset}`,
      $slots.footer ? 'overlay-drawer--has-footer' : 'overlay-drawer--without-footer',
      customClass,
    ]"
    @close="emit('close')"
    @closed="emit('closed')"
    @open="emit('open')"
    @opened="emit('opened')"
  >
    <template v-if="showHeader" #header>
      <OverlayHeader
        :title="title"
        :mobile="appStore.isMobile"
        :safe-area="appStore.isMobile && variant !== 'sheet'"
        :show-close="showClose"
        :close-mode="variant === 'workspace' ? 'back' : 'close'"
        @close="handleHeaderClose"
      >
        <template v-if="$slots.title" #title>
          <slot name="title" />
        </template>
        <template v-if="$slots['header-actions']" #actions>
          <slot name="header-actions" />
        </template>
      </OverlayHeader>
    </template>

    <div :class="['overlay-content', `overlay-content--${contentPreset}`]">
      <slot />
    </div>

    <template v-if="$slots.footer" #footer>
      <OverlayFooter
        :mobile="appStore.isMobile"
        :safe-area="appStore.isMobile"
      >
        <slot name="footer" />
      </OverlayFooter>
    </template>
  </el-drawer>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useAppStore } from "../../stores/app";
import OverlayFooter from "./OverlayFooter.vue";
import OverlayHeader from "./OverlayHeader.vue";

defineOptions({ inheritAttrs: false });

export type OverlayContentPreset = "form" | "list" | "workspace";
export type OverlayDrawerDirection = "ltr" | "rtl" | "ttb" | "btt";
export type OverlayDrawerVariant = "sheet" | "side" | "workspace";

const props = withDefaults(
  defineProps<{
    appendToBody?: boolean;
    closeOnHeader?: boolean;
    contentPreset?: OverlayContentPreset;
    customClass?: string;
    destroyOnClose?: boolean;
    direction?: OverlayDrawerDirection;
    lockScroll?: boolean;
    showClose?: boolean;
    showHeader?: boolean;
    size?: string | number;
    title?: string;
    variant?: OverlayDrawerVariant;
    visible: boolean;
  }>(),
  {
    appendToBody: false,
    closeOnHeader: true,
    contentPreset: "form",
    customClass: "",
    destroyOnClose: false,
    direction: undefined,
    lockScroll: true,
    showClose: true,
    showHeader: true,
    size: undefined,
    title: "",
    variant: "sheet",
  },
);

const emit = defineEmits<{
  (event: "update:visible", value: boolean): void;
  (event: "close"): void;
  (event: "closed"): void;
  (event: "open"): void;
  (event: "opened"): void;
  (event: "request-close"): void;
}>();

const appStore = useAppStore();

const internalVisible = computed({
  get: () => props.visible,
  set: (value) => emit("update:visible", value),
});

const handleHeaderClose = () => {
  if (props.closeOnHeader) {
    internalVisible.value = false;
    return;
  }
  emit("request-close");
};

const resolvedDirection = computed<OverlayDrawerDirection>(() => {
  if (props.direction) return props.direction;
  return props.variant === "side" ? "rtl" : "btt";
});

const resolvedSize = computed<string | number>(() => {
  if (props.size !== undefined) return props.size;
  if (props.variant === "workspace") return "100%";
  if (props.variant === "side") return "420px";
  return "auto";
});
</script>

<style>
.overlay-drawer.el-drawer {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--bg-card) !important;
  box-shadow: var(--shadow-md) !important;
}

.overlay-drawer .el-drawer__header,
.overlay-drawer .el-drawer__footer {
  margin: 0;
  padding: 0;
}

.overlay-drawer .el-drawer__body {
  display: flex;
  min-height: 0;
  flex: 1;
  flex-direction: column;
  overflow: hidden;
  padding: 0;
}

.overlay-drawer--sheet.el-drawer.btt {
  max-height: min(88vh, 720px);
  max-height: min(88dvh, 720px);
  border-radius: var(--radius-md) var(--radius-md) 0 0 !important;
}

.overlay-drawer--workspace.el-drawer {
  width: 100% !important;
  height: var(--app-viewport-height) !important;
  max-height: var(--app-viewport-height) !important;
  border-radius: 0 !important;
}

.overlay-content {
  position: relative;
  display: flex;
  width: 100%;
  min-width: 0;
  min-height: 0;
  box-sizing: border-box;
  flex: 1;
  flex-direction: column;
}

.overlay-content--form,
.overlay-content--list {
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
}

.overlay-content--form {
  padding: 16px;
}

.overlay-content--list {
  padding: 12px;
}

.overlay-content--workspace {
  overflow: hidden;
  padding: 0;
}

@media (max-width: 768px) {
  .overlay-drawer--side.el-drawer {
    width: 100% !important;
    height: var(--app-viewport-height) !important;
    max-height: var(--app-viewport-height) !important;
  }

  .overlay-content--form {
    padding: 12px;
  }

  .overlay-content--list {
    padding: 8px;
  }

  .overlay-drawer--sheet.overlay-drawer--without-footer .overlay-content--form {
    padding-bottom: calc(12px + env(safe-area-inset-bottom, 0px));
  }

  .overlay-drawer--sheet.overlay-drawer--without-footer .overlay-content--list {
    padding-bottom: calc(8px + env(safe-area-inset-bottom, 0px));
  }
}
</style>
