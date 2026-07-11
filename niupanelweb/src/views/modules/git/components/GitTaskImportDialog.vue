<template>
  <ResponsiveDialog
    v-model:visible="visible"
    title="扫描并导入任务"
    width="700px"
    append-to-body
  >
    <div class="flex h-[72vh] max-h-[620px] flex-col p-3 sm:p-4">
      <div class="mb-3 flex items-center justify-between gap-3">
        <p class="text-xs text-muted">
          已发现 {{ tasks.length }} 个潜在任务。勾选你想要导入的任务。
        </p>
        <el-checkbox v-model="selectAll" @change="handleSelectAllChange">
          全选
        </el-checkbox>
      </div>

      <div
        class="min-h-0 flex-1 overflow-auto rounded-md border border-light bg-card"
        v-loading="scanning"
      >
        <div
          v-if="tasks.length === 0 && !scanning"
          class="flex flex-col items-center justify-center h-full text-muted gap-2"
        >
          <div class="i-ep-search text-4xl opacity-20"></div>
          <span class="text-xs">未发现任务脚本</span>
          <span class="text-[10px]">
            请确保脚本包含 `new Env('Name')` (JS) 或 `# @name Name` (Py/Sh)
          </span>
        </div>

        <el-table
          v-else
          ref="tableRef"
          :data="tasks"
          style="width: 100%"
          height="100%"
          @selection-change="handleSelectionChange"
        >
          <el-table-column type="selection" width="40" />
          <el-table-column
            label="任务名称"
            prop="name"
            min-width="150"
            show-overflow-tooltip
          >
            <template #default="{ row }">
              <span class="font-bold text-xs">{{ row.name }}</span>
            </template>
          </el-table-column>
          <el-table-column label="定时规则" prop="cron" width="120">
            <template #default="{ row }">
              <span class="font-mono text-[10px] bg-base px-1 rounded">
                {{ row.cron || "-" }}
              </span>
            </template>
          </el-table-column>
          <el-table-column
            label="脚本路径"
            prop="file_path"
            min-width="200"
            show-overflow-tooltip
          >
            <template #default="{ row }">
              <span class="font-mono text-[10px] text-muted">
                {{ row.file_path }}
              </span>
            </template>
          </el-table-column>
        </el-table>
      </div>

      <div class="flex justify-end gap-2 pt-4">
        <el-button @click="visible = false">取消</el-button>
        <el-button
          type="primary"
          :disabled="selectedCount === 0"
          :loading="importing"
          @click="$emit('import')"
        >
          导入选中任务 ({{ selectedCount }})
        </el-button>
      </div>
    </div>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import type { TableInstance } from "element-plus";
import type { DiscoveredTask } from "@/api/git";
import ResponsiveDialog from "@/components/common/ResponsiveDialog.vue";

const props = defineProps<{
  importing: boolean;
  scanning: boolean;
  selectedCount: number;
  tasks: DiscoveredTask[];
}>();

const emit = defineEmits<{
  (e: "import"): void;
  (e: "selectAllChange", value: boolean): void;
  (e: "selectionChange", tasks: DiscoveredTask[]): void;
}>();

const visible = defineModel<boolean>("visible", { required: true });
const selectAll = defineModel<boolean>("selectAll", { required: true });
const tableRef = ref<TableInstance | null>(null);

const syncTableSelection = async () => {
  await nextTick();
  if (!tableRef.value) return;

  tableRef.value.clearSelection();
  if (selectAll.value && props.tasks.length > 0) {
    props.tasks.forEach((task) => {
      tableRef.value?.toggleRowSelection(task, true);
    });
  }
};

const handleSelectAllChange = (value: boolean) => {
  emit("selectAllChange", value);
  void syncTableSelection();
};

const handleSelectionChange = (tasks: DiscoveredTask[]) => {
  emit("selectionChange", tasks);
};

watch(
  () => [visible.value, props.tasks.length, props.scanning, selectAll.value],
  () => {
    if (visible.value && !props.scanning) {
      void syncTableSelection();
    }
  },
  { flush: "post" },
);
</script>
