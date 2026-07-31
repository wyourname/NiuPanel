<template>
  <template v-if="appStore.isMobile">
    <el-drawer
      v-model="internalVisible"
      v-bind="$attrs"
      :title="title"
      :size="size || '100%'"
      :direction="direction"
      :destroy-on-close="destroyOnClose"
      :append-to-body="appendToBody"
      :show-close="false"
      :class="['responsive-dialog mobile-sheet', customClass]"
      @close="emit('close')"
      @closed="emit('closed')"
      @open="emit('open')"
      @opened="emit('opened')"
    >
      <template #header>
        <div class="responsive-dialog__mobile-header">
          <div v-if="title" class="responsive-dialog__title">{{ title }}</div>
          <button
            type="button"
            class="responsive-dialog__close"
            title="关闭"
            aria-label="关闭"
            @click="internalVisible = false"
          >
            <span class="i-ep-close"></span>
          </button>
        </div>
      </template>

      <div class="h-full flex flex-col overflow-hidden relative">
        <slot />
      </div>

      <template v-if="$slots.footer" #footer>
        <div class="responsive-dialog-footer">
          <slot name="footer" />
        </div>
      </template>
    </el-drawer>
  </template>

  <template v-else>
    <el-dialog
      v-model="internalVisible"
      v-bind="$attrs"
      :title="title"
      :width="width"
      :align-center="true"
      :fullscreen="fullscreen"
      :destroy-on-close="destroyOnClose"
      :append-to-body="appendToBody"
      :class="['responsive-dialog desktop-modal', customClass]"
      @close="emit('close')"
      @closed="emit('closed')"
      @open="emit('open')"
      @opened="emit('opened')"
    >
      <div class="h-full flex flex-col overflow-hidden relative">
        <slot />
      </div>

      <template v-if="$slots.footer" #footer>
        <div class="responsive-dialog-footer">
          <slot name="footer" />
        </div>
      </template>
    </el-dialog>
  </template>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useAppStore } from "../../stores/app";

type ResponsiveDialogDirection = "ltr" | "rtl" | "ttb" | "btt";

type ResponsiveDialogProps = {
  visible: boolean;
  title?: string;
  width?: string | number;
  size?: string | number;
  fullscreen?: boolean;
  direction?: ResponsiveDialogDirection;
  destroyOnClose?: boolean;
  appendToBody?: boolean;
  customClass?: string;
};

const props = withDefaults(defineProps<ResponsiveDialogProps>(), {
  title: "",
  width: "500px",
  size: "auto",
  fullscreen: false,
  direction: "btt",
  destroyOnClose: false,
  appendToBody: false,
  customClass: "",
});

const emit = defineEmits<{
  (event: "update:visible", value: boolean): void;
  (event: "close"): void;
  (event: "closed"): void;
  (event: "open"): void;
  (event: "opened"): void;
}>();
const appStore = useAppStore();

const internalVisible = computed({
  get: () => props.visible,
  set: (val) => emit("update:visible", val),
});
</script>

<style>
/* Normalize Drawer/Dialog body to allow flex layouts inside */
.responsive-dialog .el-dialog__body {
  padding: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.responsive-dialog .el-dialog__header,
.responsive-dialog .el-dialog__footer,
.responsive-dialog .el-drawer__footer {
  margin: 0;
}

.responsive-dialog .el-drawer__body {
  padding: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.mobile-sheet .el-drawer__body {
  padding: 0 !important;
}

.responsive-dialog-footer {
  width: 100%;
  padding: 12px;
  border-top: 1px solid var(--border-light);
  background: var(--bg-card);
}

.responsive-dialog .el-drawer__header {
  margin-bottom: 0;
  padding: 0;
}

.responsive-dialog__mobile-header {
  display: flex;
  width: 100%;
  min-width: 0;
  height: 48px;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 0 12px 0 16px;
  border-bottom: 1px solid var(--border-light);
  background: var(--bg-card);
}

.responsive-dialog__title {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-default);
  font-size: var(--font-size-body);
  font-weight: 700;
}

.responsive-dialog__close {
  display: inline-flex;
  width: 32px;
  height: 32px;
  flex: 0 0 32px;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  transition: color 0.16s ease, background-color 0.16s ease;
}

.responsive-dialog__close:hover {
  color: var(--text-default);
  background: var(--bg-soft);
}

/* Mobile Sheet Specifics */
.mobile-sheet.el-drawer.btt {
  border-top-left-radius: var(--radius-md) !important;
  border-top-right-radius: var(--radius-md) !important;
  background-color: var(--bg-card) !important;
  box-shadow: 0 -10px 30px rgba(15, 23, 42, 0.14) !important;
}

/* Desktop Modal Specifics */
.desktop-modal.el-dialog {
  border-radius: var(--radius-md) !important;
  overflow: hidden;
  box-shadow: var(--shadow-md) !important;
}

.desktop-modal .el-dialog__header {
  margin-right: 0;
  padding: 14px 16px;
  border-bottom: 1px solid var(--border-light);
}

.desktop-modal .el-dialog__footer {
  padding: 0;
}

.desktop-modal .el-dialog__title {
  font-size: var(--font-size-body);
  font-weight: 700;
  letter-spacing: 0;
}
</style>
