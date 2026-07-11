import { onMounted, onUnmounted, type Ref } from "vue";

type UseFileSaveShortcutOptions = {
  isEditing: Ref<boolean>;
  saveFileContent: () => void;
};

export function useFileSaveShortcut(options: UseFileSaveShortcutOptions) {
  const handleKeydown = (event: KeyboardEvent) => {
    if (!(event.metaKey || event.ctrlKey) || event.key !== "s") return;
    if (!options.isEditing.value) return;

    event.preventDefault();
    options.saveFileContent();
  };

  onMounted(() => {
    window.addEventListener("keydown", handleKeydown);
  });

  onUnmounted(() => {
    window.removeEventListener("keydown", handleKeydown);
  });
}
