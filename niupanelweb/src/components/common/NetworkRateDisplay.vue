<template>
  <div
    role="group"
    aria-label="网络速率"
    class="network-rate"
    :class="{ 'network-rate--compact': compact }"
    title="当前节点网络速率，每 3 秒更新"
  >
    <div class="network-rate__row" :title="`上传 ${uploadLabel}`">
      <span class="i-ep-top text-primary" aria-hidden="true"></span>
      <span class="sr-only">上传</span>
      <strong class="network-rate__value">{{ uploadLabel }}</strong>
    </div>
    <div class="network-rate__row" :title="`下载 ${downloadLabel}`">
      <span class="i-ep-bottom text-emerald-700 dark:text-emerald-400" aria-hidden="true"></span>
      <span class="sr-only">下载</span>
      <strong class="network-rate__value">{{ downloadLabel }}</strong>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { formatTransferRate } from "@/utils/format";

const props = defineProps<{
  compact?: boolean;
  upload?: number | null;
  download?: number | null;
}>();

const uploadLabel = computed(() => formatTransferRate(props.upload));
const downloadLabel = computed(() => formatTransferRate(props.download));
</script>

<style scoped>
.network-rate {
  display: inline-flex;
  width: max-content;
  max-width: 100%;
  flex-direction: column;
  gap: 2px;
  padding: 2px 0;
}

.network-rate__row {
  display: grid;
  grid-template-columns: 12px minmax(11ch, 1fr);
  align-items: center;
  column-gap: 4px;
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 16px;
}

.network-rate__value {
  color: var(--text-default);
  font-family: var(--font-mono);
  font-size: 12px;
  font-weight: 500;
  font-variant-numeric: tabular-nums;
  text-align: left;
  white-space: nowrap;
}

.network-rate--compact .network-rate__row {
  grid-template-columns: 10px minmax(10ch, 1fr);
  column-gap: 2px;
  font-size: 11px;
  line-height: 14px;
}
.network-rate--compact .network-rate__value { font-size: 11px; }
</style>
