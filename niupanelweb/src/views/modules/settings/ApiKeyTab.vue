<template>
  <div class="w-full h-full flex flex-col space-y-4">
    <McpAccessPanel />

    <div class="flex min-h-8 shrink-0 items-center justify-between gap-3 px-1">
      <span class="text-[11px] font-bold text-muted">
        {{ keys.length }} 个密钥
      </span>
      <el-button
        type="primary"
        class="!h-8 !rounded-lg !px-3 !text-[12px] font-bold"
        @click="openCreate"
      >
        <div class="i-ep-plus mr-1 text-sm"></div>
        创建新密钥
      </el-button>
    </div>

    <ApiKeyList
      :is-mobile="appStore.isMobile"
      :keys="keys"
      :loading="loading"
      :on-refresh="loadKeys"
      @delete="handleDelete"
      @edit="handleEdit"
    />

    <ApiKeyPermissionDialog
      v-model:visible="showDialog"
      v-model:active-group="activeGroup"
      v-model:selected-perms="selectedPerms"
      :form="form"
      :is-edit="isEdit"
      :is-mobile="appStore.isMobile"
      :is-group-all-selected="isGroupAllSelected"
      :is-group-indeterminate="isGroupIndeterminate"
      :submitting="submitting"
      @group-select-all="handleGroupSelectAll"
      @submit="handleSubmit"
      @toggle-perm="togglePerm"
      @toggle-super-user="toggleSuperUser"
    />

    <ApiKeySuccessDialog
      v-model:visible="showSuccess"
      :token="newToken"
      @copy-token="copyToken"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, reactive } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import useClipboard from "vue-clipboard3";
import {
  listApiKeys,
  createApiKey,
  updateApiKey,
  deleteApiKey,
  type ApiKey,
} from "@/api/keys";
import { useAppStore } from "@/stores/app";
import { useHaptics } from "@/composables/useHaptics";
import ApiKeyList from "./components/ApiKeyList.vue";
import ApiKeyPermissionDialog from "./components/ApiKeyPermissionDialog.vue";
import ApiKeySuccessDialog from "./components/ApiKeySuccessDialog.vue";
import McpAccessPanel from "./components/McpAccessPanel.vue";
import {
  parsePerms,
  type ApiKeyFormState,
} from "./utils/apiKeyPermissions";
import { useApiKeyPermissions } from "./composables/useApiKeyPermissions";

const appStore = useAppStore();
const haptics = useHaptics();
const { toClipboard } = useClipboard();
const keys = ref<ApiKey[]>([]);
const loading = ref(false);
const showDialog = ref(false);
const submitting = ref(false);
const showSuccess = ref(false);
const newToken = ref("");
const isEdit = ref(false);
const editingId = ref<number | null>(null);
const {
  activeGroup,
  handleGroupSelectAll,
  isGroupAllSelected,
  isGroupIndeterminate,
  selectedPerms,
  togglePerm,
  toggleSuperUser,
} = useApiKeyPermissions({ haptics });

const form = reactive<ApiKeyFormState>({
  name: "",
  expires_in_days: 30,
  expires_at: "",
});

const loadKeys = async () => {
  loading.value = true;
  try {
    const res = await listApiKeys();
    keys.value = res.data;
  } finally {
    loading.value = false;
  }
};

const openCreate = () => {
  isEdit.value = false;
  editingId.value = null;
  form.name = "";
  form.expires_in_days = 30;
  selectedPerms.value = [
    "overview:read",
    "task:list",
    "task:read",
    "task:run",
  ];
  activeGroup.value = "all";
  showDialog.value = true;
};

const handleEdit = (row: ApiKey) => {
  isEdit.value = true;
  editingId.value = row.id;
  form.name = row.name;
  form.expires_at = row.expires_at ? String(row.expires_at) : "";
  selectedPerms.value = parsePerms(row.permissions);
  activeGroup.value = "all";
  showDialog.value = true;
};

const handleSubmit = async () => {
  if (!form.name) return ElMessage.warning("请输入密钥用途名称");
  submitting.value = true;
  try {
    const permissions = selectedPerms.value.join(",");
    if (isEdit.value && editingId.value) {
      await updateApiKey(editingId.value, {
        name: form.name,
        permissions,
        expires_at: form.expires_at ? parseInt(form.expires_at) : undefined,
      });
      ElMessage.success("密钥已更新");
      showDialog.value = false;
      loadKeys();
    } else {
      const res = await createApiKey({
        name: form.name,
        expires_in_days: form.expires_in_days,
        permissions,
      });
      newToken.value = res.data.token;
      showDialog.value = false;
      showSuccess.value = true;
      loadKeys();
    }
  } catch (e) {
  } finally {
    submitting.value = false;
  }
};

const handleDelete = async (row: ApiKey) => {
  try {
    await ElMessageBox.confirm(
      `确定要吊销 "${row.name}" 吗？此操作不可撤销且会立即断开所有 SDK 连接。`,
      "安全审计警告",
      {
        type: "error",
        confirmButtonText: "确定吊销",
        confirmButtonClass: "el-button--danger",
      },
    );
    await deleteApiKey(row.id);
    ElMessage.success("密钥已失效");
    loadKeys();
  } catch (e) {}
};

const copyToken = async () => {
  try {
    haptics.impact();
    await toClipboard(newToken.value);
    ElMessage.success("Token 已复制");
  } catch (e) {
    ElMessage.error("复制失败");
  }
};

onMounted(loadKeys);
</script>
