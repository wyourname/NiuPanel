<template>
  <ResponsiveDialog
    :visible="createVisible"
    :title="`创建${createType === 'file' ? '文件' : '目录'}`"
    width="440px"
    append-to-body
    @update:visible="emit('update:createVisible', $event)"
  >
    <div class="flex flex-col gap-5 p-5 md:p-6">
      <el-form
        ref="createFormRef"
        :model="createForm"
        :rules="createRules"
        @submit.prevent="submitCreate"
      >
        <el-form-item prop="name">
          <template #label>
            <span class="label-xs">项目名称</span>
          </template>
          <el-input
            v-model="createForm.name"
            placeholder="输入名称..."
            class="modern-input"
            clearable
            @keyup.enter="submitCreate"
          />
        </el-form-item>
      </el-form>
      <div class="flex gap-3 pt-2">
        <el-button class="flex-1 !h-9 font-bold" @click="emit('update:createVisible', false)">
          取消
        </el-button>
        <el-button
          type="primary"
          class="flex-1 !h-9 font-bold"
          :loading="creating"
          @click="submitCreate"
        >
          确认创建
        </el-button>
      </div>
    </div>
  </ResponsiveDialog>

  <ResponsiveDialog
    :visible="moveVisible"
    title="移动项目"
    width="440px"
    append-to-body
    @update:visible="emit('update:moveVisible', $event)"
  >
    <div class="flex flex-col gap-5 p-5 md:p-6">
      <el-form
        ref="moveFormRef"
        :model="moveForm"
        @submit.prevent="submitMove"
      >
        <div class="mb-4 flex max-h-32 flex-col gap-2 overflow-y-auto rounded-md border border-light bg-muted/10 p-3">
          <span class="text-xs font-bold mb-1">正在移动 {{ moveForm.items.length }} 个项目：</span>
          <span
            v-for="item in moveForm.items"
            :key="item.path"
            class="text-sm font-mono truncate text-primary"
          >
            {{ item.name }}
          </span>
        </div>
        <el-form-item prop="targetPath">
          <template #label>
            <span class="label-xs">
              目标目录相对路径 (留空表示移到根目录)
            </span>
          </template>
          <el-input
            v-model="moveForm.targetPath"
            placeholder="例如: folder/subfolder"
            class="modern-input"
            clearable
            @keyup.enter="submitMove"
          />
        </el-form-item>
      </el-form>
      <div class="flex gap-3 pt-2">
        <el-button class="flex-1 !h-9 font-bold" @click="emit('update:moveVisible', false)">
          取消
        </el-button>
        <el-button
          type="primary"
          class="flex-1 !h-9 font-bold"
          :loading="movingFile"
          @click="submitMove"
        >
          确认移动
        </el-button>
      </div>
    </div>
  </ResponsiveDialog>

  <ResponsiveDialog
    :visible="renameVisible"
    title="重命名项目"
    width="440px"
    append-to-body
    @update:visible="emit('update:renameVisible', $event)"
  >
    <div class="flex flex-col gap-5 p-5 md:p-6">
      <el-form
        ref="renameFormRef"
        :model="renameForm"
        :rules="renameRules"
        label-position="top"
        @submit.prevent="submitRename"
      >
        <el-form-item prop="newName">
          <template #label>
            <span class="label-xs">新名称</span>
          </template>
          <el-input
            v-model="renameForm.newName"
            placeholder="输入新名称..."
            class="modern-input"
            @keyup.enter="submitRename"
          />
        </el-form-item>
      </el-form>
      <div class="flex gap-3">
        <el-button class="flex-1 !h-9 font-bold" @click="emit('update:renameVisible', false)">
          取消
        </el-button>
        <el-button
          type="primary"
          class="flex-1 !h-9 font-bold"
          :loading="renaming"
          @click="submitRename"
        >
          应用更改
        </el-button>
      </div>
    </div>
  </ResponsiveDialog>

  <ResponsiveDialog
    :visible="downloadUrlVisible"
    title="远程下载资源"
    width="480px"
    append-to-body
    @update:visible="emit('update:downloadUrlVisible', $event)"
  >
    <div class="flex flex-col gap-5 p-5 md:p-6">
      <el-form
        ref="downloadUrlFormRef"
        :model="downloadUrlForm"
        :rules="downloadUrlRules"
        label-position="top"
        @submit.prevent="submitDownloadUrl"
      >
        <el-form-item prop="url">
          <template #label>
            <span class="label-xs">资源链接 URL</span>
          </template>
          <el-input
            v-model="downloadUrlForm.url"
            placeholder="https://..."
            class="modern-input"
            clearable
          />
        </el-form-item>
        <el-form-item prop="filename">
          <template #label>
            <span class="label-xs">保存名称（可选）</span>
          </template>
          <el-input
            v-model="downloadUrlForm.filename"
            placeholder="留空则自动检测"
            class="modern-input"
            clearable
          />
        </el-form-item>
      </el-form>

      <div class="flex gap-3">
        <el-button class="flex-1 !h-9 font-bold" @click="emit('update:downloadUrlVisible', false)">
          取消
        </el-button>
        <el-button
          type="primary"
          class="flex-1 !h-9 font-bold"
          :loading="downloadingUrl"
          @click="submitDownloadUrl"
        >
          开始下载
        </el-button>
      </div>
    </div>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { ref } from "vue";
import type { FormInstance } from "element-plus";
import ResponsiveDialog from "../../../../components/common/ResponsiveDialog.vue";
import type { FileItem } from "@/types";

const props = defineProps<{
  createVisible: boolean;
  createType: "file" | "directory";
  creating: boolean;
  createForm: { name: string };
  moveVisible: boolean;
  movingFile: boolean;
  moveForm: { targetPath: string; items: FileItem[] };
  renameVisible: boolean;
  renaming: boolean;
  renameForm: { oldPath: string; oldName: string; newName: string };
  downloadUrlVisible: boolean;
  downloadingUrl: boolean;
  downloadUrlForm: { url: string; filename: string };
  onCreateSubmit: (formRef: FormInstance | null) => void | Promise<void>;
  onMoveSubmit: (formRef: FormInstance | null) => void | Promise<void>;
  onRenameSubmit: (formRef: FormInstance | null) => void | Promise<void>;
  onDownloadUrlSubmit: (formRef: FormInstance | null) => void | Promise<void>;
}>();

const emit = defineEmits<{
  "update:createVisible": [value: boolean];
  "update:moveVisible": [value: boolean];
  "update:renameVisible": [value: boolean];
  "update:downloadUrlVisible": [value: boolean];
}>();

const createFormRef = ref<FormInstance | null>(null);
const moveFormRef = ref<FormInstance | null>(null);
const renameFormRef = ref<FormInstance | null>(null);
const downloadUrlFormRef = ref<FormInstance | null>(null);

const createRules = {
  name: [{ required: true, message: "必填", trigger: "blur" }],
};
const renameRules = {
  newName: [{ required: true, message: "必填", trigger: "blur" }],
};
const downloadUrlRules = {
  url: [{ required: true, message: "必填", trigger: "blur" }],
};

const submitCreate = () => props.onCreateSubmit(createFormRef.value);
const submitMove = () => props.onMoveSubmit(moveFormRef.value);
const submitRename = () => props.onRenameSubmit(renameFormRef.value);
const submitDownloadUrl = () => props.onDownloadUrlSubmit(downloadUrlFormRef.value);
</script>
