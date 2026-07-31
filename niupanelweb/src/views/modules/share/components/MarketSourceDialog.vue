<template>
  <ResponsiveDialog
    v-model:visible="visible"
    title="管理订阅源"
    width="500px"
    append-to-body
    destroy-on-close
  >
    <div class="p-5 flex flex-col gap-6">
      <div class="space-y-4 rounded-md border border-light bg-base/50 p-4">
        <div class="flex items-center gap-2">
          <div class="i-ep-plus text-primary"></div>
          <span class="text-[11px] font-bold text-muted">添加新订阅源</span>
        </div>
        <el-form :model="form" label-position="top">
          <el-form-item>
            <el-input v-model="form.name" placeholder="订阅源名称 (如: 大神A的仓库)" class="modern-input" />
          </el-form-item>
          <el-form-item>
            <el-input v-model="form.url" placeholder="订阅源 URL (如: https://worker.me/market.json)" class="modern-input" />
          </el-form-item>
          <el-button type="primary" class="w-full !h-9 !rounded-md font-bold" :loading="adding" @click="handleAdd">
            添加订阅
          </el-button>
        </el-form>
      </div>

      <div class="space-y-3">
        <div class="flex items-center gap-2 px-1">
          <div class="i-ep-list text-muted"></div>
          <span class="text-[11px] font-bold text-muted">已添加的源（{{ sources.length }}）</span>
        </div>
        <div v-if="loading" class="flex-center py-10">
          <div class="i-ep-loading animate-spin text-2xl opacity-20"></div>
        </div>
        <div v-else-if="sources.length === 0" class="text-center py-10 text-xs text-muted italic">
          暂无订阅源，快去添加一个吧
        </div>
        <div v-else class="max-h-[300px] overflow-y-auto space-y-2 pr-1 custom-scrollbar">
          <div v-for="source in sources" :key="source.id"
            class="group flex items-center justify-between rounded-md border border-light/50 bg-base p-3 transition-colors hover:border-primary/30">
            <div class="flex flex-col gap-0.5 min-w-0">
              <span class="text-xs font-bold text-default truncate">{{ source.name }}</span>
              <span class="text-[9px] font-mono text-muted truncate opacity-50">{{ source.url }}</span>
            </div>
            <div class="flex items-center gap-1 shrink-0 ml-4">
              <button class="btn-icon !w-7 !h-7 hover:text-primary" @click="handleSync(source)" :title="'同步'">
                <div class="i-ep-refresh" :class="{ 'animate-spin': syncingId === source.id }"></div>
              </button>
              <button class="btn-icon !w-7 !h-7 hover:text-rose-500" @click="handleDelete(source)" :title="'删除'">
                <div class="i-ep-delete"></div>
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import ResponsiveDialog from '../../../../components/common/ResponsiveDialog.vue';
import * as shareApi from '../../../../api/share';
import type { CreateMarketSourceRequest, MarketSource } from '@/types';

const props = defineProps<{
  modelValue: boolean;
}>();

const emit = defineEmits<{
  (event: 'update:modelValue', visible: boolean): void;
  (event: 'update-scripts'): void;
}>();

const visible = ref(props.modelValue);
watch(() => props.modelValue, (val) => visible.value = val);
watch(visible, (val) => emit('update:modelValue', val));

const loading = ref(false);
const adding = ref(false);
const syncingId = ref<number | null>(null);
const sources = ref<MarketSource[]>([]);
const form = ref<CreateMarketSourceRequest>({ name: '', url: '', description: '' });

const fetchSources = async () => {
  loading.value = true;
  try {
    const res = await shareApi.listMarketSources();
    sources.value = res.data || [];
  } finally {
    loading.value = false;
  }
};

const handleAdd = async () => {
  if (!form.value.name || !form.value.url) {
    ElMessage.warning("请填写完整信息");
    return;
  }
  adding.value = true;
  try {
    await shareApi.addMarketSource(form.value);
    ElMessage.success("订阅成功");
    form.value = { name: '', url: '', description: '' };
    await fetchSources();
    emit('update-scripts');
  } catch (error) {
    ElMessage.error(error instanceof Error ? error.message : "订阅失败");
  } finally {
    adding.value = false;
  }
};

const handleSync = async (source: MarketSource) => {
  syncingId.value = source.id;
  try {
    await shareApi.syncMarketSource(source.id);
    ElMessage.success(`${source.name} 同步完成`);
    emit('update-scripts');
  } catch (error) {
    ElMessage.error(error instanceof Error ? error.message : "同步失败");
  } finally {
    syncingId.value = null;
  }
};

const handleDelete = async (source: MarketSource) => {
  try {
    await ElMessageBox.confirm(`确定取消订阅「${source.name}」吗？`, '删除订阅', {
      type: 'warning',
      confirmButtonText: '确定',
      confirmButtonClass: 'el-button--danger'
    });
    await shareApi.deleteMarketSource(source.id);
    ElMessage.success("已移除订阅");
    await fetchSources();
    emit('update-scripts');
  } catch (error) {
    if (error === 'cancel' || error === 'close') return;
    ElMessage.error(error instanceof Error ? error.message : "删除失败");
  }
};

watch(visible, (val) => {
  if (val) void fetchSources();
});
</script>
