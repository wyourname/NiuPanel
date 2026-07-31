<template>
  <div v-if="visible" class="command-palette-overlay" @click.self="close">
    <div class="command-palette-container">
      <CommandPaletteSearchBar
        ref="searchBarRef"
        v-model="searchQuery"
        @close="close"
        @navigate-down="navigateDown"
        @navigate-up="navigateUp"
        @select-active="selectActiveItem"
      />

      <CommandPaletteResults
        :active-index="activeIndex"
        :items="filteredResults"
        :search-query="searchQuery"
        @hover="activeIndex = $event"
        @select="selectItem"
      />

      <CommandPaletteFooter />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, toRef } from "vue";
import CommandPaletteFooter from "./commandPalette/CommandPaletteFooter.vue";
import CommandPaletteResults from "./commandPalette/CommandPaletteResults.vue";
import CommandPaletteSearchBar from "./commandPalette/CommandPaletteSearchBar.vue";
import { useCommandPaletteController } from "./commandPalette/useCommandPaletteController";
import { useCommandPaletteData } from "./commandPalette/useCommandPaletteData";

const props = defineProps<{
  visible: boolean;
}>();

const emit = defineEmits<{
  (event: "update:visible", value: boolean): void;
}>();

type CommandPaletteSearchBarRef = {
  focus: () => void;
};

const searchBarRef = ref<CommandPaletteSearchBarRef | null>(null);

const close = () => {
  emit("update:visible", false);
};

const {
  environments,
  loadPaletteData,
  tasks,
  variables,
} = useCommandPaletteData();

const {
  activeIndex,
  filteredResults,
  navigateDown,
  navigateUp,
  searchQuery,
  selectActiveItem,
  selectItem,
} = useCommandPaletteController({
  close,
  environments,
  focusSearchInput: () => searchBarRef.value?.focus(),
  loadPaletteData,
  tasks,
  variables,
  visible: toRef(props, "visible"),
});
</script>

<style scoped>
.command-palette-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(4px);
  z-index: 2000;
  display: flex;
  justify-content: center;
  align-items: flex-start;
  padding-top: 15vh;
}

@media (max-width: 768px) {
  .command-palette-overlay {
    padding-top: 0;
    background: var(--bg-base);
  }
}

.command-palette-container {
  width: 600px;
  max-width: 90%;
  background: var(--bg-card);
  border-radius: 12px;
  box-shadow:
    0 20px 25px -5px rgba(0, 0, 0, 0.1),
    0 10px 10px -5px rgba(0, 0, 0, 0.04);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid var(--border-base);
  animation: slide-down 0.2s ease-out;
}

@media (max-width: 768px) {
  .command-palette-container {
    width: 100%;
    max-width: 100%;
    height: 100%;
    border-radius: 0;
    border: none;
    animation: none;
  }
}

@keyframes slide-down {
  from {
    opacity: 0;
    transform: translateY(-10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

</style>
