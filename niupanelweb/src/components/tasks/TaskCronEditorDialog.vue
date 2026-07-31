<template>
  <ResponsiveDialog
    v-model:visible="visibleValue"
    title="定时规则"
    width="480px"
    append-to-body
    destroy-on-close
  >
    <div class="flex flex-col gap-5 p-5 md:p-6">
      <div class="space-y-4">
        <div class="flex items-center justify-between">
          <span class="label-sm">定时表达式</span>
        </div>
        <CronInput v-model="cronValue" />
      </div>

      <div class="h-px bg-light/50"></div>

      <div class="space-y-5">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <div class="i-ep-magic-stick text-primary text-lg"></div>
            <span class="label-sm font-bold">随机运行模式</span>
          </div>
          <el-switch v-model="enableRandomValue" />
        </div>

        <transition name="el-fade-in">
          <div
            v-if="enableRandom"
            class="space-y-4 rounded-md border border-primary/10 bg-base/20 p-4"
          >
            <div class="flex items-center gap-4">
              <div class="flex-1">
                <span class="mb-2 block text-[10px] font-bold text-muted">
                  开始时间
                </span>
                <el-time-picker
                  v-model="randomStart"
                  format="HH:mm"
                  value-format="HH:mm"
                  placeholder="开始时间"
                  class="!w-full"
                />
              </div>
              <div class="flex-1">
                <span class="mb-2 block text-[10px] font-bold text-muted">
                  结束时间
                </span>
                <el-time-picker
                  v-model="randomEnd"
                  format="HH:mm"
                  value-format="HH:mm"
                  placeholder="结束时间"
                  class="!w-full"
                />
              </div>
            </div>
            <div>
              <span class="mb-2 block text-[10px] font-bold text-muted">
                每日运行次数
              </span>
              <el-input-number
                v-model="randomCount"
                :min="1"
                :max="100"
                class="!w-full"
              />
            </div>
            <p class="text-[10px] text-muted italic">
              开启后，系统将在设定的时间段内随机选取 {{ randomConfig.count }} 个时刻自动运行任务。
            </p>
          </div>
        </transition>
      </div>

      <div class="flex justify-end gap-3 pt-4">
        <el-button @click="visibleValue = false">取消</el-button>
        <el-button
          type="primary"
          :loading="saving"
          class="!px-8 font-bold"
          @click="emit('save')"
        >
          应用更改
        </el-button>
      </div>
    </div>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { computed } from "vue";
import CronInput from "../common/CronInput.vue";
import ResponsiveDialog from "../common/ResponsiveDialog.vue";

type RandomConfig = {
  start: string;
  end: string;
  count: number;
};

const props = defineProps<{
  cron: string;
  enableRandom: boolean;
  randomConfig: RandomConfig;
  saving: boolean;
  visible: boolean;
}>();

const emit = defineEmits<{
  (event: "save"): void;
  (event: "update:cron", value: string): void;
  (event: "update:enableRandom", value: boolean): void;
  (event: "update:randomConfig", value: RandomConfig): void;
  (event: "update:visible", value: boolean): void;
}>();

const visibleValue = computed({
  get: () => props.visible,
  set: (value: boolean) => emit("update:visible", value),
});

const cronValue = computed({
  get: () => props.cron,
  set: (value: string) => emit("update:cron", value),
});

const enableRandomValue = computed({
  get: () => props.enableRandom,
  set: (value: boolean) => emit("update:enableRandom", value),
});

const updateRandomConfig = (patch: Partial<RandomConfig>) => {
  emit("update:randomConfig", {
    ...props.randomConfig,
    ...patch,
  });
};

const randomStart = computed({
  get: () => props.randomConfig.start,
  set: (value: string) => updateRandomConfig({ start: value }),
});

const randomEnd = computed({
  get: () => props.randomConfig.end,
  set: (value: string) => updateRandomConfig({ end: value }),
});

const randomCount = computed({
  get: () => props.randomConfig.count,
  set: (value: number | undefined) => updateRandomConfig({ count: value || 1 }),
});
</script>
