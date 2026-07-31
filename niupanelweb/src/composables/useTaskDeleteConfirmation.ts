import { h, type Ref } from "vue";
import { ElMessageBox } from "element-plus";
import { useTaskStore } from "../stores/tasks";

type UseTaskDeleteConfirmationOptions = {
  clearAllSelection: () => void;
  selectedIds: Ref<number[]>;
  taskStore: ReturnType<typeof useTaskStore>;
};

export function useTaskDeleteConfirmation({
  clearAllSelection,
  selectedIds,
  taskStore,
}: UseTaskDeleteConfirmationOptions) {
  const confirmDelete = (ids: number[]) => {
    let deleteVar = false;
    let deleteScript = false;

    ElMessageBox({
      title: ids.length > 1 ? "批量删除确认" : "删除确认",
      message: h("div", null, [
        h(
          "p",
          null,
          `确定删除 ${ids.length > 1 ? `选中的 ${ids.length} 个` : "该"} 任务?`,
        ),
        h(
          "div",
          {
            class:
              "mt-3 flex flex-col gap-2 p-3 bg-gray-50 dark:bg-dark-800 rounded",
          },
          [
            h("label", { class: "flex items-center gap-2 cursor-pointer" }, [
              h("input", {
                type: "checkbox",
                class:
                  "w-4 h-4 text-primary rounded border-gray-300 focus:ring-primary",
                onChange: (event: Event) => {
                  deleteVar = (event.target as HTMLInputElement).checked;
                },
              }),
              h("span", { class: "text-sm" }, "同时删除关联变量"),
            ]),
            h("label", { class: "flex items-center gap-2 cursor-pointer" }, [
              h("input", {
                type: "checkbox",
                class:
                  "w-4 h-4 text-primary rounded border-gray-300 focus:ring-primary",
                onChange: (event: Event) => {
                  deleteScript = (event.target as HTMLInputElement).checked;
                },
              }),
              h("span", { class: "text-sm" }, "同时删除脚本文件"),
            ]),
          ],
        ),
      ]),
      showCancelButton: true,
      confirmButtonText: "确定删除",
      cancelButtonText: "取消",
      type: "warning",
    })
      .then(async () => {
        await taskStore.batchDelete(ids, deleteVar, deleteScript);
        if (ids.length > 1) clearAllSelection();
      })
      .catch(() => {});
  };

  return {
    confirmDelete,
    handleDelete: (id: number) => confirmDelete([id]),
    handleBulkDelete: () => confirmDelete(selectedIds.value),
  };
}
