<template>
  <ResponsiveDialog
    v-model:visible="visible"
    title="编辑资源信息"
    desktop-size="md"
    content-preset="form"
    mobile-mode="fullscreen"
    destroy-on-close
    append-to-body
  >
    <div class="flex flex-col h-full">
      <el-form
        :model="form"
        label-position="top"
        class="flex-1"
      >
        <el-form-item label="备注名称">
          <el-input v-model="form.note" placeholder="设置一个易记的备注" />
        </el-form-item>

        <el-form-item label="下载次数限制">
          <el-input-number
            v-model="form.downloadsRemaining"
            :min="-1"
            class="!w-full"
          />
          <div class="text-[10px] text-muted mt-1">-1 表示无限制</div>
        </el-form-item>

        <el-form-item label="过期时间">
          <el-date-picker
            v-model="form.expiresAt"
            type="datetime"
            placeholder="选择过期时间"
            class="!w-full"
            value-format="x"
          />
          <div class="text-[10px] text-muted mt-1">留空表示永不过期</div>
        </el-form-item>

        <el-form-item label="下载密码">
          <el-input
            v-model="form.password"
            placeholder="设置下载密码（可选）"
            show-password
          />
        </el-form-item>

        <el-form-item>
          <el-checkbox v-model="form.deleteOnDownload">阅后即焚</el-checkbox>
        </el-form-item>
      </el-form>

    </div>

    <template #footer>
      <el-button @click="visible = false">取消</el-button>
      <el-button type="primary" :loading="submitting" @click="handleSubmit">
        保存修改
      </el-button>
    </template>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import { ElMessage } from "element-plus";
import ResponsiveDialog from "../../../../components/common/ResponsiveDialog.vue";
import * as shareApi from "../../../../api/share";
import type { StationFile } from "@/types";

type ShareEditForm = {
  note: string;
  downloadsRemaining: number;
  expiresAt: number | null;
  password: string;
  deleteOnDownload: boolean;
};

const props = defineProps<{
  modelValue: boolean;
  share?: StationFile;
}>();

const emit = defineEmits<{
  (event: "success"): void;
  (event: "update:modelValue", visible: boolean): void;
}>();

const visible = ref(false);
const submitting = ref(false);
const form = ref<ShareEditForm>({
  note: "",
  downloadsRemaining: -1,
  expiresAt: null,
  password: "",
  deleteOnDownload: false,
});

watch(
  () => props.modelValue,
  (val) => {
    visible.value = val;
    if (val && props.share) {
      form.value = {
        note: props.share.note || "",
        downloadsRemaining:
          props.share.downloadsRemaining ??
          props.share.downloads_remaining ??
          -1,
        expiresAt: getExpiresAt(props.share)
          ? getExpiresAt(props.share)! * 1000
          : null,
        password: props.share.password || "",
        deleteOnDownload: !!(
          props.share.deleteOnDownload ?? props.share.delete_on_download
        ),
      };
    }
  },
);

watch(visible, (val) => emit("update:modelValue", val));

const handleSubmit = async () => {
  if (!props.share?.token) return;
  submitting.value = true;
  try {
    await shareApi.updateStationFile(props.share.token, {
      note: form.value.note,
      downloadsRemaining: form.value.downloadsRemaining,
      expiresAt: form.value.expiresAt
        ? Math.floor(form.value.expiresAt / 1000)
        : null,
      password: form.value.password,
      deleteOnDownload: form.value.deleteOnDownload,
    });
    ElMessage.success("已保存");
    emit("success");
    visible.value = false;
  } catch (error) {
    ElMessage.error(error instanceof Error ? error.message : "保存失败");
  } finally {
    submitting.value = false;
  }
};

const getExpiresAt = (share: StationFile) =>
  share.expiresAt ?? share.expires_at ?? null;
</script>
