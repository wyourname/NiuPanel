<template>
  <OverlayDrawer
    v-if="appStore.isMobile"
    v-model:visible="internalVisible"
    v-bind="$attrs"
    :title="title"
    :size="mobileDrawerSize"
    :direction="direction"
    :variant="isMobileFullscreen ? 'workspace' : 'sheet'"
    :content-preset="contentPreset"
    :destroy-on-close="destroyOnClose"
    :append-to-body="appendToBody"
    :show-close="showClose"
    :show-header="showHeader"
    :custom-class="customClass"
    @close="emit('close')"
    @closed="emit('closed')"
    @open="emit('open')"
    @opened="emit('opened')"
  >
    <template v-if="$slots.title" #title>
      <slot name="title" />
    </template>
    <template v-if="$slots['header-actions']" #header-actions>
      <slot name="header-actions" />
    </template>

    <slot />

    <template v-if="$slots.footer" #footer>
      <slot name="footer" />
    </template>
  </OverlayDrawer>

  <el-dialog
    v-else
    v-model="internalVisible"
    v-bind="$attrs"
    :title="title"
    :width="resolvedDesktopWidth"
    :style="desktopDialogStyle"
    :align-center="true"
    :fullscreen="fullscreen"
    :show-close="false"
    :destroy-on-close="destroyOnClose"
    :append-to-body="appendToBody"
    :class="[
      'responsive-dialog',
      'desktop-modal',
      `responsive-dialog--content-${contentPreset}`,
      showHeader ? '' : 'responsive-dialog--headerless',
      customClass,
    ]"
    @close="emit('close')"
    @closed="emit('closed')"
    @open="emit('open')"
    @opened="emit('opened')"
  >
    <template #header>
      <OverlayHeader
        v-if="showHeader"
        :title="title"
        :show-close="showClose"
        @close="internalVisible = false"
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
      <OverlayFooter>
        <slot name="footer" />
      </OverlayFooter>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, type CSSProperties } from "vue";
import { useAppStore } from "../../stores/app";
import OverlayDrawer, {
  type OverlayContentPreset,
  type OverlayDrawerDirection,
} from "./OverlayDrawer.vue";
import OverlayFooter from "./OverlayFooter.vue";
import OverlayHeader from "./OverlayHeader.vue";

defineOptions({ inheritAttrs: false });

export type ResponsiveDialogDesktopSize = "sm" | "md" | "lg" | "xl" | "fluid";
export type ResponsiveDialogMobileMode = "sheet" | "fullscreen";

type ResponsiveDialogProps = {
  appendToBody?: boolean;
  contentPreset?: OverlayContentPreset;
  customClass?: string;
  desktopHeight?: string | number;
  desktopSize?: ResponsiveDialogDesktopSize;
  destroyOnClose?: boolean;
  direction?: OverlayDrawerDirection;
  fullscreen?: boolean;
  mobileMode?: ResponsiveDialogMobileMode;
  showClose?: boolean;
  showHeader?: boolean;
  size?: string | number;
  title?: string;
  visible: boolean;
  width?: string | number;
};

const props = withDefaults(defineProps<ResponsiveDialogProps>(), {
  appendToBody: false,
  contentPreset: "form",
  customClass: "",
  desktopHeight: undefined,
  desktopSize: "md",
  destroyOnClose: false,
  direction: "btt",
  fullscreen: false,
  mobileMode: "sheet",
  showClose: true,
  showHeader: true,
  size: "auto",
  title: "",
  width: undefined,
});

const emit = defineEmits<{
  (event: "update:visible", value: boolean): void;
  (event: "close"): void;
  (event: "closed"): void;
  (event: "open"): void;
  (event: "opened"): void;
}>();

const appStore = useAppStore();

const desktopWidths: Record<ResponsiveDialogDesktopSize, string> = {
  sm: "440px",
  md: "560px",
  lg: "720px",
  xl: "920px",
  fluid: "min(1200px, calc(100vw - 32px))",
};

const internalVisible = computed({
  get: () => props.visible,
  set: (value) => emit("update:visible", value),
});

const resolvedDesktopWidth = computed(
  () => props.width ?? desktopWidths[props.desktopSize],
);

const desktopDialogStyle = computed<CSSProperties | undefined>(() => {
  if (props.desktopHeight === undefined || props.fullscreen) return undefined;

  return {
    height:
      typeof props.desktopHeight === "number"
        ? `${props.desktopHeight}px`
        : props.desktopHeight,
  };
});

const isMobileFullscreen = computed(
  () => props.mobileMode === "fullscreen" || props.size === "100%",
);

const mobileDrawerSize = computed(() =>
  isMobileFullscreen.value ? "100%" : props.size,
);
</script>

<style>
.responsive-dialog.el-dialog {
  box-sizing: border-box;
  display: flex;
  max-width: calc(100vw - 32px);
  max-height: calc(var(--app-viewport-height) - 32px);
  flex-direction: column;
  overflow: hidden;
  padding: 0;
  border: 1px solid var(--border-light);
  border-radius: var(--radius-md) !important;
  background: var(--bg-card);
  box-shadow: var(--shadow-md) !important;
}

.responsive-dialog .el-dialog__header,
.responsive-dialog .el-dialog__footer {
  margin: 0;
  padding: 0;
}

.responsive-dialog .el-dialog__body {
  display: flex;
  min-height: 0;
  flex: 1;
  flex-direction: column;
  overflow: hidden;
  padding: 0;
}

.responsive-dialog--headerless .el-dialog__header {
  display: none;
}
</style>
