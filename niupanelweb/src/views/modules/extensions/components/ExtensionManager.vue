<template>
  <div class="mx-auto w-full max-w-[1280px] space-y-4">
    <header class="flex flex-wrap items-center justify-between gap-3">
      <div class="min-w-0">
        <div class="flex items-center gap-2.5">
          <span class="h-9 w-9 shrink-0 rounded-md bg-primary text-white flex-center">
            <span class="i-carbon-plug text-[17px]"></span>
          </span>
          <div class="min-w-0">
            <h1 class="m-0 truncate text-[18px] font-bold text-default">扩展</h1>
            <p class="mt-0.5 text-[11px] text-muted">
              {{ allPlugins.length }} 个已安装 · {{ enabledCount }} 个运行中 · {{ appCount }} 个应用 · {{ themeCount }} 个主题
            </p>
          </div>
        </div>
      </div>

      <div class="flex items-center gap-2">
        <button
          type="button"
          class="h-9 w-9 rounded-md border border-light bg-card text-secondary flex-center transition-colors hover:bg-soft hover:text-default"
          title="刷新"
          aria-label="刷新扩展"
          @click="loadAll"
        >
          <span class="i-ep-refresh" :class="loading ? 'animate-spin' : ''"></span>
        </button>
        <button
          type="button"
          class="h-9 w-9 rounded-md border border-light bg-card text-secondary flex-center transition-colors hover:bg-soft hover:text-default"
          title="发布源设置"
          aria-label="发布源设置"
          @click="marketSourcesDialogVisible = true"
        >
          <span class="i-ep-setting"></span>
        </button>
        <el-dropdown trigger="click" @command="openInstall">
          <button
            type="button"
            class="h-9 rounded-md bg-primary px-3 text-[12px] font-bold text-white flex items-center gap-1.5 transition-opacity hover:opacity-90"
          >
            <span class="i-ep-plus"></span>
            安装扩展
            <span class="i-ep-arrow-down text-[10px]"></span>
          </button>
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item command="upload">
                <span class="i-ep-upload mr-2"></span>上传安装包
              </el-dropdown-item>
              <el-dropdown-item command="path">
                <span class="i-ep-folder-opened mr-2"></span>服务端路径
              </el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
      </div>
    </header>

    <div class="flex flex-wrap items-center gap-3 border-y border-light py-3">
      <div class="flex rounded-md border border-light bg-card p-0.5">
        <button
          v-for="view in views"
          :key="view.id"
          type="button"
          class="h-8 rounded px-3 text-[12px] font-bold transition-colors"
          :class="activeView === view.id ? 'bg-soft text-primary' : 'text-muted hover:text-default'"
          @click="activeView = view.id"
        >
          {{ view.label }}
          <span v-if="view.id === 'installed'" class="ml-1 text-[10px] text-muted">
            {{ allPlugins.length }}
          </span>
          <span v-if="view.id === 'market' && market.updates.length" class="ml-1 text-[10px] text-rose-500">
            {{ market.updates.length }}
          </span>
          <span v-if="view.id === 'themes'" class="ml-1 text-[10px] text-muted">
            {{ pluginThemes.themes.length }}
          </span>
        </button>
      </div>

      <el-input
        v-model="searchQuery"
        clearable
        class="min-w-[220px] flex-1"
        placeholder="搜索名称、ID 或能力"
      >
        <template #prefix><span class="i-ep-search text-muted"></span></template>
      </el-input>

      <el-select
        v-if="activeView === 'installed'"
        v-model="statusFilter"
        class="w-[130px]"
        aria-label="状态筛选"
      >
        <el-option label="全部状态" value="all" />
        <el-option label="已启用" value="enabled" />
        <el-option label="已禁用" value="disabled" />
        <el-option label="异常" value="error" />
      </el-select>
    </div>

    <section v-if="activeView === 'installed'">
      <div v-if="loading" class="h-40 flex-center text-[12px] font-semibold text-muted">正在读取扩展...</div>
      <div v-else-if="visiblePlugins.length === 0" class="h-44 flex flex-col items-center justify-center gap-2 text-muted">
        <span class="i-carbon-plug text-[24px]"></span>
        <span class="text-[12px] font-semibold">没有符合条件的扩展</span>
      </div>
      <div v-else class="overflow-hidden rounded-md border border-light bg-card">
        <article
          v-for="item in visiblePlugins"
          :key="item.record.manifest.id"
          class="flex flex-wrap items-center gap-3 border-b border-light px-3 py-3 last:border-b-0 transition-colors hover:bg-soft/45"
        >
          <div
            class="h-10 w-10 shrink-0 rounded-md flex-center"
            :class="item.record.enabled ? 'accent-subtle' : 'bg-soft text-muted'"
          >
            <span :class="pluginIcon(item)" class="text-[17px]"></span>
          </div>

          <div class="min-w-[180px] flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <h2 class="m-0 truncate text-[13px] font-bold text-default">{{ item.record.manifest.name }}</h2>
              <span
                v-for="capability in visibleCapabilities(item)"
                :key="capability"
                class="rounded bg-soft px-1.5 py-0.5 text-[9px] font-bold text-secondary"
              >
                {{ capabilityLabel(capability) }}
              </span>
              <span
                v-if="!visibleCapabilities(item).length && item.record.manifest.ui?.enabled"
                class="rounded bg-amber-500/10 px-1.5 py-0.5 text-[9px] font-bold text-amber-700 dark:text-amber-300"
              >
                同源原生 UI
              </span>
              <span
                v-if="item.record.manifest.theme?.enabled"
                class="rounded bg-emerald-500/10 px-1.5 py-0.5 text-[9px] font-bold text-emerald-700 dark:text-emerald-300"
              >
                面板主题
              </span>
              <span
                v-if="item.record.manifest.runtime_permissions.includes('network_outbound')"
                class="rounded bg-amber-500/10 px-1.5 py-0.5 text-[9px] font-bold text-amber-700 dark:text-amber-300"
              >
                Web 出站
              </span>
              <span v-if="item.record.source === 'builtin'" class="rounded bg-blue-500/10 px-1.5 py-0.5 text-[9px] font-bold text-blue-600 dark:text-blue-300">
                内置
              </span>
            </div>
            <p class="mt-1 truncate text-[10px] text-muted">
              {{ item.record.manifest.id }} · v{{ item.record.manifest.version }} · {{ item.record.manifest.description || "无描述" }}
            </p>
          </div>

          <div class="hidden min-w-[130px] md:block">
            <div class="text-[9px] font-semibold text-muted">运行时</div>
            <div class="mt-1 font-mono text-[10px] font-semibold text-secondary">{{ item.record.manifest.runtime }}</div>
          </div>

          <div class="min-w-[110px]">
            <div class="text-[9px] font-semibold text-muted">健康状态</div>
            <div class="mt-1 flex items-center gap-1.5 text-[10px] font-bold" :class="healthTone(pluginHealthReport(item))">
              <span class="h-1.5 w-1.5 rounded-full bg-current"></span>
              {{ healthText(pluginHealthReport(item)) }}
            </div>
          </div>

          <div class="ml-auto flex items-center gap-2">
            <el-switch
              v-if="item.record.source !== 'builtin'"
              :model-value="item.record.enabled"
              size="small"
              :loading="busyPlugin === item.record.manifest.id"
              @change="togglePlugin(item)"
            />
            <button
              v-if="item.record.manifest.ui?.enabled && item.record.enabled"
              type="button"
              class="accent-subtle h-8 rounded-md px-2.5 text-[11px] font-bold transition-[filter,box-shadow] duration-200 hover:brightness-95 focus-visible:ring-2 focus-visible:ring-primary/25"
              @click="openPluginApp(item)"
            >
              打开
            </button>
            <el-dropdown
              v-if="item.record.source !== 'builtin'"
              trigger="click"
              @command="handlePluginCommand($event, item)"
            >
              <button
                type="button"
                class="h-8 w-8 rounded-md text-secondary flex-center transition-colors hover:bg-soft hover:text-default"
                title="更多操作"
              >
                <span class="i-ep-more-filled"></span>
              </button>
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item command="path-update">路径更新</el-dropdown-item>
                  <el-dropdown-item command="upload-update">上传更新</el-dropdown-item>
                  <el-dropdown-item command="history">版本历史</el-dropdown-item>
                  <el-dropdown-item divided command="uninstall">
                    <span class="text-rose-600">卸载</span>
                  </el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
          </div>
        </article>
      </div>
    </section>

    <section v-else-if="activeView === 'themes'" class="space-y-3">
      <div class="flex flex-wrap items-center justify-between gap-3 border-b border-light pb-3">
        <div>
          <h2 class="m-0 text-[13px] font-bold text-default">面板外观</h2>
          <p class="mt-1 text-[10px] text-muted">主题只使用经过校验的颜色令牌，不加载脚本或任意 CSS。</p>
        </div>
        <el-button v-if="pluginThemes.activeThemeId" size="small" @click="pluginThemes.setActiveTheme(null)">
          恢复默认主题
        </el-button>
      </div>

      <div v-if="pluginThemes.loading" class="h-40 flex-center text-[12px] font-semibold text-muted">正在读取主题...</div>
      <div v-else-if="!pluginThemes.themes.length" class="h-44 flex flex-col items-center justify-center gap-2 text-muted">
        <span class="i-carbon-color-palette text-[24px]"></span>
        <span class="text-[12px] font-semibold">暂无已启用主题扩展</span>
      </div>
      <div v-else class="grid gap-3 md:grid-cols-2">
        <article
          v-for="theme in pluginThemes.themes"
          :key="theme.plugin_id"
          class="border p-3 transition-colors"
          :class="pluginThemes.activeThemeId === theme.plugin_id ? 'border-primary bg-soft/50' : 'border-light bg-card'"
        >
          <div class="flex items-start gap-3">
            <div class="grid h-11 w-11 shrink-0 grid-cols-2 overflow-hidden rounded-md border border-light">
              <span
                v-for="color in themeSwatches(theme)"
                :key="color"
                :style="{ backgroundColor: color }"
              ></span>
            </div>
            <div class="min-w-0 flex-1">
              <div class="flex flex-wrap items-center gap-2">
                <h3 class="m-0 truncate text-[12px] font-bold text-default">{{ theme.name }}</h3>
                <span v-if="pluginThemes.activeThemeId === theme.plugin_id" class="accent-subtle rounded px-1.5 py-0.5 text-[9px] font-bold">使用中</span>
              </div>
              <p class="mt-1 line-clamp-2 text-[10px] leading-4 text-muted">{{ theme.description }}</p>
              <div class="mt-1 font-mono text-[9px] text-muted">{{ theme.plugin_id }} · v{{ theme.version }}</div>
            </div>
          </div>
          <div class="mt-3 flex justify-end">
            <el-button
              size="small"
              :type="pluginThemes.activeThemeId === theme.plugin_id ? 'default' : 'primary'"
              :disabled="pluginThemes.activeThemeId === theme.plugin_id"
              @click="pluginThemes.setActiveTheme(theme.plugin_id)"
            >
              {{ pluginThemes.activeThemeId === theme.plugin_id ? "已应用" : "应用主题" }}
            </el-button>
          </div>
        </article>
      </div>
    </section>

    <section v-else class="space-y-3">
      <div class="flex flex-wrap items-center gap-2">
        <el-select
          v-model="market.selectedUrl"
          class="min-w-[260px] flex-1"
          placeholder="选择插件发布源"
          filterable
        >
          <el-option
            v-for="source in market.sources"
            :key="source.url"
            :label="`${source.name || source.url}${source.enabled ? '' : '（停用）'}`"
            :value="source.url"
          />
        </el-select>
        <el-button :loading="market.loading" @click="loadMarket">
          <span class="i-ep-download mr-1.5"></span>加载目录
        </el-button>
        <el-button :loading="market.updatesLoading" @click="checkMarketUpdates">
          <span class="i-ep-refresh mr-1.5"></span>检查更新
        </el-button>
      </div>

      <div v-if="marketVisibleUpdates.length" class="overflow-hidden rounded-md border border-amber-300/60 bg-amber-50/60 dark:border-amber-700/50 dark:bg-amber-950/15">
        <div class="border-b border-amber-300/50 px-3 py-2 text-[11px] font-bold text-amber-800 dark:border-amber-700/40 dark:text-amber-200">
          {{ marketVisibleUpdates.length }} 个扩展可更新
        </div>
        <div
          v-for="update in marketVisibleUpdates"
          :key="`${update.source_url}:${update.plugin_id}`"
          class="flex flex-wrap items-center gap-3 border-b border-amber-200/60 px-3 py-2.5 last:border-b-0 dark:border-amber-800/40"
        >
          <div class="min-w-0 flex-1">
            <div class="text-[12px] font-bold text-default">{{ update.entry.name }}</div>
            <div class="mt-0.5 text-[10px] text-muted">v{{ update.installed_version }} → v{{ update.available_version }} · {{ update.source_name }}</div>
          </div>
          <el-button size="small" type="warning" @click="installFromMarket(update.entry, update.source_url)">更新</el-button>
        </div>
      </div>

      <div v-if="market.loading" class="h-40 flex-center text-[12px] font-semibold text-muted">正在读取插件目录...</div>
      <div v-else-if="!market.index" class="h-40 flex-center text-[12px] font-semibold text-muted">请选择发布源并加载目录</div>
      <div v-else-if="marketVisiblePlugins.length === 0" class="h-40 flex-center text-[12px] font-semibold text-muted">没有符合条件的市场扩展</div>
      <div v-else class="overflow-hidden rounded-md border border-light bg-card">
        <article
          v-for="entry in marketVisiblePlugins"
          :key="entry.id"
          class="flex flex-wrap items-center gap-3 border-b border-light px-3 py-3 last:border-b-0 transition-colors hover:bg-soft/45"
        >
          <div class="h-10 w-10 shrink-0 rounded-md bg-soft text-primary flex-center">
            <span :class="marketIcon()" class="text-[17px]"></span>
          </div>
          <div class="min-w-[200px] flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <h2 class="m-0 text-[13px] font-bold text-default">{{ entry.name }}</h2>
              <span
                class="rounded px-1.5 py-0.5 text-[9px] font-bold"
                :class="entry.signature_ed25519 ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-300' : 'bg-amber-500/10 text-amber-600 dark:text-amber-300'"
              >
                {{ entry.signature_ed25519 ? "已签名" : "未签名" }}
              </span>
            </div>
            <p class="mt-1 truncate text-[10px] text-muted">{{ entry.description || entry.id }}</p>
          </div>
          <div class="min-w-[100px] text-right sm:text-left">
            <div class="text-[9px] font-semibold text-muted">版本</div>
            <div class="mt-1 font-mono text-[10px] font-bold text-secondary">v{{ entry.version }}</div>
          </div>
          <div class="ml-auto">
            <el-button size="small" type="primary" @click="installFromMarket(entry)">
              {{ installedVersion(entry) ? "更新" : "安装" }}
            </el-button>
          </div>
        </article>
      </div>
    </section>

    <el-dialog
      v-model="installDialog.visible"
      :title="installDialog.operation === 'install' ? '安装扩展' : `更新 ${installDialog.pluginName}`"
      width="560px"
      class="responsive-dialog"
    >
      <div class="space-y-4">
        <label class="block">
          <span class="mb-1 block text-[11px] font-bold text-secondary">安装方式</span>
          <el-segmented v-model="installDialog.method" :options="installMethodOptions" block :disabled="installDialog.operation === 'update'" />
        </label>

        <label v-if="installDialog.method === 'path'" class="block">
          <span class="mb-1 block text-[11px] font-bold text-secondary">服务端目录</span>
          <el-input v-model="installDialog.sourcePath" placeholder="examples/plugins/vue-app-template" />
        </label>

        <template v-else>
          <label
            class="group flex min-h-[76px] cursor-pointer items-center gap-3 rounded-lg border px-3.5 py-3 transition-[border-color,background-color,box-shadow] duration-200 focus-within:ring-2 focus-within:ring-primary/20"
            :class="
              installDialog.file
                ? 'border-primary/45 bg-primary/5 hover:border-primary/65'
                : 'border-dashed border-light bg-subtle/60 hover:border-primary/40 hover:bg-soft/70'
            "
          >
            <input
              type="file"
              accept=".zip,.tar,.tar.gz,.tgz"
              class="sr-only"
              @change="handleInstallFile"
            />
            <span
              class="h-10 w-10 shrink-0 rounded-lg flex-center transition-colors"
              :class="
                installDialog.file
                  ? 'bg-primary text-white'
                  : 'border border-light bg-card text-primary group-hover:border-primary/25'
              "
            >
              <span
                :class="installDialog.file ? 'i-ep-document-checked' : 'i-ep-upload-filled'"
                class="text-[17px]"
              ></span>
            </span>
            <span class="min-w-0 flex-1">
              <span
                class="block truncate text-[12px] font-bold"
                :class="installDialog.file ? 'text-default' : 'text-secondary'"
              >
                {{ installDialog.file?.name || "选择扩展安装包" }}
              </span>
              <span class="mt-1 block text-[10px] leading-4 text-muted">
                {{
                  installDialog.file
                    ? "安装包已就绪，点击此处可重新选择"
                    : "支持 .zip、.tar、.tar.gz 和 .tgz 格式"
                }}
              </span>
            </span>
            <span
              class="shrink-0 rounded-md border border-light bg-card px-2.5 py-1.5 text-[10px] font-bold text-secondary shadow-sm transition-colors group-hover:border-primary/25 group-hover:text-primary"
            >
              {{ installDialog.file ? "更换文件" : "浏览文件" }}
            </span>
          </label>
          <el-input v-model="installDialog.checksumSha256" placeholder="SHA-256 校验值（可选）" />
        </template>

        <el-checkbox v-if="installDialog.operation === 'install'" v-model="installDialog.enable">安装后立即启用</el-checkbox>
      </div>
      <template #footer>
        <el-button @click="installDialog.visible = false">取消</el-button>
        <el-button type="primary" :loading="installDialog.submitting" @click="submitInstallDialog">
          {{ installDialog.operation === "install" ? "安装" : "更新" }}
        </el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="marketSourcesDialogVisible" title="插件发布源" width="680px" class="responsive-dialog">
      <div class="space-y-3">
        <div class="grid gap-2 sm:grid-cols-[160px_minmax(0,1fr)_auto]">
          <el-input v-model="market.draftName" placeholder="名称" />
          <el-input v-model="market.draftUrl" placeholder="https://example.com/plugins/index.json" />
          <el-button @click="addMarketSource">添加</el-button>
        </div>
        <div class="overflow-hidden rounded-md border border-light">
          <div
            v-for="source in market.sources"
            :key="source.url"
            class="flex items-center gap-2 border-b border-light px-3 py-2.5 last:border-b-0"
          >
            <el-checkbox v-model="source.enabled" />
            <div class="min-w-0 flex-1">
              <div class="truncate text-[11px] font-bold text-default">{{ source.name || "未命名发布源" }}</div>
              <div class="mt-0.5 truncate font-mono text-[9px] text-muted">{{ source.url }}</div>
            </div>
            <button type="button" class="h-8 w-8 rounded-md text-rose-600 flex-center hover:bg-rose-50 dark:text-rose-300 dark:hover:bg-rose-950/20" title="删除" @click="removeMarketSource(source)">
              <span class="i-ep-delete"></span>
            </button>
          </div>
          <div v-if="!market.sources.length" class="px-3 py-8 text-center text-[11px] text-muted">暂无发布源</div>
        </div>
      </div>
      <template #footer>
        <el-button @click="marketSourcesDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="market.sourcesSaving" @click="saveMarketSources">保存</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="historyDialog.visible" :title="`${historyDialog.pluginName} 版本历史`" width="620px" class="responsive-dialog">
      <div v-if="historyDialog.loading" class="h-28 flex-center text-[11px] font-semibold text-muted">正在读取历史版本...</div>
      <div v-else-if="!historyDialog.versions.length" class="h-28 flex-center text-[11px] font-semibold text-muted">暂无历史版本</div>
      <div v-else class="overflow-hidden rounded-md border border-light">
        <div
          v-for="version in historyDialog.versions"
          :key="version.id"
          class="flex items-center gap-3 border-b border-light px-3 py-2.5 last:border-b-0"
        >
          <div class="min-w-0 flex-1">
            <div class="font-mono text-[11px] font-bold text-default">v{{ version.version }}</div>
            <div class="mt-0.5 text-[9px] text-muted">{{ formatTime(version.archived_at) }}</div>
          </div>
          <el-button size="small" @click="rollbackVersion(version)">回滚</el-button>
        </div>
      </div>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { useExtensionManager } from "../composables/useExtensionManager";

const {
  views, installMethodOptions, router, appStore, workspace, pluginApps, pluginThemes, activeView,
  searchQuery, statusFilter, loading, busyPlugin, installedPluginRecords, pluginHealth, marketSourcesDialogVisible, market,
  installDialog, historyDialog, allPlugins, capabilityLabel, visibleCapabilities, normalizedSearch, visiblePlugins, marketVisiblePlugins,
  marketVisibleUpdates, enabledCount, appCount, themeCount, themeSwatches, healthByPlugin, pluginHealthReport, pluginIcon,
  marketIcon, healthText, healthTone, formatTime, loadPlugins, loadMarketSources, loadAll, addMarketSource,
  removeMarketSource, saveMarketSources, loadMarket, checkMarketUpdates, installedVersion, impactSummary, confirmPreview, openInstall,
  openUpdate, handleInstallFile, uploadForm, submitInstallDialog, togglePlugin, removePlugin, handlePluginCommand, openHistory,
  rollbackVersion, installFromMarket, openPluginApp,
} = useExtensionManager();
</script>
