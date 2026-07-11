import type { Ref } from "vue";
import { ElMessage } from "element-plus";
import * as variableApi from "../api/variable";
import type { VariableRequest } from "@/types";

type UseVariableImportExportOptions = {
  activeTab: Ref<string>;
  getScopedTaskId: () => number | null;
  reload: () => unknown;
};

const chooseJsonFile = () => {
  return new Promise<File | null>((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".json";
    input.onchange = (event: Event) => {
      const target = event.target as HTMLInputElement;
      resolve(target.files?.[0] ?? null);
    };
    input.click();
  });
};

const readFileText = (file: File) => {
  return new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = (event) => resolve(String(event.target?.result ?? ""));
    reader.onerror = () => reject(reader.error);
    reader.readAsText(file);
  });
};

type LegacyVariableImportItem = {
  key?: unknown;
  name?: unknown;
  value?: unknown;
  remarks?: unknown;
  status?: unknown;
  enabled?: unknown;
  scope?: unknown;
  scope_id?: unknown;
  scope_ids?: unknown;
};

const toStringValue = (value: unknown) =>
  typeof value === "string" ? value : String(value ?? "");

const normalizeImportedVariables = (items: unknown[]): VariableRequest[] => {
  return items.map((item) => {
    const source = item as LegacyVariableImportItem;
    if (
      source.name !== undefined &&
      source.value !== undefined &&
      source.key === undefined
    ) {
      return {
        key: toStringValue(source.name),
        value: toStringValue(source.value),
        remarks: toStringValue(source.remarks),
        enabled: source.status !== undefined ? source.status === 0 : true,
        scope: "Global",
      };
    }

    return {
      key: toStringValue(source.key),
      value: toStringValue(source.value),
      remarks: toStringValue(source.remarks),
      enabled:
        typeof source.enabled === "boolean" ? source.enabled : true,
      scope: typeof source.scope === "string" ? source.scope : "Global",
      scope_id:
        typeof source.scope_id === "number" ? source.scope_id : undefined,
      scope_ids: Array.isArray(source.scope_ids)
        ? source.scope_ids.filter(
            (scopeId): scopeId is number => typeof scopeId === "number",
          )
        : undefined,
    };
  });
};

export function useVariableImportExport({
  activeTab,
  getScopedTaskId,
  reload,
}: UseVariableImportExportOptions) {
  const handleExport = async () => {
    try {
      const loadingMessage = ElMessage({
        message: "正在导出数据，请稍候...",
        type: "info",
        duration: 0,
      });

      const res = await variableApi.getVariables({
        scope: activeTab.value,
        scope_id: getScopedTaskId() ?? undefined,
        page: 1,
        page_size: 100000,
      });

      loadingMessage.close();

      const dataStr = JSON.stringify(res.data.items || [], null, 2);
      const blob = new Blob([dataStr], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `env_vars_${activeTab.value}.json`;
      a.click();
      URL.revokeObjectURL(url);
      ElMessage.success("导出成功");
    } catch {
      ElMessage.error("导出失败");
    }
  };

  const handleImport = async () => {
    const file = await chooseJsonFile();
    if (!file) return;

    try {
      const text = await readFileText(file);
      const json = JSON.parse(text);
      if (!Array.isArray(json)) return;

      await variableApi.importVariables(normalizeImportedVariables(json));
      ElMessage.success("Imported");
      reload();
    } catch {
      ElMessage.error("Format error");
    }
  };

  return {
    handleExport,
    handleImport,
  };
}
