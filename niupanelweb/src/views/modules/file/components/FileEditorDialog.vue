<template>
  <ResponsiveDialog
    v-model:visible="visibleValue"
    :title="currentFile?.name"
    desktop-size="fluid"
    desktop-height="min(760px, calc(var(--app-viewport-height) - 48px))"
    content-preset="workspace"
    size="100%"
    destroy-on-close
    append-to-body
    custom-class="editor-overlay"
  >
    <template #title>
      <div class="min-w-0">
        <div class="truncate text-[13px] font-bold leading-tight text-default">
          {{ currentFile?.name || "文件编辑器" }}
        </div>
        <div class="mt-0.5 truncate font-mono text-[10px] leading-tight text-muted">
          {{ currentFile?.path }}
        </div>
      </div>
    </template>

    <template #header-actions>
      <span class="hidden rounded-md bg-soft px-2 py-1 font-mono text-[10px] font-bold text-secondary md:inline-flex">
        {{ editorLanguage }}
      </span>
      <el-button
        type="primary"
        :loading="saving"
        class="!min-h-11 !rounded-md !px-3 font-bold md:!min-h-9"
        @click="emit('save')"
      >
        <span class="i-ep-check mr-1" aria-hidden="true"></span>
        保存
      </el-button>
    </template>

    <div class="flex h-full min-h-0 flex-1 flex-col overflow-hidden bg-[var(--editor-bg)]">
      <FileCodeEditor
        v-if="visible"
        v-model:content="contentValue"
        :file-name="currentFile?.name || ''"
        :is-dark="isDark"
        :is-mobile="isMobile"
        @save="emit('save')"
      />
    </div>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { FileItem } from "../../../../composables/useFileOperations";
import { getLanguageFromFilename } from "../../../../utils/editor";
import ResponsiveDialog from "../../../../components/common/ResponsiveDialog.vue";
import FileCodeEditor from "./FileCodeEditor.vue";

const props = defineProps<{
  content: string;
  currentFile: FileItem | null;
  isDark: boolean;
  isMobile: boolean;
  saving: boolean;
  visible: boolean;
}>();

const emit = defineEmits<{
  (event: "save"): void;
  (event: "update:content", content: string): void;
  (event: "update:visible", visible: boolean): void;
}>();

const visibleValue = computed({
  get: () => props.visible,
  set: (visible: boolean) => emit("update:visible", visible),
});

const contentValue = computed({
  get: () => props.content,
  set: (content: string) => emit("update:content", content),
});

const editorLanguage = computed(() => {
  if (!props.currentFile) return "plaintext";
  return getLanguageFromFilename(props.currentFile.name);
});
</script>
