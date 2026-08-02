import { defineAsyncComponent, type Component } from "vue";

type MonacoEditorComponent = Component;

let monacoEditorPromise: Promise<MonacoEditorComponent> | null = null;

/**
 * Monaco is intentionally loaded behind a single async boundary.  Keeping
 * the promise here prevents every editor surface (task, file and compiler)
 * from creating its own loader and worker graph.
 */
const loadMonacoEditor = async (): Promise<MonacoEditorComponent> => {
  const [
    monaco,
    vueMonaco,
    editorWorker,
    jsonWorker,
    cssWorker,
    htmlWorker,
    typescriptWorker,
  ] = await Promise.all([
    import("monaco-editor"),
    import("@guolao/vue-monaco-editor"),
    import("monaco-editor/editor/editor.worker.js?worker"),
    import("monaco-editor/language/json/json.worker.js?worker"),
    import("monaco-editor/language/css/css.worker.js?worker"),
    import("monaco-editor/language/html/html.worker.js?worker"),
    import("monaco-editor/language/typescript/ts.worker.js?worker"),
  ]);

  self.MonacoEnvironment = {
    getWorker(_workerId: string, label: string) {
      if (label === "json") return new jsonWorker.default();
      if (label === "css" || label === "scss" || label === "less") {
        return new cssWorker.default();
      }
      if (label === "html" || label === "handlebars" || label === "razor") {
        return new htmlWorker.default();
      }
      if (label === "typescript" || label === "javascript") {
        return new typescriptWorker.default();
      }
      return new editorWorker.default();
    },
  };
  vueMonaco.loader.config({ monaco });
  await vueMonaco.loader.init();

  return vueMonaco.VueMonacoEditor;
};

export const createAsyncMonacoEditor = () =>
  defineAsyncComponent({
    loader: () => {
      monacoEditorPromise ??= loadMonacoEditor();
      return monacoEditorPromise;
    },
    loadingComponent: {
      template:
        '<div class="flex-center h-full"><div class="i-ep-loading animate-spin text-2xl"></div></div>',
    },
    delay: 120,
    timeout: 30000,
  });
