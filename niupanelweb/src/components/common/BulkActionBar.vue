<template>
  <transition name="el-fade-in-linear">
    <div
      v-if="count > 0"
      class="transition-colors"
      :class="[
        appStore.isMobile
          ? 'fixed bottom-[calc(var(--mobile-dock-clearance)+8px)] left-2 right-2 z-100 rounded-lg border border-light bg-card px-3 py-2 shadow-md flex flex-col'
          : 'relative z-10 shrink-0 border-b border-[var(--accent-subtle-border)] bg-[var(--accent-subtle-bg)] px-4 py-2 flex items-center justify-between gap-3',
      ]"
    >
      <!-- Mobile Header -->
      <div
        v-if="appStore.isMobile"
        class="mb-1.5 flex items-center justify-between gap-3"
      >
        <div class="flex items-center gap-2">
          <span class="text-[12px] font-bold text-default"
            >{{ t("common.selected") }} {{ count }}
            {{ t("common.items") }}</span
          >
        </div>
        <div class="flex items-center gap-2">
          <el-button
            v-if="showSelectAll"
            link
            type="primary"
            class="!text-[12px]"
            @click="emit('select-all')"
          >
            {{
              isAllSelected ? t("common.deselect_all") : t("common.select_all")
            }}
          </el-button>
          <div v-if="showSelectAll" class="w-px h-3 bg-base"></div>
          <el-button
            link
            type="primary"
            class="!text-[12px] font-bold"
            @click="emit('cancel')"
            >{{ t("common.cancel") }}</el-button
          >
        </div>
      </div>

      <!-- Desktop Counter -->
      <div v-else class="mr-1 flex shrink-0 items-center gap-1.5">
        <span class="text-[12px] font-bold text-[var(--accent-subtle-text)]">{{ count }}</span>
        <span class="text-[11px] text-muted">{{ t("common.selected") }}</span>

        <template v-if="showSelectAll">
          <div class="mx-1 h-4 w-px bg-light"></div>
          <el-button link type="primary" size="small" @click="emit('select-all')">
            {{ isAllSelected ? t("common.deselect_all") : t("common.select_all") }}
          </el-button>
        </template>

        <div class="mx-1 h-4 w-px bg-light"></div>
        <el-button link type="info" size="small" @click="emit('cancel')">{{
          t("common.cancel")
        }}</el-button>
      </div>

      <!-- Actions Area -->
      <div
        class="flex items-center"
        :class="appStore.isMobile ? 'justify-between w-full' : 'gap-2'"
      >
        <div class="flex items-center gap-1">
          <slot name="actions" />
        </div>

        <div
          v-if="$slots.more || showDelete"
          class="w-px"
          :class="
            appStore.isMobile ? 'h-6 bg-light' : 'h-4 bg-light'
          "
        ></div>

        <el-dropdown
          v-if="$slots.more"
          trigger="click"
          @command="handleCommand"
        >
          <el-button
            link
            type="primary"
            :size="appStore.isMobile ? 'default' : 'small'"
            class="!border-none !px-2"
          >
            <div class="flex items-center">
              <span v-if="!appStore.isMobile">{{ t("common.more") }}</span>
              <div v-else class="i-ep-more-filled text-lg"></div>
              <div v-if="!appStore.isMobile" class="i-ep-arrow-down ml-1"></div>
            </div>
          </el-button>
          <template #dropdown>
            <el-dropdown-menu>
              <slot name="more" />
            </el-dropdown-menu>
          </template>
        </el-dropdown>

        <el-button
          v-if="showDelete"
          type="danger"
          :circle="appStore.isMobile"
          :plain="!appStore.isMobile"
          :size="appStore.isMobile ? 'default' : 'small'"
          @click="emit('delete')"
        >
          <div
            class="i-ep-delete"
            :class="!appStore.isMobile ? 'mr-1' : 'text-lg'"
          ></div>
          <template v-if="!appStore.isMobile">{{
            t("common.delete")
          }}</template>
        </el-button>
      </div>
    </div>
  </transition>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { useAppStore } from "../../stores/app";

const { t } = useI18n();

withDefaults(
  defineProps<{
    count?: number;
    isAllSelected?: boolean;
    showDelete?: boolean;
    showSelectAll?: boolean;
  }>(),
  {
    count: 0,
    isAllSelected: false,
    showDelete: true,
    showSelectAll: false,
  },
);

const emit = defineEmits<{
  (event: "cancel"): void;
  (event: "command", command: string): void;
  (event: "delete"): void;
  (event: "select-all"): void;
}>();
const appStore = useAppStore();

defineSlots<{
  actions?: () => unknown;
  more?: () => unknown;
}>();

const handleCommand = (command: unknown) => {
  if (typeof command === "string") {
    emit("command", command);
  }
};
</script>
