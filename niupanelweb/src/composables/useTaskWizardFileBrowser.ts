import { computed, onScopeDispose, ref, watch, type ComputedRef } from "vue";
import { debounce } from "lodash-es";
import type { UploadFile } from "element-plus";
import * as fileApi from "../api/file_manager";
import {
  inferScriptEnvironment,
  type TaskWizardForm,
} from "./taskWizardHelpers";

export type TaskWizardBrowserItem = {
  name: string;
  path: string;
  is_dir: boolean;
  size?: number;
};

type UseTaskWizardFileBrowserOptions = {
  form: Pick<TaskWizardForm, "env_type" | "env_version" | "name" | "path">;
  isEdit: ComputedRef<boolean>;
  nodeVersions: ComputedRef<string[]>;
  pythonVersions: ComputedRef<string[]>;
};

export function useTaskWizardFileBrowser({
  form,
  isEdit,
  nodeVersions,
  pythonVersions,
}: UseTaskWizardFileBrowserOptions) {
  const currentPath = ref("");
  const searchQuery = ref("");
  const browserItems = ref<TaskWizardBrowserItem[]>([]);
  const browserLoading = ref(false);
  const uploadedFile = ref<File | null>(null);

  const pathParts = computed(() =>
    currentPath.value ? currentPath.value.split("/") : [],
  );

  const navigate = async (path: string, query?: string) => {
    currentPath.value = path;
    browserLoading.value = true;

    try {
      const res = await fileApi.listDirectoryContents(
        path,
        query ? { q: query } : undefined,
      );

      browserItems.value = (res.data || [])
        .map((item) => ({ ...item }))
        .sort((a: TaskWizardBrowserItem, b: TaskWizardBrowserItem) =>
          Number(b.is_dir) - Number(a.is_dir),
        );
    } catch {
      browserItems.value = [];
    } finally {
      browserLoading.value = false;
    }
  };

  const debouncedSearch = debounce((query: string) => {
    navigate(currentPath.value, query);
  }, 500);

  watch(searchQuery, (newQuery) => {
    debouncedSearch(newQuery);
  });

  onScopeDispose(() => {
    debouncedSearch.cancel();
  });

  const navigateUp = () => {
    const parts = currentPath.value.split("/");
    parts.pop();
    navigate(parts.join("/"));
  };

  const handleBrowserItemClick = (item: TaskWizardBrowserItem) => {
    if (item.is_dir) {
      searchQuery.value = "";
      debouncedSearch.cancel();
      navigate(item.path);
      return;
    }

    form.path = item.path;

    if (searchQuery.value) {
      const lastSlashIndex = item.path.lastIndexOf("/");
      const parentDir =
        lastSlashIndex !== -1 ? item.path.substring(0, lastSlashIndex) : "";

      debouncedSearch.cancel();
      searchQuery.value = "";
      navigate(parentDir);
    }

    if (!isEdit.value) {
      form.name = item.name;
    }

    const inferredEnv = inferScriptEnvironment(
      item.name,
      pythonVersions.value,
      nodeVersions.value,
    );
    form.env_type = inferredEnv.env_type;
    form.env_version = inferredEnv.env_version;
  };

  const applyUploadedFile = (file: File) => {
    uploadedFile.value = file;

    if (!isEdit.value) {
      form.name = file.name;
    }

    const inferredEnv = inferScriptEnvironment(
      file.name,
      pythonVersions.value,
      nodeVersions.value,
    );
    form.env_type = inferredEnv.env_type;
    form.env_version = inferredEnv.env_version;
  };

  const handleScriptUpload = (file: UploadFile) => {
    if (file.raw) {
      applyUploadedFile(file.raw);
    }
  };

  return {
    applyUploadedFile,
    browserItems,
    browserLoading,
    currentPath,
    handleBrowserItemClick,
    handleScriptUpload,
    navigate,
    navigateUp,
    pathParts,
    searchQuery,
    uploadedFile,
  };
}
