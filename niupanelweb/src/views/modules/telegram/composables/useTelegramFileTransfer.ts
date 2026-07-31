import { ref } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import * as fileApi from "@/api/file_manager";
import * as telegramApi from "@/api/telegram";
import type { FileItem } from "@/types";

type FileActionCommand = "local" | "server";

type UseTelegramFileTransferOptions = {
  getAdminChatId: () => string;
};

const isFileActionCommand = (command: unknown): command is FileActionCommand => {
  return command === "local" || command === "server";
};

const joinPath = (currentPath: string, name: string) => {
  return currentPath === "/" ? `/${name}` : `${currentPath}/${name}`;
};

export function useTelegramFileTransfer({
  getAdminChatId,
}: UseTelegramFileTransferOptions) {
  const showServerFileSelector = ref(false);
  const serverFiles = ref<FileItem[]>([]);
  const currentPath = ref("/");
  const localFileInput = ref<HTMLInputElement | null>(null);

  const loadServerFiles = async (path: string) => {
    try {
      const res = await fileApi.listDirectoryContents(path);
      if (res.data) {
        serverFiles.value = res.data;
        currentPath.value = path;
      }
    } catch {
      ElMessage.error("访问失败");
    }
  };

  const handleFileAction = (command: unknown) => {
    if (!isFileActionCommand(command)) return;

    if (command === "local") {
      localFileInput.value?.click();
    } else {
      void loadServerFiles("/");
      showServerFileSelector.value = true;
    }
  };

  const onLocalFileChange = async (event: Event) => {
    if (!(event.target instanceof HTMLInputElement)) return;

    const adminChatId = getAdminChatId();
    const file = event.target.files?.[0];
    if (!file || !adminChatId) return;

    try {
      await telegramApi.sendFile(adminChatId, file);
      ElMessage.success("文件已发送");
    } catch {
      ElMessage.error("发送失败");
    } finally {
      event.target.value = "";
    }
  };

  const confirmSendServerFile = (path: string) => {
    const adminChatId = getAdminChatId();
    if (!adminChatId) {
      ElMessage.warning("未配置管理员 ID");
      return;
    }

    ElMessageBox.confirm(`发送文件 ${path} 到 Telegram？`, "确认", {
      confirmButtonText: "发送",
      cancelButtonText: "取消",
      type: "info",
    }).then(async () => {
      try {
        await telegramApi.sendServerFile(adminChatId, path);
        ElMessage.success("已发送");
        showServerFileSelector.value = false;
      } catch {
        ElMessage.error("发送失败");
      }
    });
  };

  const handleFileRowClick = (row: FileItem) => {
    const filePath = joinPath(currentPath.value, row.name);
    if (row.is_dir) {
      void loadServerFiles(filePath);
    } else {
      confirmSendServerFile(filePath);
    }
  };

  const goBack = () => {
    const parts = currentPath.value.split("/").filter(Boolean);
    parts.pop();
    void loadServerFiles(parts.length === 0 ? "/" : `/${parts.join("/")}`);
  };

  const formatSize = (bytes: number) => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), sizes.length - 1);
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))}${sizes[i]}`;
  };

  const getFileIconClass = (name: string) => {
    const ext = name.split(".").pop()?.toLowerCase();
    if (ext && ["jpg", "jpeg", "png", "gif", "webp"].includes(ext)) return "i-ep-picture";
    if (ext && ["pdf", "doc", "docx", "txt", "md"].includes(ext)) return "i-ep-document";
    if (ext && ["sh", "js", "py", "ts", "yaml", "toml", "json"].includes(ext)) return "i-ep-document-copy";
    return "i-ep-files";
  };

  return {
    currentPath,
    formatSize,
    getFileIconClass,
    goBack,
    handleFileAction,
    handleFileRowClick,
    loadServerFiles,
    localFileInput,
    onLocalFileChange,
    serverFiles,
    showServerFileSelector,
  };
}
