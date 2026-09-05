<template>
  <div class="flex flex-col gap-4 sm:gap-6">
    <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
      <section class="rounded-md border border-light bg-soft/50 p-3 sm:p-4">
        <div class="label-xs mb-3 flex items-center gap-2 sm:mb-4">
          <div class="i-ep-cpu text-primary"></div>
          运行环境
        </div>
        <div class="space-y-4">
          <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
            <el-select v-model="form.env_type" class="w-full modern-input">
              <el-option label="Python" value="python" />
              <el-option label="Node.js" value="node" />
              <el-option label="Shell" value="sh" />
            </el-select>
            <el-select
              v-if="form.env_type === 'python'"
              v-model="form.env_version"
              class="w-full modern-input"
              placeholder="默认版本"
            >
              <el-option
                v-for="version in pythonVersions"
                :key="version"
                :label="version"
                :value="version"
              />
            </el-select>
            <el-select
              v-else-if="form.env_type === 'node'"
              v-model="form.env_version"
              class="w-full modern-input"
              :placeholder="nodeVersions.length === 0 ? '无 Node 环境' : '选择环境'"
              :disabled="nodeVersions.length === 0"
            >
              <el-option
                v-for="version in nodeVersions"
                :key="version"
                :label="version"
                :value="version"
              />
            </el-select>
          </div>
          <el-input
            v-model="form.requirements"
            type="textarea"
            :rows="3"
            :placeholder="
              form.env_type === 'node'
                ? '依赖 (每行一个，如: axios)'
                : '依赖包 (每行一个)'
            "
            class="modern-input !text-xs"
          />
        </div>
      </section>

      <section class="rounded-md border border-light bg-soft/50 p-3 sm:p-4">
        <div class="label-xs mb-3 flex items-center gap-2 sm:mb-4">
          <div class="i-ep-odometer text-primary"></div>
          资源限制
        </div>
        <div class="grid grid-cols-3 gap-2 sm:gap-3">
          <div>
            <span class="text-[10px] text-muted mb-1.5 block">CPU (%)</span>
            <el-input-number
              v-model="form.cpu_limit"
              :min="0"
              :max="100"
              class="!w-full"
              controls-position="right"
            />
          </div>
          <div>
            <span class="text-[10px] text-muted mb-1.5 block">超时 (秒)</span>
            <el-input-number
              v-model="form.timeout_sec"
              :min="0"
              class="!w-full"
              controls-position="right"
            />
          </div>
          <div>
            <span class="text-[10px] text-muted mb-1.5 block">内存 (MB)</span>
            <el-input-number
              v-model="form.memory_limit"
              :min="0"
              class="!w-full"
              controls-position="right"
            />
          </div>
        </div>
        <div class="mt-3 flex items-center gap-1 text-[10px] italic text-muted opacity-60 sm:mt-4">
          <div class="i-ep-warning"></div>
          设置为 0 表示不应用资源配额限制
        </div>
      </section>
    </div>

    <section class="flex flex-1 flex-col rounded-md border border-light bg-soft/50 p-3 sm:p-4">
      <div class="mb-3 flex flex-col gap-3 sm:mb-4 sm:flex-row sm:items-center sm:justify-between">
        <div class="min-w-0">
          <div class="label-xs flex items-center gap-2">
            <div class="i-ep-set-up text-primary"></div>
            环境变量
          </div>
          <p class="mt-1 text-[10px] leading-4 text-muted">每行一个变量，任务运行时会注入对应的 Key 和 Value。</p>
        </div>
        <div class="grid grid-cols-2 rounded-md border border-light bg-base p-0.5 sm:shrink-0" role="group" aria-label="变量输入方式">
          <button
            type="button"
            class="min-h-8 rounded px-3 py-1.5 text-[11px] font-bold transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/30"
            :class="
              variableMode === 'bulk'
                ? 'bg-card text-primary shadow-sm'
                : 'text-muted hover:text-default'
            "
            :aria-pressed="variableMode === 'bulk'"
            @click="emit('update:variableMode', 'bulk')"
          >
            批量
          </button>
          <button
            type="button"
            class="min-h-8 rounded px-3 py-1.5 text-[11px] font-bold transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/30"
            :class="
              variableMode === 'list'
                ? 'bg-card text-primary shadow-sm'
                : 'text-muted hover:text-default'
            "
            :aria-pressed="variableMode === 'list'"
            @click="emit('update:variableMode', 'list')"
          >
            列表
          </button>
        </div>
      </div>
      <div v-if="variableMode === 'bulk'" class="flex-1">
        <el-input
          :model-value="variablesBulk"
          type="textarea"
          :rows="9"
          placeholder="KEY=VALUE\nAPI_TOKEN=..."
          class="modern-input font-mono !text-xs"
          @update:model-value="emit('update:variablesBulk', String($event))"
        />
      </div>
      <div v-else class="space-y-3">
        <article
          v-for="(variable, index) in variablesList"
          :key="index"
          class="rounded-md border border-light bg-card p-3 sm:p-3.5"
        >
          <header class="mb-3 flex items-center justify-between gap-3">
            <span class="font-mono text-[10px] font-semibold text-muted">变量 {{ index + 1 }}</span>
            <button
              type="button"
              class="flex h-8 w-8 shrink-0 cursor-pointer items-center justify-center rounded-md text-muted transition-colors hover:bg-rose-500/10 hover:text-rose-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-rose-500/30"
              title="删除变量"
              aria-label="删除变量"
              @click="variablesList.splice(index, 1)"
            >
              <div class="i-ep-delete text-base"></div>
            </button>
          </header>
          <div class="grid gap-3 sm:grid-cols-[minmax(0,0.9fr)_minmax(0,1.6fr)]">
            <label class="block min-w-0">
              <span class="mb-1.5 block text-[10px] font-semibold text-muted">变量名</span>
              <el-input
                v-model="variable.key"
                placeholder="例如 API_TOKEN"
                class="w-full modern-input font-mono !text-xs"
              />
            </label>
            <label class="block min-w-0">
              <span class="mb-1.5 block text-[10px] font-semibold text-muted">变量值</span>
              <el-input
                v-model="variable.value"
                type="textarea"
                :autosize="{ minRows: 2, maxRows: 5 }"
                placeholder="输入变量值"
                class="w-full modern-input font-mono !text-xs"
              />
            </label>
          </div>
        </article>
        <button
          type="button"
          class="flex min-h-10 w-full cursor-pointer items-center justify-center rounded-md border border-dashed border-light px-3 py-2 text-xs font-bold text-muted transition-colors hover:border-primary/30 hover:bg-hover hover:text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/30"
          @click="variablesList.push({ key: '', value: '' })"
        >
          <span class="i-ep-plus mr-1.5 text-sm"></span>添加环境变量
        </button>
        <p class="text-center text-[10px] text-muted">已添加 {{ variablesList.length }} 项</p>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import type {
  TaskVariableItem,
  TaskWizardForm,
} from "../../composables/taskWizardHelpers";

type TaskWizardVariableMode = "bulk" | "list";

defineProps<{
  form: TaskWizardForm;
  nodeVersions: string[];
  pythonVersions: string[];
  variableMode: TaskWizardVariableMode;
  variablesBulk: string;
  variablesList: TaskVariableItem[];
}>();

const emit = defineEmits<{
  (event: "update:variableMode", value: TaskWizardVariableMode): void;
  (event: "update:variablesBulk", value: string): void;
}>();
</script>
