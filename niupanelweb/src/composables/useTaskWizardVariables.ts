import { ref, watch } from "vue";
import {
  formatVariableListText,
  parseVariableBulkText,
  type TaskVariableItem,
} from "./taskWizardHelpers";

type TaskWizardVariableMode = "bulk" | "list";

export function useTaskWizardVariables() {
  const variableMode = ref<TaskWizardVariableMode>("bulk");
  const variablesBulk = ref("");
  const variablesList = ref<TaskVariableItem[]>([]);

  const setVariables = (items: TaskVariableItem[]) => {
    variablesList.value = [...items];
    variablesBulk.value = formatVariableListText(items);
  };

  const getSubmitVariables = () => {
    if (variableMode.value === "bulk") {
      return parseVariableBulkText(variablesBulk.value);
    }

    return variablesList.value.filter((item) => item.key);
  };

  watch(variableMode, (newMode) => {
    if (newMode === "list") {
      variablesList.value = parseVariableBulkText(variablesBulk.value);
    } else {
      variablesBulk.value = formatVariableListText(variablesList.value);
    }
  });

  return {
    getSubmitVariables,
    setVariables,
    variableMode,
    variablesBulk,
    variablesList,
  };
}
