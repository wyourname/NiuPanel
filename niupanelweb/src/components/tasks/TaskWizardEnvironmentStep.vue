<template>
  <div class="flex flex-col gap-6">
    <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
      <section class="rounded-md border border-light bg-soft/50 p-4">
        <div class="label-xs mb-4 flex items-center gap-2">
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

      <section class="rounded-md border border-light bg-soft/50 p-4">
        <div class="label-xs mb-4 flex items-center gap-2">
          <div class="i-ep-odometer text-primary"></div>
          资源限制
        </div>
        <div class="grid grid-cols-1 gap-3 sm:grid-cols-3">
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
        <div class="text-[10px] text-muted mt-4 opacity-60 italic flex items-center gap-1">
          <div class="i-ep-warning"></div>
          设置为 0 表示不应用资源配额限制
        </div>
      </section>
    </div>

    <section class="flex flex-1 flex-col rounded-md border border-light bg-soft/50 p-4">
      <div class="flex items-center justify-between mb-4">
        <div class="label-xs flex items-center gap-2">
          <div class="i-ep-set-up text-primary"></div>
          环境变量
        </div>
        <div class="flex rounded-md border border-light bg-base p-0.5">
          <button
            type="button"
            class="rounded px-4 py-1.5 text-[10px] font-bold transition-colors"
            :class="
              variableMode === 'bulk'
                ? 'bg-card text-primary shadow-sm'
                : 'text-muted hover:text-default'
            "
            @click="emit('update:variableMode', 'bulk')"
          >
            批量
          </button>
          <button
            type="button"
            class="rounded px-4 py-1.5 text-[10px] font-bold transition-colors"
            :class="
              variableMode === 'list'
                ? 'bg-card text-primary shadow-sm'
                : 'text-muted hover:text-default'
            "
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
          :rows="8"
          placeholder="KEY=VALUE"
          class="modern-input font-mono !text-xs"
          @update:model-value="emit('update:variablesBulk', String($event))"
        />
      </div>
      <div v-else class="space-y-3">
        <div
          v-for="(variable, index) in variablesList"
          :key="index"
          class="flex gap-3 items-start group"
        >
          <el-input
            v-model="variable.key"
            placeholder="Key"
            class="w-1/3 shrink-0 modern-input font-mono !text-xs"
          />
          <el-input
            v-model="variable.value"
            type="textarea"
            :autosize="{ minRows: 1, maxRows: 4 }"
            placeholder="Value"
            class="flex-1 modern-input font-mono !text-xs"
          />
          <button
            type="button"
            class="mt-0.5 shrink-0 rounded-md p-2 text-muted transition-colors hover:bg-rose-500/10 hover:text-rose-500"
            title="删除变量"
            aria-label="删除变量"
            @click="variablesList.splice(index, 1)"
          >
            <div class="i-ep-delete"></div>
          </button>
        </div>
        <button
          type="button"
          class="w-full rounded-md border border-dashed border-light py-2.5 text-xs font-bold text-muted transition-colors hover:border-primary/20 hover:bg-hover hover:text-primary"
          @click="variablesList.push({ key: '', value: '' })"
        >
          <span class="i-ep-plus mr-1"></span>添加环境变量
        </button>
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
