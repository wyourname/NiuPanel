<template>
  <nav class="file-breadcrumbs" :class="{ 'file-breadcrumbs--mobile': mobile }" aria-label="文件目录">
    <button type="button" class="up-button" aria-label="返回上级目录" :disabled="currentPath === '/' || currentPath === ''" @click="emit('back')">
      <span class="i-ep-top" aria-hidden="true" />
    </button>
    <div ref="trail" class="breadcrumb-trail">
      <button type="button" class="breadcrumb-part" :aria-current="currentPath === '/' || currentPath === '' ? 'location' : undefined" @click="emit('navigate', '/')">
        <span class="i-ep-house" aria-hidden="true" /><span>根目录</span>
      </button>
      <template v-for="(item, index) in collapsedBreadcrumbs" :key="index">
        <span class="breadcrumb-separator i-ep-arrow-right" aria-hidden="true" />
        <el-dropdown v-if="item.type === 'ellipsis'" trigger="click" @command="handleCommand">
          <button type="button" class="breadcrumb-part" aria-label="展开上级目录">…</button>
          <template #dropdown>
            <el-dropdown-menu class="modern-dropdown">
              <el-dropdown-item v-for="hidden in item.items" :key="hidden.path" :command="hidden.path">{{ hidden.name }}</el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
        <button v-else type="button" class="breadcrumb-part" :title="item.name" :aria-current="index === collapsedBreadcrumbs.length - 1 ? 'location' : undefined" @click="emit('navigate', item.path)">
          <span class="truncate">{{ item.name }}</span>
        </button>
      </template>
    </div>
  </nav>
</template>
<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'
import type { Breadcrumb } from '@/composables/useFileOperations'
const props = withDefaults(defineProps<{ currentPath: string; collapsedBreadcrumbs: Breadcrumb[]; mobile?: boolean }>(), { mobile: false })
const emit = defineEmits<{ (event: 'back'): void; (event: 'navigate', path: string): void }>()
const trail = ref<HTMLElement | null>(null)
const handleCommand = (path: unknown) => { if (typeof path === 'string') emit('navigate', path) }
watch(() => props.currentPath, async () => {
  await nextTick()
  if (trail.value) trail.value.scrollLeft = trail.value.scrollWidth
}, { immediate: true })
</script>
<style scoped>
.file-breadcrumbs { display: flex; align-items: center; gap: 4px; min-width: 0; height: 32px; }
.breadcrumb-trail { display: flex; align-items: center; min-width: 0; gap: 2px; overflow-x: auto; scrollbar-width: none; }
.up-button, .breadcrumb-part { display: inline-flex; align-items: center; justify-content: center; gap: 5px; flex-shrink: 0; height: 30px; border: 0; border-radius: 5px; color: var(--text-secondary); background: transparent; cursor: pointer; font-size: 11px; }
.up-button { width: 30px; font-size: 15px; }
.up-button:disabled { opacity: .3; cursor: default; }
.breadcrumb-part { max-width: 150px; padding: 0 6px; }
.breadcrumb-part[aria-current] { color: var(--text-default); font-weight: 600; }
.breadcrumb-separator { flex-shrink: 0; width: 10px; height: 10px; color: var(--text-muted); }
button:hover:not(:disabled), button:active:not(:disabled) { background: var(--bg-soft); color: var(--el-color-primary); }
.file-breadcrumbs--mobile { height: 44px; }
.file-breadcrumbs--mobile button { min-height: 44px; min-width: 44px; font-size: 12px; }
button:focus-visible { outline: 2px solid var(--el-color-primary); outline-offset: -2px; }
</style>
