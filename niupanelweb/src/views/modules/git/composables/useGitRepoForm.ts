import { reactive, ref } from "vue";
import { ElMessage, type FormInstance, type FormRules } from "element-plus";
import * as gitApi from "@/api/git";
import type { GitRepo, GitRepoRequest } from "@/api/git";

export type GitRepoForm = {
  name: string;
  repo_url: string;
  branch: string;
  auth_token: string;
  proxy_url: string;
  auto_sync: boolean;
};

type UseGitRepoFormOptions = {
  onSaved: () => void | Promise<void>;
};

const createDefaultForm = (): GitRepoForm => ({
  name: "",
  repo_url: "",
  branch: "main",
  auth_token: "",
  proxy_url: "",
  auto_sync: false,
});

const buildPayload = (form: GitRepoForm): GitRepoRequest => ({
  name: form.name,
  repo_url: form.repo_url,
  branch: form.branch,
  auth_token: form.auth_token,
  proxy_url: form.proxy_url,
  auto_sync: form.auto_sync,
});

export function useGitRepoForm({ onSaved }: UseGitRepoFormOptions) {
  const dialogVisible = ref(false);
  const isEdit = ref(false);
  const editingId = ref<number | null>(null);
  const submitting = ref(false);
  const formRef = ref<FormInstance | null>(null);
  const form = reactive<GitRepoForm>(createDefaultForm());

  const rules: FormRules<GitRepoForm> = {
    name: [{ required: true, message: "请输入名称", trigger: "blur" }],
    repo_url: [{ required: true, message: "请输入地址", trigger: "blur" }],
    branch: [{ required: true, message: "请输入分支", trigger: "blur" }],
  };

  const resetForm = () => {
    Object.assign(form, createDefaultForm());
  };

  const openCreate = () => {
    isEdit.value = false;
    editingId.value = null;
    resetForm();
    dialogVisible.value = true;
  };

  const handleEdit = (row: GitRepo) => {
    isEdit.value = true;
    editingId.value = row.id;
    Object.assign(form, {
      name: row.name,
      repo_url: row.repo_url,
      branch: row.branch,
      auth_token: row.auth_token || "",
      proxy_url: row.proxy_url || "",
      auto_sync: row.auto_sync,
    });
    dialogVisible.value = true;
  };

  const handleSubmit = async (formInstance?: FormInstance | null) => {
    const targetForm = formInstance ?? formRef.value;
    if (!targetForm) return;

    await targetForm.validate(async (valid: boolean) => {
      if (!valid) return;

      submitting.value = true;
      try {
        const payload = buildPayload(form);
        if (isEdit.value && editingId.value) {
          await gitApi.updateGitRepo(editingId.value, payload);
          ElMessage.success("更新成功");
        } else {
          await gitApi.createGitRepo(payload);
          ElMessage.success("添加成功");
        }
        dialogVisible.value = false;
        await onSaved();
      } catch {
      } finally {
        submitting.value = false;
      }
    });
  };

  return {
    dialogVisible,
    form,
    formRef,
    handleEdit,
    handleSubmit,
    isEdit,
    openCreate,
    rules,
    submitting,
  };
}
