<template>
  <div class="flex flex-col gap-6">
    <el-form ref="formRef" :model="form" :rules="rules" label-position="top">
      <div class="grid grid-cols-1 md:grid-cols-2 gap-x-6 gap-y-2">
        <el-form-item prop="name" label="任务名称">
          <el-input v-model="form.name" placeholder="例如：每日签到脚本" class="modern-input" />
        </el-form-item>
        <el-form-item label="后续任务">
          <el-select
            v-model="form.trigger_next_tasks"
            placeholder="成功后自动触发后续任务 (可选)"
            class="w-full modern-input"
            multiple
            filterable
            collapse-tags
            collapse-tags-tooltip
          >
            <template #default>
              <el-option
                v-for="item in allTasks"
                :key="item.id"
                :label="item.name"
                :value="item.id"
                :disabled="item.id === initialTaskId"
              >
                <span class="float-left font-bold text-xs">{{ item.name }}</span>
                <span class="float-right text-muted text-[10px] font-mono">#{{ item.id }}</span>
              </el-option>
            </template>
          </el-select>
        </el-form-item>
      </div>

      <section class="mt-2 space-y-4 rounded-md border border-light bg-soft/50 p-4">
        <!-- Header: Strategy Selection & Status -->
        <div class="flex items-center justify-between gap-4">
          <div class="flex flex-wrap items-center gap-4">
            <el-radio v-model="form.enableRandom" :label="false" class="!mr-0">
              <span class="text-xs font-bold text-default">常规定时</span>
            </el-radio>
            <el-radio v-model="form.enableRandom" :label="true" class="!mr-0">
              <span class="text-xs font-bold text-default">区间随机分发执行</span>
            </el-radio>
          </div>

          <div
            v-if="!form.enableRandom && form.cron_schedule"
            class="truncate rounded border border-light bg-base px-2 py-0.5 text-[10px] font-medium text-muted"
            :class="!cronValid ? '!text-rose-500 !border-rose-500/20' : ''"
          >
            {{ cronDescription }}
          </div>
        </div>

        <!-- Content Area -->
        <div class="pt-1">
          <transition name="el-fade-in" mode="out-in">
            <!-- Strategy: Fixed -->
            <div v-if="!form.enableRandom" key="cron" class="animate-in fade-in slide-in-from-top-1 duration-200">
              <CronInput
                v-model="form.cron_schedule"
                @update:description="emit('update:cronDescription', $event)"
                @update:valid="emit('update:cronValid', $event)"
              />
            </div>

            <!-- Strategy: Random -->
            <div v-else key="random" class="space-y-4 animate-in fade-in slide-in-from-top-1 duration-200">
              <div class="grid grid-cols-1 gap-3 sm:grid-cols-3">
                <div>
                  <el-time-picker
                    v-model="form.random_config.start"
                    format="HH:mm"
                    value-format="HH:mm"
                    placeholder="起始时间"
                    class="!w-full modern-input"
                  />
                </div>
                <div>
                  <el-time-picker
                    v-model="form.random_config.end"
                    format="HH:mm"
                    value-format="HH:mm"
                    placeholder="截止时间"
                    class="!w-full modern-input"
                  />
                </div>
                <div>
                  <el-input-number
                    v-model="form.random_config.count"
                    :min="1"
                    :max="100"
                    placeholder="运行次数"
                    class="!w-full"
                    controls-position="right"
                  />
                </div>
              </div>
            </div>
          </transition>
        </div>
      </section>

      <div class="grid grid-cols-1 gap-6 mt-6">
        <el-form-item label="任务描述">
          <el-input
            v-model="form.description"
            type="textarea"
            :rows="2"
            placeholder="补充任务用途或运维说明（可选）"
            class="modern-input"
          />
        </el-form-item>

        <div
          class="flex items-center justify-between rounded-md border border-light bg-soft/50 p-4"
        >
          <div class="flex items-center gap-3">
            <div class="h-9 w-9 rounded-md bg-orange-400/10 text-orange-500 flex-center">
              <div class="i-ep-bell"></div>
            </div>
            <div class="flex flex-col">
              <span class="text-sm font-bold text-default">执行结束通知</span>
              <span class="text-[10px] font-medium text-muted">任务完成或失败时通过系统通知渠道推送</span>
            </div>
          </div>
          <el-switch v-model="form.notify" />
        </div>
      </div>
    </el-form>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import type { FormInstance } from "element-plus";
import type { TaskWizardForm } from "../../composables/taskWizardHelpers";
import CronInput from "../common/CronInput.vue";

type TaskWizardSimpleTask = {
  id: number;
  name: string;
};

defineProps<{
  allTasks: TaskWizardSimpleTask[];
  cronDescription: string;
  cronValid: boolean;
  form: TaskWizardForm;
  initialTaskId?: number;
}>();

const emit = defineEmits<{
  (event: "update:cronDescription", value: string): void;
  (event: "update:cronValid", value: boolean): void;
}>();

const formRef = ref<FormInstance | null>(null);

const rules = {
  name: [{ required: true, message: "必填", trigger: "blur" }],
};

const validate = () =>
  new Promise<boolean>((resolve) => {
    if (!formRef.value) {
      resolve(false);
      return;
    }
    formRef.value.validate((valid) => resolve(valid));
  });

defineExpose({ validate });
</script>
