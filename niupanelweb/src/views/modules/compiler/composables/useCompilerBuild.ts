import { computed, onMounted, reactive, ref } from "vue";
import { ElMessage } from "element-plus";
import type { editor } from "monaco-editor";
import * as compilerApi from "@/api/compiler";
import { useAppStore } from "@/stores/app";
import request from "@/utils/request";

type CompilerForm = {
  code: string;
  function_name: string;
  obfuscate: boolean;
  target_versions: string[];
};

const DEFAULT_CODE =
  'def main():\n    print("Hello NiuPanel!")\n\nif __name__=="__main__":\n    main()';

export function useCompilerBuild() {
  const appStore = useAppStore();
  const availableVersions = ref<string[]>([]);
  const loadingVersions = ref(false);
  const submitting = ref(false);
  const resultFile = ref("");
  const currentFileName = ref("");
  const showMobileOps = ref(false);

  const form = reactive<CompilerForm>({
    code: DEFAULT_CODE,
    target_versions: [],
    function_name: "main",
    obfuscate: true,
  });

  const editorOptions = computed<editor.IStandaloneEditorConstructionOptions>(() => ({
    automaticLayout: true,
    fontSize: appStore.isMobile ? 12 : 13,
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
    lineNumbers: "on",
    roundedSelection: false,
    wordWrap: "on",
    scrollbar: {
      vertical: "visible",
      horizontal: "visible",
      verticalScrollbarSize: 10,
      horizontalScrollbarSize: 10,
    },
  }));

  const setSourceFile = (code: string, fileName: string) => {
    form.code = code;
    currentFileName.value = fileName;
  };

  const toggleVersion = (version: string) => {
    const index = form.target_versions.indexOf(version);
    if (index > -1) {
      form.target_versions.splice(index, 1);
      return;
    }
    form.target_versions.push(version);
  };

  const fetchVersions = async () => {
    loadingVersions.value = true;
    try {
      const res = await compilerApi.getSupportedVersions();
      availableVersions.value = res.data;
      if (res.data.length > 0) {
        form.target_versions = [res.data[0]];
      }
    } catch {
      ElMessage.error("获取 Python 版本失败");
    } finally {
      loadingVersions.value = false;
    }
  };

  const handleCompile = async () => {
    if (!form.code.trim()) {
      ElMessage.warning("请输入源代码");
      return;
    }
    if (form.target_versions.length === 0) {
      ElMessage.warning("请选择目标版本");
      return;
    }

    submitting.value = true;
    resultFile.value = "";
    try {
      const res = await compilerApi.encryptCode({
        code: form.code,
        versions: form.target_versions,
        function_name: form.function_name,
        obfuscate: form.obfuscate,
      });
      resultFile.value = res.data;
      ElMessage.success("加密任务已提交");
      if (appStore.isMobile) showMobileOps.value = false;
    } catch {
      ElMessage.error("编译提交失败");
    } finally {
      submitting.value = false;
    }
  };

  const handleDownload = () => {
    if (!resultFile.value) return;
    window.open(
      `${request.defaults.baseURL}/files/download/compile/${resultFile.value}`,
    );
  };

  onMounted(fetchVersions);

  return {
    availableVersions,
    currentFileName,
    editorOptions,
    form,
    handleCompile,
    handleDownload,
    loadingVersions,
    resultFile,
    setSourceFile,
    showMobileOps,
    submitting,
    toggleVersion,
  };
}
