<template>
  <ResponsiveDialog
    v-model:visible="visible"
    title="获取资源"
    desktop-size="md"
    content-preset="form"
    mobile-mode="fullscreen"
    destroy-on-close
    append-to-body
  >
    <div class="h-full">
      <ShareImportWizard ref="wizardRef" @success="emit('success')" />
    </div>
  </ResponsiveDialog>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import ResponsiveDialog from "../../../../components/common/ResponsiveDialog.vue";
import ShareImportWizard from "./ShareImportWizard.vue";

type ShareImportWizardExpose = {
  setImportUrl: (url: string, isReimport: boolean) => void;
};

const props = defineProps<{
  modelValue: boolean;
}>();

const emit = defineEmits<{
  (event: "success"): void;
  (event: "update:modelValue", visible: boolean): void;
}>();

const wizardRef = ref<ShareImportWizardExpose | null>(null);

const visible = computed({
  get: () => props.modelValue,
  set: (value: boolean) => emit("update:modelValue", value),
});

const setImportUrl = (url: string, isReimport: boolean) => {
  wizardRef.value?.setImportUrl(url, isReimport);
};

defineExpose({
  setImportUrl,
});
</script>
