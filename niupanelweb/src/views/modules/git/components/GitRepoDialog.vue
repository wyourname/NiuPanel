<template>
  <ResponsiveDialog
    v-model:visible="visible"
    :title="isEdit ? '编辑仓库配置' : '添加 Git 仓库'"
    desktop-size="md"
    content-preset="form"
    append-to-body
  >
    <div class="flex flex-col gap-4">
      <el-form
        ref="formRef"
        :model="form"
        label-position="top"
        :rules="rules"
      >
        <el-form-item label="仓库名称" prop="name">
          <el-input v-model="form.name" placeholder="例如: 我的脚本库" />
        </el-form-item>
        <el-form-item label="仓库地址 (HTTPS)" prop="repo_url">
          <el-input
            v-model="form.repo_url"
            placeholder="https://github.com/user/repo.git"
          />
        </el-form-item>
        <div class="grid grid-cols-1 gap-3 sm:grid-cols-2 sm:gap-4">
          <el-form-item label="分支" prop="branch">
            <el-input v-model="form.branch" placeholder="main" />
          </el-form-item>
          <el-form-item label="访问令牌 (可选)">
            <el-input
              v-model="form.auth_token"
              type="password"
              show-password
              :placeholder="
                isEdit && form.has_auth_token
                  ? '已配置；留空将保持不变'
                  : '私有仓库填写'
              "
              :disabled="form.clear_auth_token"
            />
            <el-checkbox
              v-if="isEdit && form.has_auth_token"
              v-model="form.clear_auth_token"
              class="mt-1"
            >
              清除已保存的令牌
            </el-checkbox>
          </el-form-item>
        </div>
        <el-form-item label="网络代理 (可选)">
          <el-input
            v-model="form.proxy_url"
            placeholder="http://127.0.0.1:7890"
          />
        </el-form-item>
        <div
          class="flex items-center justify-between rounded-lg border border-light bg-subtle p-3"
        >
          <span class="text-sm font-bold">自动同步</span>
          <el-switch v-model="form.auto_sync" />
        </div>
      </el-form>
    </div>

    <template #footer>
      <el-button @click="visible = false">取消</el-button>
      <el-button type="primary" :loading="submitting" @click="submit">
        {{ isEdit ? "保存修改" : "立即添加" }}
      </el-button>
    </template>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { ref } from "vue";
import type { FormInstance, FormRules } from "element-plus";
import ResponsiveDialog from "@/components/common/ResponsiveDialog.vue";
import type { GitRepoForm } from "../composables/useGitRepoForm";

defineProps<{
  isEdit: boolean;
  rules: FormRules<GitRepoForm>;
  submitting: boolean;
}>();

const emit = defineEmits<{
  (e: "submit", form: FormInstance | null): void;
}>();

const visible = defineModel<boolean>("visible", { required: true });
const form = defineModel<GitRepoForm>("form", { required: true });
const formRef = ref<FormInstance | null>(null);

const submit = () => {
  emit("submit", formRef.value);
};
</script>
