<template>
  <ResponsiveDialog
    :visible="installDialog.visible"
    :title="installDialog.operation === 'install' ? '安装扩展' : `更新 ${installDialog.pluginName}`"
    desktop-size="md"
    content-preset="form"
    size="82%"
    append-to-body
    @update:visible="handleInstallDialogVisible"
  >
    <div class="space-y-4">
      <label class="block">
        <span class="mb-1 block text-[11px] font-bold text-secondary">安装方式</span>
        <el-segmented
          v-model="installDialog.method"
          :options="installMethodOptions"
          block
          :disabled="installDialog.operation === 'update'"
        />
      </label>

      <label v-if="installDialog.method === 'path'" class="block">
        <span class="mb-1 block text-[11px] font-bold text-secondary">服务端目录</span>
        <el-input
          v-model="installDialog.sourcePath"
          placeholder="examples/plugins/vue-app-template"
        />
      </label>

      <template v-else>
        <label
          class="group flex min-h-[76px] cursor-pointer items-center gap-3 rounded-lg border px-3.5 py-3 transition-[border-color,background-color,box-shadow] duration-200 focus-within:ring-2 focus-within:ring-primary/20"
          :class="installDialog.file
            ? 'border-primary/45 bg-primary/5 hover:border-primary/65'
            : 'border-dashed border-light bg-subtle/60 hover:border-primary/40 hover:bg-soft/70'"
        >
          <input
            type="file"
            accept=".zip,.tar,.tar.gz,.tgz"
            class="sr-only"
            @change="handleInstallFile"
          />
          <span
            class="h-10 w-10 shrink-0 rounded-lg flex-center transition-colors"
            :class="installDialog.file
              ? 'bg-primary text-white'
              : 'border border-light bg-card text-primary group-hover:border-primary/25'"
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
        <el-input
          v-model="installDialog.checksumSha256"
          placeholder="SHA-256 校验值（可选）"
        />
        <div
          v-if="installDialog.submitting && uploadProgress > 0"
          class="rounded-md border border-light bg-subtle/60 px-3 py-2.5"
          role="status"
          aria-live="polite"
        >
          <div class="mb-1.5 flex items-center justify-between gap-3 text-[11px]">
            <span class="truncate font-semibold text-default">
              {{ uploading ? "正在上传" : "上传完成" }}
            </span>
            <span class="shrink-0 font-mono text-secondary">{{ uploadProgress }}%</span>
          </div>
          <el-progress :percentage="uploadProgress" :show-text="false" :stroke-width="6" />
          <div class="mt-1.5 flex items-center justify-between gap-3 text-[10px] text-muted">
            <span>
              {{ formatFileSize(uploadLoadedBytes) }} /
              {{ formatFileSize(uploadTotalBytes || installDialog.file?.size || 0) }}
            </span>
            <el-button
              v-if="uploading"
              text
              circle
              size="small"
              title="取消上传"
              aria-label="取消上传"
              @click="cancelPluginUpload"
            >
              <span class="i-ep-close"></span>
            </el-button>
          </div>
        </div>
      </template>

      <el-checkbox
        v-if="installDialog.operation === 'install'"
        v-model="installDialog.enable"
      >
        安装后立即启用
      </el-checkbox>
    </div>
    <template #footer>
      <el-button @click="closeInstallDialog">取消</el-button>
      <el-button
        type="primary"
        :loading="installDialog.submitting"
        @click="submitInstallDialog"
      >
        {{ installDialog.operation === "install" ? "安装" : "更新" }}
      </el-button>
    </template>
  </ResponsiveDialog>

  <ResponsiveDialog
    v-model:visible="marketSourcesDialogVisible"
    title="插件发布源"
    desktop-size="lg"
    content-preset="list"
    size="86%"
    append-to-body
  >
    <div class="space-y-3">
      <div class="grid gap-2 md:grid-cols-[160px_minmax(0,1fr)_auto]">
        <el-input v-model="market.draftName" placeholder="名称" />
        <el-input
          v-model="market.draftUrl"
          placeholder="https://example.com/plugins/index.json"
        />
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
            <div class="truncate text-[11px] font-bold text-default">
              {{ source.name || "未命名发布源" }}
            </div>
            <div class="mt-0.5 truncate font-mono text-[9px] text-muted">
              {{ source.url }}
            </div>
          </div>
          <button
            type="button"
            class="h-11 w-11 rounded-md text-rose-600 flex-center hover:bg-rose-50 dark:text-rose-300 dark:hover:bg-rose-950/20 md:h-8 md:w-8"
            title="删除"
            @click="removeMarketSource(source)"
          >
            <span class="i-ep-delete"></span>
          </button>
        </div>
        <div
          v-if="!market.sources.length"
          class="px-3 py-8 text-center text-[11px] text-muted"
        >
          暂无发布源
        </div>
      </div>
    </div>
    <template #footer>
      <el-button @click="marketSourcesDialogVisible = false">取消</el-button>
      <el-button type="primary" :loading="market.sourcesSaving" @click="saveMarketSources">
        保存
      </el-button>
    </template>
  </ResponsiveDialog>

  <ResponsiveDialog
    v-model:visible="historyDialog.visible"
    :title="`${historyDialog.pluginName} 版本历史`"
    desktop-size="lg"
    content-preset="list"
    size="82%"
    append-to-body
  >
    <div
      v-if="historyDialog.loading"
      class="h-28 flex-center text-[11px] font-semibold text-muted"
    >
      正在读取历史版本...
    </div>
    <div
      v-else-if="!historyDialog.versions.length"
      class="h-28 flex-center text-[11px] font-semibold text-muted"
    >
      暂无历史版本
    </div>
    <div v-else class="overflow-y-auto rounded-md border border-light">
      <div
        v-for="version in historyDialog.versions"
        :key="version.id"
        class="flex items-center gap-3 border-b border-light px-3 py-2.5 last:border-b-0"
      >
        <div class="min-w-0 flex-1">
          <div class="font-mono text-[11px] font-bold text-default">
            v{{ version.version }}
          </div>
          <div class="mt-0.5 text-[9px] text-muted">
            {{ formatTime(version.archived_at) }}
          </div>
        </div>
        <el-button size="small" @click="rollbackVersion(version)">回滚</el-button>
      </div>
    </div>
  </ResponsiveDialog>

  <ExtensionImpactPreviewDialog
    v-if="impactDialog.preview"
    :visible="impactDialog.visible"
    :preview="impactDialog.preview"
    @cancel="resolveImpactPreview(false)"
    @confirm="resolveImpactPreview(true)"
    @update:visible="handleImpactDialogVisible"
  />
</template>

<script setup lang="ts">
import ResponsiveDialog from "@/components/common/ResponsiveDialog.vue";
import { useExtensionManager } from "../composables/useExtensionManager";
import ExtensionImpactPreviewDialog from "./ExtensionImpactPreviewDialog.vue";

const props = defineProps<{
  manager: ReturnType<typeof useExtensionManager>;
}>();

const {
  installMethodOptions,
  installDialog,
  historyDialog,
  impactDialog,
  marketSourcesDialogVisible,
  market,
  addMarketSource,
  removeMarketSource,
  saveMarketSources,
  resolveImpactPreview,
  handleImpactDialogVisible,
  handleInstallFile,
  submitInstallDialog,
  rollbackVersion,
  uploading,
  uploadProgress,
  uploadLoadedBytes,
  uploadTotalBytes,
  formatFileSize,
  cancelPluginUpload,
  closeInstallDialog,
  handleInstallDialogVisible,
  formatTime,
} = props.manager;
</script>
