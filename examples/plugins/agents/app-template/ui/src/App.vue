<template>
  <section class="min-h-full overflow-auto bg-[#f7f8fa] p-4 text-[#1f2937]">
    <div class="mx-auto flex max-w-6xl flex-col gap-4">
      <header class="flex flex-wrap items-center justify-between gap-3 border-b border-[#d9dee7] bg-white px-4 py-3">
        <div class="min-w-0">
          <div class="flex flex-wrap items-center gap-2">
            <h1 class="truncate text-base font-semibold">{{ context.app.name }}</h1>
            <span class="rounded border border-[#cfd6e2] px-2 py-0.5 text-[11px] font-medium text-[#475569]">
              v{{ context.app.version }}
            </span>
          </div>
          <p class="mt-1 text-xs text-[#64748b]">{{ context.app.description }}</p>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <button
            type="button"
            class="rounded border border-[#cfd6e2] px-3 py-2 text-xs font-semibold transition-colors hover:bg-[#f1f5f9]"
            :class="activePath === '' ? 'bg-[#eff6ff] text-[#1d4ed8]' : 'bg-white text-[#334155]'"
            @click="navigate('/plugins/agent-app-template')"
          >
            概览
          </button>
          <button
            type="button"
            class="rounded border border-[#cfd6e2] px-3 py-2 text-xs font-semibold transition-colors hover:bg-[#f1f5f9]"
            :class="activePath === 'runs' ? 'bg-[#eff6ff] text-[#1d4ed8]' : 'bg-white text-[#334155]'"
            @click="navigate('/plugins/agent-app-template/runs')"
          >
            运行
          </button>
          <button
            type="button"
            class="rounded bg-[#2563eb] px-3 py-2 text-xs font-semibold text-white transition-colors hover:bg-[#1d4ed8]"
            @click="refreshPanelState"
          >
            刷新状态
          </button>
        </div>
      </header>

      <div class="grid gap-4 lg:grid-cols-[1fr_360px]">
        <main class="flex min-w-0 flex-col gap-4">
          <section v-if="activeView === 'overview'" class="border border-[#d9dee7] bg-white p-4">
            <div class="flex flex-wrap items-start justify-between gap-3">
              <div>
                <h2 class="text-sm font-semibold">应用概览</h2>
                <p class="mt-1 text-xs text-[#64748b]">
                  私有 Agent 工作台当前运行正常。
                </p>
              </div>
              <span class="rounded bg-[#ecfdf5] px-2 py-1 text-[11px] font-semibold text-[#047857]">
                route: {{ activePath || "overview" }}
              </span>
            </div>
            <div class="mt-4 grid gap-3 md:grid-cols-3">
              <div class="border border-[#e2e8f0] p-3">
                <div class="text-xs text-[#64748b]">能力</div>
                <div class="mt-2 text-sm font-semibold">{{ context.app.capabilities.join(", ") }}</div>
              </div>
              <div class="border border-[#e2e8f0] p-3">
                <div class="text-xs text-[#64748b]">显示位置</div>
                <div class="mt-2 text-sm font-semibold">
                  {{ context.app.ui.display.sidebar ? "sidebar" : "hidden" }} / {{ context.app.ui.display.workspace ? "workspace" : "route" }}
                </div>
              </div>
              <div class="border border-[#e2e8f0] p-3">
                <div class="text-xs text-[#64748b]">API 白名单</div>
                <div class="mt-2 text-sm font-semibold">{{ context.app.ui.api.allow.length }} rules</div>
              </div>
            </div>
          </section>

          <section v-if="activeView === 'runs'" class="border border-[#d9dee7] bg-white p-4">
            <div class="flex flex-wrap items-start justify-between gap-3">
              <div>
                <h2 class="text-sm font-semibold">Agent 调用</h2>
                <p class="mt-1 text-xs text-[#64748b]">
                  提交一次轻量任务，检查后端处理链路。
                </p>
              </div>
              <span class="rounded bg-[#ecfdf5] px-2 py-1 text-[11px] font-semibold text-[#047857]">
                native vue app
              </span>
            </div>

            <label class="mt-4 block text-xs font-semibold text-[#475569]" for="agent-note">
              输入
            </label>
            <textarea
              id="agent-note"
              v-model="note"
              class="mt-2 h-28 w-full resize-none rounded border border-[#cfd6e2] bg-white p-3 text-sm leading-5 outline-none transition-colors focus:border-[#2563eb]"
              placeholder="输入要交给 agent 后端处理的内容"
            ></textarea>

            <div class="mt-3 flex flex-wrap items-center gap-2">
              <button
                type="button"
                class="rounded bg-[#2563eb] px-3 py-2 text-xs font-semibold text-white transition-colors hover:bg-[#1d4ed8] disabled:cursor-not-allowed disabled:bg-[#93c5fd]"
                :disabled="invoking"
                @click="invokeAgent"
              >
                {{ invoking ? "调用中" : "调用 Agent" }}
              </button>
              <button
                type="button"
                class="rounded border border-[#cfd6e2] bg-white px-3 py-2 text-xs font-semibold text-[#334155] transition-colors hover:bg-[#f1f5f9]"
                @click="clearResult"
              >
                清空结果
              </button>
            </div>

            <pre class="mt-4 max-h-80 overflow-auto rounded bg-[#0f172a] p-3 text-xs leading-5 text-[#dbeafe]">{{ invokePreview }}</pre>
          </section>

          <section v-if="activeView === 'settings'" class="border border-[#d9dee7] bg-white p-4">
            <div class="flex flex-wrap items-start justify-between gap-3">
              <div>
                <h2 class="text-sm font-semibold">应用配置</h2>
                <p class="mt-1 text-xs text-[#64748b]">
                  当前插件清单和宿主运行上下文。
                </p>
              </div>
              <span class="rounded bg-[#f1f5f9] px-2 py-1 text-[11px] font-semibold text-[#475569]">
                hidden route
              </span>
            </div>
            <pre class="mt-4 max-h-[520px] overflow-auto rounded bg-[#0f172a] p-3 text-xs leading-5 text-[#dbeafe]">{{ settingsPreview }}</pre>
          </section>
        </main>

        <aside class="flex min-w-0 flex-col gap-4">
          <section class="border border-[#d9dee7] bg-white p-4">
            <h2 class="text-sm font-semibold">路由</h2>
            <div class="mt-3 flex flex-col gap-2">
              <button
                v-for="item in context.app.ui.routes"
                :key="item.path"
                type="button"
                class="flex cursor-pointer items-center justify-between gap-3 rounded border border-[#e2e8f0] px-3 py-2 text-left text-xs transition-colors hover:bg-[#f8fafc]"
                :class="routePathFor(item.path) === activePath ? 'bg-[#eff6ff]' : 'bg-white'"
                @click="navigate(item.path)"
              >
                <span class="min-w-0">
                  <span class="block truncate font-semibold">{{ item.title }}</span>
                  <span class="mt-0.5 block truncate text-[#64748b]">{{ item.path }}</span>
                </span>
                <span v-if="item.hidden" class="rounded bg-[#f1f5f9] px-2 py-0.5 text-[11px] text-[#475569]">
                  hidden
                </span>
              </button>
            </div>
          </section>

          <section class="border border-[#d9dee7] bg-white p-4">
            <h2 class="text-sm font-semibold">权限和 API 白名单</h2>
            <div class="mt-3 flex flex-wrap gap-2">
              <span
                v-for="permission in context.app.ui.permissions"
                :key="permission"
                class="rounded border border-[#cfd6e2] px-2 py-1 text-[11px] font-medium text-[#334155]"
              >
                {{ permission }}
              </span>
            </div>
            <div class="mt-4 flex flex-col gap-2">
              <div
                v-for="rule in context.app.ui.api.allow"
                :key="rule.path"
                class="rounded bg-[#f8fafc] px-3 py-2 text-xs text-[#475569]"
              >
                <span class="font-semibold text-[#334155]">
                  {{ rule.methods?.join(", ") || "*" }}
                </span>
                <span class="ml-2">{{ rule.path }}</span>
              </div>
            </div>
          </section>

          <section class="border border-[#d9dee7] bg-white p-4">
            <h2 class="text-sm font-semibold">上下文</h2>
            <pre class="mt-3 max-h-60 overflow-auto rounded bg-[#f8fafc] p-3 text-xs leading-5 text-[#334155]">{{ contextPreview }}</pre>
          </section>
        </aside>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import type { NiuPanelPluginContext } from "@niupanel/plugin-sdk";

const props = defineProps<{
  context: NiuPanelPluginContext;
}>();

const note = ref("检查当前插件 App 和 agents 后端调用是否正常。");
const invoking = ref(false);
const invokeResult = ref<unknown>(null);
const activePath = ref(props.context.route.path);
let unsubscribeRoute: (() => void) | undefined;

const activeView = computed(() => {
  if (activePath.value === "runs") return "runs";
  if (activePath.value === "settings") return "settings";
  return "overview";
});

const contextPreview = computed(() =>
  JSON.stringify(
    {
      pluginId: props.context.pluginId,
      route: props.context.route,
      capabilities: props.context.app.capabilities,
      display: props.context.app.ui.display,
      capabilitiesAreBackendDeclared: true,
    },
    null,
    2,
  ),
);

const invokePreview = computed(() =>
  JSON.stringify(invokeResult.value ?? { status: "waiting" }, null, 2),
);

const settingsPreview = computed(() =>
  JSON.stringify(
    {
      app: props.context.app,
      route: {
        path: activePath.value,
        query: props.context.route.query,
      },
    },
    null,
    2,
  ),
);

const navigate = async (path: string) => {
  await props.context.ui.navigate(path);
};

const routePathFor = (path: string) => path.replace(/^\/plugins\/agent-app-template\/?/, "");

const invokeAgent = async () => {
  invoking.value = true;
  try {
    invokeResult.value = await props.context.api.invoke("summarize", {
      note: note.value,
      route: props.context.route.path,
    });
    props.context.ui.toast("Agent 调用完成", "success");
  } catch (error) {
    invokeResult.value = {
      error: error instanceof Error ? error.message : String(error),
    };
    props.context.ui.toast("Agent 调用失败", "error");
  } finally {
    invoking.value = false;
  }
};

const refreshPanelState = async () => {
  try {
    await props.context.api.request({
      method: "GET",
      path: "/plugins/apps",
    });
    props.context.ui.toast("面板状态已刷新", "success");
  } catch (error) {
    props.context.ui.toast(
      error instanceof Error ? error.message : "刷新失败",
      "error",
    );
  }
};

const clearResult = () => {
  invokeResult.value = null;
};

onMounted(() => {
  unsubscribeRoute = props.context.route.onChange((route) => {
    activePath.value = route.path;
  });
});

onUnmounted(() => {
  unsubscribeRoute?.();
});
</script>
