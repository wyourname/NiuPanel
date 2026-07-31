import { computed, ref } from "vue";
import type { useHaptics } from "@/composables/useHaptics";
import {
  getPermissionGroup,
  type ApiPermissionNavGroupId,
} from "../utils/apiKeyPermissions";

type UseApiKeyPermissionsOptions = {
  haptics: ReturnType<typeof useHaptics>;
};

export function useApiKeyPermissions({
  haptics,
}: UseApiKeyPermissionsOptions) {
  const activeGroup = ref<ApiPermissionNavGroupId>("all");
  const selectedPerms = ref<string[]>([]);

  const currentGroupMeta = computed(() => getPermissionGroup(activeGroup.value));

  const isGroupAllSelected = computed(() => {
    const meta = currentGroupMeta.value;
    if (!meta) return false;
    return meta.perms.every((permission) =>
      selectedPerms.value.includes(permission.value),
    );
  });

  const isGroupIndeterminate = computed(() => {
    const meta = currentGroupMeta.value;
    if (!meta) return false;
    if (isGroupAllSelected.value) return false;
    const selectedCount = meta.perms.filter((permission) =>
      selectedPerms.value.includes(permission.value),
    ).length;
    return selectedCount > 0;
  });

  const handleGroupSelectAll = () => {
    haptics.impact();
    const meta = currentGroupMeta.value;
    if (!meta) return;

    const permValues = meta.perms.map((permission) => permission.value);
    if (isGroupAllSelected.value) {
      selectedPerms.value = selectedPerms.value.filter(
        (permission) => !permValues.includes(permission),
      );
      return;
    }

    const current = new Set(selectedPerms.value);
    permValues.forEach((permission) => current.add(permission));
    selectedPerms.value = Array.from(current);
  };

  const togglePerm = (permission: string) => {
    haptics.selectionChanged();
    const index = selectedPerms.value.indexOf(permission);
    if (index > -1) {
      selectedPerms.value.splice(index, 1);
      return;
    }
    selectedPerms.value.push(permission);
  };

  const toggleSuperUser = () => {
    haptics.notification();
    togglePermValue("*:*");
  };

  const togglePermValue = (permission: string) => {
    const index = selectedPerms.value.indexOf(permission);
    if (index > -1) {
      selectedPerms.value.splice(index, 1);
      return;
    }
    selectedPerms.value.push(permission);
  };

  return {
    activeGroup,
    currentGroupMeta,
    handleGroupSelectAll,
    isGroupAllSelected,
    isGroupIndeterminate,
    selectedPerms,
    togglePerm,
    toggleSuperUser,
  };
}
