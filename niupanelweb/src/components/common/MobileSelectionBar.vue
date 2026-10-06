<template>
  <section v-if="count > 0" class="mobile-selection-bar" :aria-label="label">
    <div class="selection-header">
      <span>{{ busy ? '处理中…' : `已选 ${count} 项` }}</span>
      <button type="button" :disabled="busy" @click="$emit('select-all')">{{ isAllSelected ? '取消全选' : '全选' }}</button>
      <button type="button" :disabled="busy" @click="$emit('cancel')">取消</button>
    </div>
    <div class="selection-actions" :style="{ gridTemplateColumns: `repeat(${Math.min(5, actions.length + ($slots.more ? 1 : 0))}, minmax(0, 1fr))` }">
      <button v-for="action in actions" :key="action.command" type="button" :disabled="busy || action.disabled" :class="{ 'selection-danger': action.danger }" @click="$emit('command', action.command)">
        <span :class="action.icon" aria-hidden="true" />{{ action.label }}
      </button>
      <el-dropdown v-if="$slots.more" trigger="click" :disabled="busy" @command="$emit('command', String($event))">
        <button type="button" :disabled="busy" aria-label="更多批量操作"><span class="i-ep-more-filled" aria-hidden="true" />更多</button>
        <template #dropdown><el-dropdown-menu class="modern-dropdown"><slot name="more" /></el-dropdown-menu></template>
      </el-dropdown>
    </div>
  </section>
</template>
<script setup lang="ts">
export type MobileSelectionAction = { command: string; label: string; icon: string; danger?: boolean; disabled?: boolean }
withDefaults(defineProps<{ count: number; actions: MobileSelectionAction[]; isAllSelected: boolean; busy?: boolean; label?: string }>(), { busy: false, label: '多选操作' })
defineEmits<{ (event: 'command', command: string): void; (event: 'cancel' | 'select-all'): void }>()
</script>
<style scoped>
.mobile-selection-bar { position: fixed; bottom: calc(var(--mobile-dock-clearance) + 8px); left: 8px; right: 8px; z-index: 100; padding: 0 6px 6px; border: 1px solid var(--border-light); border-radius: 8px; background: var(--bg-card); box-shadow: var(--shadow-md); }
.selection-header { display: flex; align-items: center; min-height: 44px; border-bottom: 1px solid var(--border-light); }
.selection-header > span { flex: 1; padding-left: 6px; font-size: 12px; font-weight: 600; color: var(--text-default); }
.selection-header button { min-width: 44px; min-height: 44px; padding: 0 8px; font-size: 12px; color: var(--el-color-primary); }
.selection-actions { display: grid; padding-top: 4px; }
.selection-actions button { display: flex; width: 100%; flex-direction: column; align-items: center; justify-content: center; gap: 5px; min-height: 52px; min-width: 0; font-size: 11px; color: var(--text-secondary); }
.selection-actions button > span { width: 20px; height: 20px; }
.selection-actions .selection-danger { color: var(--el-color-danger); }
.selection-actions :deep(.el-dropdown) { min-width: 0; }
button { border: 0; border-radius: 6px; background: transparent; cursor: pointer; }
button:active:not(:disabled) { background: var(--bg-soft); }
button:disabled { opacity: .45; cursor: default; }
button:focus-visible { outline: 2px solid var(--el-color-primary); outline-offset: -2px; }
</style>
