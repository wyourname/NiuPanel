<template>
  <div class="flex flex-col gap-3">
    <section class="border-b border-light/80 pb-3">
      <div class="mb-2 text-xs font-semibold text-secondary">
        起点函数
      </div>
      <el-select
        :model-value="functionName"
        @update:model-value="$emit('update:functionName', $event)"
        filterable
        allow-create
        default-first-option
        placeholder="起点函数"
        size="default"
        class="w-full"
      >
        <el-option label="main" value="main" />
        <el-option label="run" value="run" />
        <el-option label="start" value="start" />
      </el-select>
    </section>

    <section class="border-b border-light/80 pb-3">
      <div class="mb-2 text-xs font-semibold text-secondary">
        编译选项
      </div>
      <div
        class="flex items-center justify-between rounded-lg border border-light bg-card px-3 py-2 transition-colors hover:bg-soft"
      >
        <span class="text-xs font-bold text-default">代码混淆</span>
        <el-switch
          :model-value="obfuscate"
          @update:model-value="$emit('update:obfuscate', $event)"
          size="small"
        />
      </div>
    </section>

    <section class="border-b border-light/80 pb-3">
      <div class="mb-2 text-xs font-semibold text-secondary">
        目标环境
      </div>
      <div class="grid grid-cols-1 gap-1.5" v-loading="loadingVersions">
        <div
          v-for="v in availableVersions"
          :key="v"
          class="flex cursor-pointer items-center justify-between rounded-lg border border-light bg-card px-3 py-2 transition-colors hover:bg-soft"
          @click="$emit('toggleVersion', v)"
        >
          <span class="font-mono text-xs font-bold text-default">Python {{ v }}</span>
          <el-checkbox
            :model-value="targetVersions.includes(v)"
            size="small"
            @click.stop="$emit('toggleVersion', v)"
          />
        </div>
        <div
          v-if="availableVersions.length === 0 && !loadingVersions"
          class="py-4 text-center text-[11px] font-bold text-muted"
        >
          无可用环境
        </div>
      </div>
    </section>

    <el-button
      type="primary"
      :loading="submitting"
      class="sticky bottom-0 z-10 w-full !h-10 !rounded-lg"
      @click="$emit('compile')"
    >
      <div class="i-ep-cpu mr-2"></div>
      立即编译并加密
    </el-button>

    <section
      v-if="resultFile"
      class="border-t border-light/80 pt-3"
    >
      <div class="mb-2 text-xs font-semibold text-secondary">
        生成文件
      </div>
      <div class="flex flex-col gap-2">
        <div class="flex min-w-0 items-center gap-2 rounded-lg border border-light bg-card px-3 py-2">
          <div class="i-ep-document shrink-0 text-lg text-primary"></div>
          <span class="min-w-0 truncate font-mono text-xs font-bold text-default">
            {{ resultFile }}
          </span>
        </div>
        <el-button
          type="primary"
          class="w-full !h-9 !rounded-lg"
          @click="$emit('download')"
        >
          <div class="i-ep-download mr-1"></div>
          下载结果文件
        </el-button>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  functionName: string;
  obfuscate: boolean;
  submitting: boolean;
  loadingVersions: boolean;
  availableVersions: string[];
  targetVersions: string[];
  resultFile: string;
}>();

defineEmits<{
  (e: "update:functionName", val: string): void;
  (e: "update:obfuscate", val: boolean): void;
  (e: "compile"): void;
  (e: "toggleVersion", val: string): void;
  (e: "download"): void;
}>();
</script>
