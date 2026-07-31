import type {
  TaskBulkCommand,
  TaskBulkMoreCommand,
} from "./taskPageTypes";

type UseTaskBulkCommandRouterOptions = {
  handleBulkDelete: () => void;
  handleBulkDisable: () => unknown;
  handleBulkEnable: () => unknown;
  handleBulkPin: () => unknown;
  handleBulkResume: () => unknown;
  handleBulkShare: () => unknown;
  handleBulkStop: () => unknown;
  handleBulkUnpin: () => unknown;
};

export function useTaskBulkCommandRouter({
  handleBulkDelete,
  handleBulkDisable,
  handleBulkEnable,
  handleBulkPin,
  handleBulkResume,
  handleBulkShare,
  handleBulkStop,
  handleBulkUnpin,
}: UseTaskBulkCommandRouterOptions) {
  const bulkHandlers: Record<TaskBulkCommand, () => unknown> = {
    disable: handleBulkDisable,
    enable: handleBulkEnable,
    pin: handleBulkPin,
    resume: handleBulkResume,
    share: handleBulkShare,
    stop: handleBulkStop,
    unpin: handleBulkUnpin,
  };

  const bulkMoreHandlers: Record<TaskBulkMoreCommand, () => unknown> = {
    delete: handleBulkDelete,
    resume: handleBulkResume,
    unpin: handleBulkUnpin,
  };

  const handleBulkMoreCommand = (command: TaskBulkMoreCommand) => {
    bulkMoreHandlers[command]();
  };

  const handleBulkCommand = (command: TaskBulkCommand) => {
    bulkHandlers[command]();
  };

  return {
    handleBulkCommand,
    handleBulkMoreCommand,
  };
}
