import { computed, nextTick, ref, watch, type Ref } from "vue";
import { useRouter } from "vue-router";
import { storageKey as makeStorageKey } from "@/utils/storage";
import { commandItems, navItems } from "./staticItems";
import type { PaletteItem } from "./types";

type UseCommandPaletteControllerOptions = {
  close: () => void;
  environments: Ref<PaletteItem[]>;
  focusSearchInput: () => void;
  loadPaletteData: () => Promise<void>;
  tasks: Ref<PaletteItem[]>;
  variables: Ref<PaletteItem[]>;
  visible: Ref<boolean>;
};

const matchesPaletteQuery = (item: PaletteItem, query: string) => {
  return (
    item.title.toLowerCase().includes(query) ||
    (item.desc && item.desc.toLowerCase().includes(query))
  );
};

export function useCommandPaletteController({
  close,
  environments,
  focusSearchInput,
  loadPaletteData,
  tasks,
  variables,
  visible,
}: UseCommandPaletteControllerOptions) {
  const router = useRouter();
  const searchQuery = ref("");
  const activeIndex = ref(0);

  const filteredResults = computed(() => {
    if (!searchQuery.value) return [];

    const query = searchQuery.value.toLowerCase();
    const matches = (item: PaletteItem) => matchesPaletteQuery(item, query);

    return [
      ...navItems.filter(matches),
      ...tasks.value.filter(matches),
      ...variables.value.filter(matches),
      ...environments.value.filter(matches),
      ...commandItems.filter(matches),
    ].slice(0, 12);
  });

  watch(
    visible,
    (newVal) => {
      if (newVal) {
        searchQuery.value = "";
        activeIndex.value = 0;
        void loadPaletteData();
        void nextTick(() => {
          focusSearchInput();
        });
      }
    },
  );

  watch(searchQuery, () => {
    activeIndex.value = 0;
  });

  const navigateDown = () => {
    if (activeIndex.value < filteredResults.value.length - 1) {
      activeIndex.value++;
    }
  };

  const navigateUp = () => {
    if (activeIndex.value > 0) {
      activeIndex.value--;
    }
  };

  const runCommand = (item: PaletteItem) => {
    if (item.action === "logout") {
      localStorage.removeItem(makeStorageKey("user_info"));
      void router.push("/login");
    } else if (item.action === "refresh") {
      window.location.reload();
    } else if (item.action === "toggle_theme") {
      document.documentElement.classList.toggle("dark");
    }
  };

  const selectItem = (index = activeIndex.value) => {
    const item = filteredResults.value[index];
    if (!item) return;

    if (item.path) {
      void router.push({ path: item.path, query: item.query });
    } else if (item.type === "command") {
      runCommand(item);
    }

    close();
  };

  const selectActiveItem = () => {
    selectItem(activeIndex.value);
  };

  return {
    activeIndex,
    filteredResults,
    navigateDown,
    navigateUp,
    searchQuery,
    selectActiveItem,
    selectItem,
  };
}
