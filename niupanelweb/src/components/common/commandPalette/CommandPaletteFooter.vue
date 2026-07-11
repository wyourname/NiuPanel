<template>
  <div class="palette-footer">
    <div class="footer-item">
      <span class="key">↵</span> <span>选择</span>
    </div>
    <div class="footer-item">
      <span class="key">↑↓</span> <span>导航</span>
    </div>
  </div>
</template>

<style scoped>
.palette-footer {
  padding: 8px 16px;
  background: var(--bg-base);
  border-top: 1px solid var(--border-base);
  display: flex;
  gap: 16px;
  font-size: 12px;
  color: var(--text-secondary);
}

@media (max-width: 768px) {
  .palette-footer {
    padding-bottom: calc(8px + env(safe-area-inset-bottom));
  }
}

.footer-item {
  display: flex;
  align-items: center;
  gap: 6px;
}

.key {
  background: var(--bg-hover);
  padding: 1px 4px;
  border-radius: 3px;
  font-family: monospace;
  font-weight: bold;
  border: 1px solid var(--border-light);
}
</style>
