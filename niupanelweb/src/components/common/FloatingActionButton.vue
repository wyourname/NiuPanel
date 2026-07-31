<template>
  <transition name="el-zoom-in-bottom">
    <button
      v-if="visible"
      class="fab-button"
      :class="[
        type === 'primary' ? 'bg-primary' :
        type === 'sky-blue' ? 'bg-sky-600' :
        `bg-${type}`,
        customClass,
      ]"
      @click="emit('click')"
    >
      <slot>
        <div :class="icon || 'i-ep-plus'" class="text-[26px]"></div>
      </slot>
    </button>
  </transition>
</template>

<script setup lang="ts">
type FloatingActionButtonProps = {
  visible?: boolean;
  icon?: string;
  type?: string;
  customClass?: string;
};

withDefaults(defineProps<FloatingActionButtonProps>(), {
  visible: true,
  icon: "",
  type: "primary",
  customClass: "",
});

const emit = defineEmits<{
  (event: "click"): void;
}>();
</script>

<style scoped>
.fab-button {
  position: fixed;
  right: 20px;
  bottom: calc(20px + env(safe-area-inset-bottom, 0px));
  /* Offset for bottom nav if present */
  margin-bottom: 64px;
  width: 52px;
  height: 52px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  border: none;
  cursor: pointer;
  z-index: 100;
  box-shadow: 0 8px 20px rgba(15, 23, 42, 0.18);
  transition: background-color 0.16s ease, box-shadow 0.16s ease;
}

.fab-button:active {
  box-shadow: 0 4px 12px rgba(15, 23, 42, 0.18);
}

/* Colors from UnoCSS/Theme */
.bg-primary {
  background-color: var(--el-color-primary);
}
.bg-success {
  background-color: var(--el-color-success);
}
.bg-warning {
  background-color: var(--el-color-warning);
}
.bg-danger {
  background-color: var(--el-color-danger);
}
.bg-info {
  background-color: var(--el-color-info);
}
</style>
