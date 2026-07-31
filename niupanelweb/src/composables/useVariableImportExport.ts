import type { Ref } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import * as variableApi from "../api/variable";
import type { Variable, VariableRequest } from "@/types";

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
  task_ids?: unknown;
};

const toStringValue = (value: unknown) =>
  typeof value === "string" ? value : String(value ?? "");

const normalizeTaskIds = (source: LegacyVariableImportItem) => {
  const rawIds = Array.isArray(source.task_ids)
    ? source.task_ids
    : source.scope_ids;
  return Array.isArray(rawIds)
    ? rawIds.filter(
        (scopeId): scopeId is number =>
          typeof scopeId === "number" && Number.isInteger(scopeId),
      )
    : undefined;
};

const normalizeImportedVariables = (
  items: unknown[],
  activeScope: string,
  scopedTaskId: number | null,
): VariableRequest[] => {
  return items.map((item) => {
    if (!item || typeof item !== "object") {
      throw new Error("变量条目必须是对象");
    }
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
        scope: activeScope,
        scope_ids:
          activeScope === "Script" && scopedTaskId ? [scopedTaskId] : undefined,
      };
    }

    const scope =
      source.scope === "Global" || source.scope === "Script"
        ? source.scope
        : activeScope;
    const taskIds = normalizeTaskIds(source);
    return {
      key: toStringValue(source.key),
      value: toStringValue(source.value),
      remarks: toStringValue(source.remarks),
      enabled:
        typeof source.enabled === "boolean" ? source.enabled : true,
      scope,
      scope_id:
        scope === "Script" && typeof source.scope_id === "number"
          ? source.scope_id
          : undefined,
      scope_ids:
        scope === "Script"
          ? taskIds?.length
            ? taskIds
            : scopedTaskId
              ? [scopedTaskId]
              : undefined
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
      await ElMessageBox.confirm(
        "导出文件将包含变量明文，请妥善保存并避免上传到公开位置。",
        "导出敏感数据",
        {
          type: "warning",
          confirmButtonText: "继续导出",
          cancelButtonText: "取消",
        },
      );
    } catch {
      return;
    }

    const loadingMessage = ElMessage({
      message: "正在导出数据，请稍候...",
      type: "info",
      duration: 0,
    });
    try {
      const items: Variable[] = [];
      let page = 1;
      let total = 0;
      do {
        const res = await variableApi.getVariablesWithValues({
          scope: activeTab.value,
          scope_id: getScopedTaskId() ?? undefined,
          page,
          page_size: 1000,
        });
        const pageItems = res.data.items || [];
        if (pageItems.length === 0 && items.length < res.data.total) {
          throw new Error("导出分页数据不完整");
        }
        items.push(...pageItems);
        total = res.data.total;
        page += 1;
      } while (items.length < total);

      const dataStr = JSON.stringify(items, null, 2);
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
    } finally {
      loadingMessage.close();
    }
  };

  const handleImport = async () => {
    const file = await chooseJsonFile();
    if (!file) return;

    try {
      const text = await readFileText(file);
      const json = JSON.parse(text);
      if (!Array.isArray(json)) {
        throw new Error("JSON 顶层必须是数组");
      }

      await variableApi.importVariables(
        normalizeImportedVariables(
          json,
          activeTab.value,
          getScopedTaskId(),
        ),
      );
      ElMessage.success("导入成功");
      reload();
    } catch {
      ElMessage.error("导入格式错误或数据不合法");
    }
  };

  return {
    handleExport,
    handleImport,
  };
}
