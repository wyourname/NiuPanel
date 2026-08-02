<template>
  <ResponsiveDialog
    :visible="visible"
    title="扩展影响预览"
    desktop-size="md"
    content-preset="list"
    mobile-mode="fullscreen"
    append-to-body
    destroy-on-close
    @update:visible="handleVisibleChange"
  >
    <ExtensionImpactPreview :preview="preview" />

    <template #footer>
      <template v-if="preview.install_allowed">
        <el-button @click="emit('cancel')">取消</el-button>
        <el-button type="primary" @click="emit('confirm')">
          {{ preview.operation === "update" ? "继续更新" : "继续安装" }}
        </el-button>
      </template>
      <el-button v-else type="primary" @click="emit('cancel')">关闭</el-button>
    </template>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import ResponsiveDialog from "@/components/common/ResponsiveDialog.vue";
import type { PluginImpactPreview } from "@/types";
import ExtensionImpactPreview from "./ExtensionImpactPreview.vue";

defineProps<{
  preview: PluginImpactPreview;
  visible: boolean;
}>();

const emit = defineEmits<{
  (event: "cancel"): void;
  (event: "confirm"): void;
  (event: "update:visible", visible: boolean): void;
}>();

const handleVisibleChange = (visible: boolean) => {
  emit("update:visible", visible);
  if (!visible) emit("cancel");
};
</script>
