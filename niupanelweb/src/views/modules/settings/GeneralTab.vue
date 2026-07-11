<template>
  <div class="space-y-6">
    <section class="space-y-4">
      <div class="flex items-center justify-between gap-3 px-1">
        <div class="flex items-center gap-2">
          <div class="w-1.5 h-1.5 rounded-full bg-primary"></div>
          <h4 class="text-[13px] font-bold text-default">基础配置</h4>
        </div>
        <el-button
          type="primary"
          :loading="saving"
          @click="handleSave"
          class="!h-8 !rounded-md !px-3 !text-[12px] font-bold"
        >
          <div class="i-ep-check mr-1 text-sm"></div>
          保存修改
        </el-button>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        <div class="space-y-2">
          <label class="label-xs ml-1">系统名称</label>
          <el-input v-model="form.name" placeholder="NiuPanel" class="modern-input" />
        </div>
        <div class="space-y-2">
          <label class="label-xs ml-1">Logo URL</label>
          <el-input v-model="form.logo" placeholder="https://..." class="modern-input" />
        </div>
        <div class="space-y-2">
          <label class="label-xs ml-1">时区</label>
          <el-select v-model="form.timezone" class="w-full modern-input">
            <el-option label="Asia/Shanghai" value="Asia/Shanghai" />
            <el-option label="UTC" value="UTC" />
          </el-select>
        </div>
      </div>
    </section>

    <section class="space-y-4">
      <div class="flex items-center gap-2 px-1">
        <div class="w-1.5 h-1.5 rounded-full bg-indigo-500"></div>
        <h4 class="text-[13px] font-bold text-default">并发与性能</h4>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        <div class="space-y-2">
          <label class="label-xs ml-1">并发限制</label>
          <div class="flex items-center gap-2">
            <el-input-number
              v-model="form.max_concurrency"
              :min="1"
              :max="50"
              class="!w-full modern-number"
              controls-position="right"
            />
            <span class="w-8 shrink-0 text-[10px] font-bold text-muted">任务</span>
          </div>
          <p class="text-[10px] text-muted/50 leading-tight px-1">同时执行的最大脚本数</p>
        </div>

        <div class="space-y-2">
          <label class="label-xs ml-1">日志保留</label>
          <div class="flex items-center gap-2">
            <el-input-number
              v-model="form.log_retention_days"
              :min="1"
              :max="365"
              class="!w-full modern-number"
              controls-position="right"
            />
            <span class="w-8 shrink-0 text-[10px] font-bold text-muted">天</span>
          </div>
          <p class="text-[10px] text-muted/50 leading-tight px-1">自动清理过期审计和任务日志</p>
        </div>

        <div class="space-y-2">
          <label class="label-xs ml-1">GitHub 代理</label>
          <el-input v-model="form.github_proxy_url" placeholder="https://ghproxy.com/" class="modern-input" />
          <p class="text-[10px] text-muted/50 leading-tight px-1">加速脚本和前端资源拉取</p>
        </div>
      </div>
    </section>

    <section class="space-y-4">
      <div class="flex items-center gap-2 px-1">
        <div class="w-1.5 h-1.5 rounded-full bg-emerald-500"></div>
        <h4 class="text-[13px] font-bold text-default">包管理镜像</h4>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
        <div class="space-y-2">
          <label class="label-xs ml-1">Python (UV) 镜像</label>
          <el-input v-model="form.uv_python_mirror" placeholder="https://..." class="modern-input" />
        </div>
        <div class="space-y-2">
          <label class="label-xs ml-1">Node (FNM) 镜像</label>
          <el-input v-model="form.fnm_node_dist_mirror" placeholder="https://..." class="modern-input" />
        </div>
        <div class="space-y-2">
          <label class="label-xs ml-1">npm Registry 镜像</label>
          <el-input v-model="form.npm_registry_mirror" placeholder="https://registry.npmjs.org/" class="modern-input" />
        </div>
      </div>
    </section>
  </div>
</template>


<script setup lang="ts">
import { ref, reactive, onMounted } from "vue";
import { ElMessage } from "element-plus";
import * as settingsApi from "../../../api/settings";
import { useSystemSettings } from "../../../composables/useSystemSettings";
import type { GeneralSettings, SettingItem } from "@/types";

const { updateSystemSettingsState } = useSystemSettings();
const saving = ref(false);

const form = reactive<GeneralSettings>({
  name: "",
  logo: "",
  timezone: "Asia/Shanghai",
  max_concurrency: 5,
  log_retention_days: 30,
  github_proxy_url: "",
  uv_python_mirror: "",
  uv_pypi_mirror: "",
  default_python_version: "",
  default_node_version: "",
  fnm_node_dist_mirror: "",
  npm_registry_mirror: "",
});

const toSettingsMap = (items: SettingItem[]) =>
  new Map(items.map((item) => [item.key, item.value]));

const applySettingIfPresent = <K extends keyof GeneralSettings>(
  key: string,
  field: K,
  settings: Map<string, string>,
  transform: (value: string) => GeneralSettings[K] = (value) =>
    value as GeneralSettings[K],
) => {
  const value = settings.get(key);
  if (value !== undefined) form[field] = transform(value);
};

const load = async () => {
  try {
    const res = await settingsApi.getSettings();
    if (res.data) {
      const settings = toSettingsMap(res.data);
      applySettingIfPresent("system.name", "name", settings);
      applySettingIfPresent("system.logo", "logo", settings);
      applySettingIfPresent("system.timezone", "timezone", settings);
      applySettingIfPresent("system.max_concurrency", "max_concurrency", settings, Number);
      applySettingIfPresent("system.log_retention_days", "log_retention_days", settings, Number);
      applySettingIfPresent("system.github_proxy_url", "github_proxy_url", settings);
      applySettingIfPresent("system.uv_python_mirror", "uv_python_mirror", settings);
      applySettingIfPresent("system.uv_pypi_mirror", "uv_pypi_mirror", settings);
      applySettingIfPresent("system.default_python_version", "default_python_version", settings);
      applySettingIfPresent("system.default_node_version", "default_node_version", settings);
      applySettingIfPresent("system.fnm_node_dist_mirror", "fnm_node_dist_mirror", settings);
      applySettingIfPresent("system.npm_registry_mirror", "npm_registry_mirror", settings);
    }
  } catch (e) {}
};

const handleSave = async () => {
  saving.value = true;
  try {
    await settingsApi.updateGeneralSettings({ ...form });
    updateSystemSettingsState(form.name, form.logo);
    ElMessage.success("基础设置已保存");
  } finally {
    saving.value = false;
  }
};

onMounted(load);
</script>
