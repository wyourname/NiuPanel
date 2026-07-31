<template>
  <div class="cron-input-container w-full flex flex-col gap-2">
    <div class="flex items-center gap-2">
      <el-input
        v-model="internalValue"
        placeholder="* * * * * (分 时 日 月 周)"
        class="font-mono flex-1"
        @input="handleInput"
        clearable
      >
        <template #prefix>
          <div class="i-ep-timer text-muted"></div>
        </template>
      </el-input>

      <el-dropdown trigger="click" @command="applyPreset">
        <el-button plain>
          <div class="i-ep-magic-stick mr-1"></div>
          预设
        </el-button>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item
              v-for="p in presets"
              :key="p.value"
              :command="p.value"
            >
              <div class="flex flex-col">
                <span class="font-bold text-xs">{{ p.label }}</span>
                <code class="text-[10px] text-muted opacity-70">{{
                  p.value
                }}</code>
              </div>
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </div>

    <!-- Info/Preview Area -->
    <div
      v-if="internalValue?.trim()"
      class="flex flex-col gap-1.5 p-2.5 bg-base/50 rounded-lg border border-base/30"
    >
      <!-- 1. Translation Area -->
      <div class="flex items-start gap-2">
        <div
          class="shrink-0 mt-0.5"
          :class="
            parseResult.valid
              ? 'i-ep-info-filled text-primary'
              : 'i-ep-warning text-rose-500'
          "
        ></div>
        <div class="flex flex-col overflow-hidden justify-center">
          <span
            class="text-xs font-bold"
            :class="parseResult.valid ? 'text-default' : 'text-rose-500'"
          >
            {{ parseResult.valid ? "运行规则" : "表达式错误" }}
          </span>
        </div>
      </div>

      <!-- 2. Next Runs Area (Only if valid) -->
      <div
        v-if="parseResult.valid && parseResult.nextRuns.length > 0"
        class="flex flex-col gap-1 border-t border-base mt-1 pt-1.5"
      >
        <span class="text-[10px] font-semibold text-muted"
          >预期下次运行时间</span
        >
        <div class="flex flex-wrap gap-1">
          <div
            v-for="(time, idx) in parseResult.nextRuns"
            :key="idx"
            class="text-[10px] font-mono py-0.5 px-1.5 rounded bg-white dark:bg-gray-800 border border-base whitespace-nowrap"
          >
            {{ time }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed } from "vue";
import cronstrue from "cronstrue/i18n";
import { CronExpressionParser } from "cron-parser";

type CronInputProps = {
  modelValue?: string;
};

type CronParseResult = {
  valid: boolean;
  description: string;
  nextRuns: string[];
};

const props = withDefaults(defineProps<CronInputProps>(), {
  modelValue: "",
});

const emit = defineEmits<{
  (event: "update:modelValue", value: string): void;
  (event: "update:description", value: string): void;
  (event: "update:valid", value: boolean): void;
}>();

const internalValue = ref(props.modelValue);

const presets = [
  { label: "每分钟", value: "* * * * *" },
  { label: "每小时", value: "0 * * * *" },
  { label: "每天 0:00", value: "0 0 * * *" },
  { label: "每天 8:00", value: "0 8 * * *" },
  { label: "每隔 5 分钟", value: "*/5 * * * *" },
  { label: "工作日 9:00", value: "0 9 * * 1-5" },
] as const;

watch(
  () => props.modelValue,
  (newVal) => {
    internalValue.value = newVal;
  },
);

const handleInput = (val: string) => {
  emit("update:modelValue", val);
};

const applyPreset = (val: unknown) => {
  if (typeof val !== "string") return;
  internalValue.value = val;
  handleInput(val);
};

// 统一解析逻辑：一个 Computed 搞定所有，确保一致性
const parseResult = computed<CronParseResult>(() => {
  const val = internalValue.value?.trim();
  if (!val) return { valid: true, description: "手动执行", nextRuns: [] };

  let description = "";
  let nextRuns: string[] = [];
  let valid = false;

  // 1. 尝试使用 cronstrue 进行翻译（它通常比 parser 更宽容）
  try {
    description = cronstrue.toString(val, {
      locale: "zh_CN",
      use24HourTimeFormat: true,
    });
    valid = true; // 翻译成功，初步认为有效
  } catch {
    return {
      valid: false,
      description: "无效的 Cron 表达式格式",
      nextRuns: [],
    };
  }

  // 2. 尝试获取下次运行时间
  try {
    const interval = CronExpressionParser.parse(val);
    for (let i = 0; i < 3; i++) {
      const date = interval.next().toDate();
      nextRuns.push(
        date.toLocaleString("zh-CN", {
          month: "2-digit",
          day: "2-digit",
          hour: "2-digit",
          minute: "2-digit",
          hour12: false,
        }),
      );
    }
  } catch (error: unknown) {
    // 如果 parser 报错但 cronstrue 成功了，说明是 parser 的 picky 导致的
    // 这种情况我们依然标记为有效，只是不显示下次运行时间，或者显示 parser 的报错作为说明
    console.warn("Cron parser failed while translator succeeded:", error);
    // 如果翻译成功了，我们就倾向于认为它是有效的，只是 parser 可能不支持某些特殊语法
  }

  return { valid, description, nextRuns };
});

watch(
  parseResult,
  (res) => {
    emit("update:description", res.description);
    emit("update:valid", res.valid);
  },
  { immediate: true },
);
</script>

<style scoped>
.cron-input-container :deep(.el-input__wrapper) {
  box-shadow: none !important;
  border: 1px solid var(--el-border-color);
}
.cron-input-container :deep(.el-input__wrapper.is-focus) {
  border-color: var(--el-color-primary);
}
</style>
