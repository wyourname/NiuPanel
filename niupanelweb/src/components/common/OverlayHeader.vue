<template>
  <header
    :class="[
      'overlay-header',
      mobile ? 'overlay-header--mobile' : 'overlay-header--desktop',
      safeArea ? 'overlay-header--safe' : '',
    ]"
  >
    <button
      v-if="showClose && closeMode === 'back'"
      type="button"
      class="overlay-header__button overlay-header__button--leading"
      title="返回"
      aria-label="返回"
      @click="emit('close')"
    >
      <span class="i-ep-arrow-left" aria-hidden="true"></span>
    </button>

    <div class="overlay-header__title-wrap">
      <slot name="title">
        <div v-if="title" class="overlay-header__title" :title="title">
          {{ title }}
        </div>
      </slot>
    </div>

    <div v-if="$slots.actions" class="overlay-header__actions">
      <slot name="actions" />
    </div>

    <button
      v-if="showClose && closeMode === 'close'"
      type="button"
      class="overlay-header__button"
      title="关闭"
      aria-label="关闭"
      @click="emit('close')"
    >
      <span class="i-ep-close" aria-hidden="true"></span>
    </button>
  </header>
</template>

<script setup lang="ts">
type OverlayCloseMode = "back" | "close";

withDefaults(
  defineProps<{
    closeMode?: OverlayCloseMode;
    mobile?: boolean;
    safeArea?: boolean;
    showClose?: boolean;
    title?: string;
  }>(),
  {
    closeMode: "close",
    mobile: false,
    safeArea: false,
    showClose: true,
    title: "",
  },
);

const emit = defineEmits<{
  (event: "close"): void;
}>();
</script>

<style scoped>
.overlay-header {
  display: flex;
  width: 100%;
  min-width: 0;
  box-sizing: border-box;
  align-items: center;
  gap: 8px;
  border-bottom: 1px solid var(--border-light);
  background: var(--bg-card);
}

.overlay-header--desktop {
  min-height: 48px;
  padding: 0 8px 0 16px;
}

.overlay-header--mobile {
  min-height: 52px;
  padding: 0 8px 0 12px;
}

.overlay-header--mobile.overlay-header--safe {
  min-height: calc(52px + env(safe-area-inset-top, 0px));
  padding-top: env(safe-area-inset-top, 0px);
}

.overlay-header__title-wrap {
  min-width: 0;
  flex: 1;
}

.overlay-header__title {
  overflow: hidden;
  color: var(--text-default);
  font-size: var(--font-size-body);
  font-weight: 700;
  line-height: 1.35;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.overlay-header__actions {
  display: flex;
  min-width: 0;
  flex: 0 0 auto;
  align-items: center;
  gap: 8px;
}

.overlay-header__button {
  display: inline-flex;
  width: 36px;
  height: 36px;
  flex: 0 0 36px;
  cursor: pointer;
  align-items: center;
  justify-content: center;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-secondary);
  font-size: 18px;
  transition: color 0.16s ease, background-color 0.16s ease;
}

.overlay-header--mobile .overlay-header__button {
  width: var(--mobile-touch-target);
  height: var(--mobile-touch-target);
  flex-basis: var(--mobile-touch-target);
}

.overlay-header__button:hover {
  background: var(--bg-soft);
  color: var(--text-default);
}

.overlay-header__button:focus-visible {
  outline: 2px solid rgb(var(--brand-primary-rgb) / 0.5);
  outline-offset: 1px;
}

.overlay-header__button--leading {
  margin-left: -4px;
}
</style>
