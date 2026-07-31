<template>
  <div class="h-full min-h-0">
    <div v-if="!appStore.isMobile" class="flex h-full min-h-0 flex-col bg-card">
      <BulkActionBar
        :count="selectedIds.length"
        :show-select-all="true"
        :is-all-selected="selectedIds.length === variables.length && variables.length > 0"
        @cancel="clearSelection"
        @delete="handleBulkDelete"
        @select-all="handleSelectAll"
      >
        <template #actions>
          <div class="flex items-center gap-2">
            <el-button type="success" plain size="small" @click="handleBulkToggle(true)">批量启用</el-button>
            <el-button type="warning" plain size="small" @click="handleBulkToggle(false)">批量禁用</el-button>
          </div>
        </template>
      </BulkActionBar>

      <VariableDesktopTable
        class="min-h-0 flex-1"
        :active-tab="activeTab"
        :search-query="searchQuery"
        :variables="variables"
        :selected-ids="selectedIds"
        :loading="loading"
        :has-more="hasMore"
        :is-value-loading="isValueLoading"
        :is-value-visible="isValueVisible"
        @select-scope="selectVariableScope"
        @search-input="handleVariableSearchInput"
        @create="handleCreate"
        @import="handleImport"
        @export="handleExport"
        @load-more="loadMore"
        @selection-change="(row, selected) => updateSelection(row, Boolean(selected))"
        @toggle-value="toggleValueVisibility"
        @copy-value="copyValue"
        @status-change="handleCardStatusChange"
        @edit="handleEdit"
        @delete="handleDelete"
      />
    </div>

    <PageShell v-else compact :padded="false">
      <div class="flex-1 min-h-0 overflow-hidden flex flex-col bg-card">
        <VariablePageToolbar
          v-model:active-tab="activeTab"
          v-model:search-query="searchQuery"
          :initial-loading="loading && currentPage === 1"
          :is-mobile="appStore.isMobile"
          @create="handleCreate"
          @export-json="handleExport"
          @import-json="handleImport"
          @search="handleSearch"
          @tab-change="handleTabChange"
        />

        <div class="relative flex-1 min-h-0 flex flex-col bg-base">
          <BulkActionBar
            :count="selectedIds.length"
            :show-select-all="true"
            :is-all-selected="selectedIds.length === variables.length && variables.length > 0"
            @cancel="clearSelection"
            @delete="handleBulkDelete"
            @select-all="handleSelectAll"
          >
            <template #actions>
              <div class="flex items-center gap-2">
                <el-button type="success" plain size="small" @click="handleBulkToggle(true)">
                  批量启用
                </el-button>
                <el-button type="warning" plain size="small" @click="handleBulkToggle(false)">
                  批量禁用
                </el-button>
              </div>
            </template>
          </BulkActionBar>

          <VariableListPanel
            :drag-over-index="dragOverIndex"
            :get-task-names="getTaskNames"
            :has-more="hasMore"
            :is-mobile="appStore.isMobile"
            :is-value-loading="isValueLoading"
            :is-value-visible="isValueVisible"
            :loading="loading"
            :search-query="searchQuery"
            :selected-ids="selectedIds"
            :touch-drag-index="touchDragIndex"
            :variables="variables"
            @card-click="handleVariableCardClick"
            @copy-value="copyValue"
            @delete="handleDelete"
            @drag-end="handleDragEnd"
            @drag-over="handleDragOver"
            @drag-start="handleDragStart"
            @drop="handleDrop"
            @edit="handleEdit"
            @load-more="loadMore"
            @selection-change="toggleMobileSelection"
            @status-change="handleCardStatusChange"
            @toggle-value="toggleValueVisibility"
            @touch-end="handleTouchEnd"
            @touch-move="handleTouchMove"
            @touch-start="handleTouchStart"
          />
        </div>
      </div>
    </PageShell>

    <FloatingActionButton
      v-if="appStore.isMobile && selectedIds.length === 0"
      @click="handleCreate"
    />

    <VariableFormDialog
      v-model:visible="dialogVisible"
      :active-tab="activeTab"
      :editing-id="editingId"
      :form="form"
      :is-mobile="appStore.isMobile"
      :rules="rules"
      :submitting="submitting"
      :tasks="tasks"
      @submit="submitForm"
    />
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useAppStore } from "../../stores/app";
import { useHaptics } from "../../composables/useHaptics";
import { useVariableForm } from "../../composables/useVariableForm";
import { useVariableImportExport } from "../../composables/useVariableImportExport";
import { useVariableMutations } from "../../composables/useVariableMutations";
import {
  useVariablePageData,
  type VariablePageRow,
} from "../../composables/useVariablePageData";
import { useVariablePageSelection } from "../../composables/useVariablePageSelection";
import { useVariableReorder } from "../../composables/useVariableReorder";
import { useVariableValueVisibility } from "../../composables/useVariableValueVisibility";
import * as variableApi from "../../api/variable";
import BulkActionBar from "../../components/common/BulkActionBar.vue";
import FloatingActionButton from "../../components/common/FloatingActionButton.vue";
import PageShell from "../../components/common/PageShell.vue";
import VariableDesktopTable from "../../components/variables/VariableDesktopTable.vue";
import VariableFormDialog from "../../components/variables/VariableFormDialog.vue";
import VariableListPanel from "../../components/variables/VariableListPanel.vue";
import VariablePageToolbar from "../../components/variables/VariablePageToolbar.vue";

const appStore = useAppStore();
const haptics = useHaptics();
const route = useRoute();
const router = useRouter();

const variables = ref<VariablePageRow[]>([]);
const valueCache = new Map<number, string>();
let resetValueVisibility = () => {};

const invalidateSensitiveValues = () => {
  valueCache.clear();
  resetValueVisibility();
};

const resolveVariableValue = async (id: number) => {
  const cached = valueCache.get(id);
  if (cached !== undefined) return cached;

  const response = await variableApi.getVariableValue(id);
  const value = response.data.value;
  valueCache.set(id, value);
  const row = variables.value.find((item) => item.id === id);
  if (row) row.value = value;
  return value;
};

const {
  clearSelection,
  handleCardClick,
  handleSelectAll,
  selectedIds,
  toggleMobileSelection,
  updateSelection,
} = useVariablePageSelection({
  haptics,
  variables,
});

const {
  activeTab,
  currentPage,
  getScopedTaskId,
  getTaskNames,
  handleSearch,
  handleTabChange,
  hasMore,
  loadData,
  loading,
  loadMore,
  searchQuery,
  tasks,
} = useVariablePageData({
  clearSelection,
  haptics,
  onReset: invalidateSensitiveValues,
  variables,
});

const valueVisibility = useVariableValueVisibility({
  haptics,
  resolveValue: resolveVariableValue,
});
const {
  copyValue,
  isValueLoading,
  isValueVisible,
  toggleValueVisibility,
} = valueVisibility;
resetValueVisibility = valueVisibility.resetValueVisibility;

const reloadWithSecretsInvalidated = () => {
  invalidateSensitiveValues();
  return loadData();
};

const { handleExport, handleImport } = useVariableImportExport({
  activeTab,
  getScopedTaskId,
  reload: reloadWithSecretsInvalidated,
});

const {
  dialogVisible,
  editingId,
  form,
  handleCreate,
  handleEdit,
  rules,
  submitForm,
  submitting,
} = useVariableForm({
  activeTab,
  haptics,
  reload: reloadWithSecretsInvalidated,
});

const {
  handleBulkDelete,
  handleBulkToggle,
  handleDelete,
  handleStatusChange,
} = useVariableMutations({
  getScopedTaskId,
  haptics,
  reload: reloadWithSecretsInvalidated,
  selectedIds,
  variables,
});

const handleCardStatusChange = (row: VariablePageRow, value: unknown) => {
  row.enabled = !!value;
  handleStatusChange(row, value);
};

const handleVariableCardClick = (row: VariablePageRow) => {
  if (selectedIds.value.length > 0) {
    handleCardClick(row);
    return;
  }
  handleEdit(row);
};

const handleVariableSearchInput = (value: string | number) => {
  searchQuery.value = String(value);
  handleSearch();
};

const selectVariableScope = async (scope: "Script" | "Global") => {
  activeTab.value = scope;

  const query = { ...route.query };
  delete query.scope_id;
  await router.replace({ query });
  handleTabChange();
};

const {
  dragOverIndex,
  handleDragEnd,
  handleDragOver,
  handleDragStart,
  handleDrop,
  handleTouchEnd,
  handleTouchMove,
  handleTouchStart,
  touchDragIndex,
} = useVariableReorder({
  activeTab,
  getScopedTaskId,
  hasMore,
  haptics,
  searchQuery,
  variables,
});
</script>
