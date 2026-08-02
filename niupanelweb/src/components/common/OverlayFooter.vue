<template>
  <footer
    :class="[
      'overlay-footer',
      mobile ? 'overlay-footer--mobile' : 'overlay-footer--desktop',
      safeArea ? 'overlay-footer--safe' : '',
    ]"
  >
    <slot />
  </footer>
</template>

<script setup lang="ts">
withDefaults(
  defineProps<{
    mobile?: boolean;
    safeArea?: boolean;
  }>(),
  {
    mobile: false,
    safeArea: false,
  },
);
</script>

<style scoped>
.overlay-footer {
  display: flex;
  width: 100%;
  box-sizing: border-box;
  flex-wrap: wrap;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  border-top: 1px solid var(--border-light);
  background: var(--bg-card);
}

.overlay-footer--desktop {
  padding: 10px 12px;
}

.overlay-footer--mobile {
  padding: 8px 12px;
}

.overlay-footer--mobile.overlay-footer--safe {
  padding-bottom: calc(8px + env(safe-area-inset-bottom, 0px));
}

.overlay-footer :deep(.el-button + .el-button) {
  margin-left: 0;
}

.overlay-footer--mobile :deep(.el-button:not(.is-link)) {
  min-height: var(--mobile-touch-target);
}
</style>
