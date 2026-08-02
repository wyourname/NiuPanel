<template>
  <div class="flex h-full min-h-0 flex-col overflow-hidden bg-card">
    <div
      class="flex h-10 shrink-0 items-center gap-2 border-b border-light bg-card px-2.5"
    >
      <div class="min-w-0 flex-1 flex items-center gap-2">
        <div class="i-ep-folder-opened shrink-0 text-[13px] text-muted"></div>
        <span class="truncate font-mono text-[10px] font-semibold text-secondary">
          {{ payload.filePath }}
        </span>
      </div>

      <span
        class="hidden shrink-0 rounded-md bg-soft px-2 py-1 font-mono text-[10px] font-bold text-secondary sm:inline-flex"
      >
        {{ editorLanguage }}
      </span>

      <span
        class="hidden shrink-0 items-center gap-1.5 text-[10px] font-bold sm:inline-flex"
        :class="isDirty ? 'text-amber-600 dark:text-amber-300' : 'text-muted'"
      >
        <span
          class="h-1.5 w-1.5 rounded-full"
          :class="isDirty ? 'bg-amber-500' : 'bg-emerald-500'"
          aria-hidden="true"
        ></span>
        {{ isDirty ? "未保存" : "已保存" }}
      </span>

      <button
        type="button"
        class="h-8 shrink-0 cursor-pointer rounded-md px-3 text-[11px] font-bold flex-center gap-1.5 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/35 disabled:cursor-not-allowed disabled:opacity-50"
        :class="isDirty ? 'bg-primary text-white hover:bg-primary/90' : 'bg-soft text-secondary'"
        :disabled="loading || saving || !isDirty"
        :aria-label="saving ? '正在保存文件' : '保存文件'"
        @click="saveFile"
      >
        <span
          :class="saving ? 'i-ep-loading animate-spin' : 'i-ep-check'"
          aria-hidden="true"
        ></span>
        {{ saving ? "保存中" : "保存" }}
      </button>
    </div>

    <div v-if="loading" class="min-h-0 flex-1 bg-[var(--editor-bg)] flex-center">
      <div class="flex items-center gap-2 text-xs font-bold text-muted">
        <div class="i-ep-loading animate-spin text-base text-primary"></div>
        正在加载文件
      </div>
    </div>

    <div
      v-else-if="loadError"
      class="min-h-0 flex-1 bg-[var(--editor-bg)] px-5 flex-center"
    >
      <div class="max-w-sm text-center">
        <div class="i-ep-warning mb-2 text-2xl text-amber-500"></div>
        <div class="text-sm font-bold text-default">文件加载失败</div>
        <div class="mt-1 break-all text-xs leading-5 text-muted">
          {{ loadError }}
        </div>
        <button
          type="button"
          class="mt-4 h-8 cursor-pointer rounded-md bg-soft px-3 text-xs font-bold text-secondary transition-colors hover:text-default focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/35"
          @click="loadFile"
        >
          重新加载
        </button>
      </div>
    </div>

    <FileCodeEditor
      v-else
      v-model:content="content"
      :file-name="payload.fileName"
      :is-dark="appStore.isDark"
      :is-mobile="appStore.isMobile"
      @save="saveFile"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import * as fileManagerApi from "@/api/file_manager";
import { useAppStore } from "@/stores/app";
import { useWorkspaceStore } from "@/stores/workspace";
import type { FileEditorWindowPayload } from "@/types/workspace";
import { getLanguageFromFilename } from "@/utils/editor";
import FileCodeEditor from "@/views/modules/file/components/FileCodeEditor.vue";

const props = defineProps<{
  payload: FileEditorWindowPayload;
  windowId: string;
}>();

const appStore = useAppStore();
const workspace = useWorkspaceStore();
const session = props.payload.session;
let unregisterCloseGuard: (() => void) | null = null;

const editorLanguage = computed(() =>
  getLanguageFromFilename(props.payload.fileName),
);
const content = computed({
  get: () => session.content,
  set: (value: string) => {
    session.content = value;
  },
});
const loading = computed(() => session.loading || !session.initialized);
const saving = computed(() => session.saving);
const loadError = computed(() => session.loadError);
const isDirty = computed(
  () =>
    !loading.value &&
    !loadError.value &&
    session.content !== session.savedContent,
);

const loadFile = async () => {
  session.loading = true;
  session.loadError = "";

  try {
    const response = await fileManagerApi.readFileContent(props.payload.filePath);
    const nextContent = response.data ?? "";
    session.content = nextContent;
    session.savedContent = nextContent;
  } catch {
    session.loadError = `无法读取 ${props.payload.filePath}`;
  } finally {
    session.initialized = true;
    session.loading = false;
  }
};

const saveFile = async () => {
  if (loading.value || saving.value || loadError.value) return;

  const sanitizedContent = session.content.replace(/\r\n/g, "\n");
  session.saving = true;

  try {
    await fileManagerApi.writeFileContent(
      props.payload.filePath,
      sanitizedContent,
    );
    session.content = sanitizedContent;
    session.savedContent = sanitizedContent;
    ElMessage.success("保存成功");
  } finally {
    session.saving = false;
  }
};

const confirmClose = async () => {
  if (!isDirty.value) return true;
  if (saving.value) {
    ElMessage.info("文件正在保存，请稍候");
    return false;
  }

  try {
    await ElMessageBox.confirm(
      `确定放弃对“${props.payload.fileName}”的修改吗？`,
      "未保存的修改",
      {
        type: "warning",
        customClass: "file-editor-unsaved-message",
        confirmButtonText: "放弃修改",
        cancelButtonText: "继续编辑",
        distinguishCancelAndClose: true,
      },
    );
    return true;
  } catch {
    return false;
  }
};

const handleBeforeUnload = (event: BeforeUnloadEvent) => {
  if (!isDirty.value) return;
  event.preventDefault();
  event.returnValue = "";
};

onMounted(() => {
  unregisterCloseGuard = workspace.registerCloseGuard(
    props.windowId,
    confirmClose,
  );
  window.addEventListener("beforeunload", handleBeforeUnload);
  if (!session.initialized && !session.loading) void loadFile();
});

onBeforeUnmount(() => {
  unregisterCloseGuard?.();
  window.removeEventListener("beforeunload", handleBeforeUnload);
});
</script>
