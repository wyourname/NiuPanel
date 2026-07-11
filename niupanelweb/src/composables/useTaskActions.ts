import { useTaskStore } from '../stores/tasks'
import { useAppStore } from '../stores/app'
import type { Env, Task } from '@/types'
import { useTaskBulkActions } from './useTaskBulkActions'
import { useTaskDeleteConfirmation } from './useTaskDeleteConfirmation'
import { useTaskDialogState } from './useTaskDialogState'
import { useTaskScriptDialog } from './useTaskScriptDialog'
import { useTaskSelection } from './useTaskSelection'

export type { Task, Env }

export function useTaskActions() {
  const taskStore = useTaskStore()
  const appStore = useAppStore()

  const {
    selectedIds,
    selectedTasks,
    handleSelectAll,
    clearAllSelection,
    handleMobileSelection,
  } = useTaskSelection(taskStore)

  // Basic Actions
  const handleToggleEnable = (task: Task, val: boolean) => {
    taskStore.toggleEnable(task, val)
  }

  const {
    wizardVisible,
    editingTask,
    logVisible,
    currentLogTask,
    shareVisible,
    tasksToShare,
    variableEditorVisible,
    currentTaskForVariables,
    openCreate,
    openEdit,
    handleWizardSuccess,
    openLogs,
    openVariableEditor,
    handleVariableEditSuccess,
    openShare,
  } = useTaskDialogState({ appStore, taskStore })

  const {
    scriptEditorVisible,
    scriptEditorContent,
    scriptEditorLoading,
    currentScriptTask,
    isFileMode,
    handleEditScript,
    saveScript,
  } = useTaskScriptDialog({ appStore, taskStore })

  const {
    handleDelete,
    handleBulkDelete,
  } = useTaskDeleteConfirmation({
    clearAllSelection,
    selectedIds,
    taskStore,
  })

  const {
    handleBulkRun,
    handleBulkPause,
    handleBulkResume,
    handleBulkStop,
    handleBulkEnable,
    handleBulkDisable,
    handleBulkPin,
    handleBulkUnpin,
    handleBulkShare,
  } = useTaskBulkActions({
    selectedIds,
    selectedTasks,
    shareVisible,
    taskStore,
    tasksToShare,
  })

  return {
    selectedIds,
    selectedTasks,
    wizardVisible,
    editingTask,
    logVisible,
    currentLogTask,
    shareVisible,
    tasksToShare,
    scriptEditorVisible,
    scriptEditorContent,
    scriptEditorLoading,
    currentScriptTask,
    isFileMode,
    variableEditorVisible,
    currentTaskForVariables,
    handleSelectAll,
    clearAllSelection,
    handleMobileSelection,
    handleToggleEnable,
    openCreate,
    openEdit,
    handleWizardSuccess,
    openLogs,
    handleEditScript,
    saveScript,
    openVariableEditor,
    handleVariableEditSuccess,
    openShare,
    handleDelete,
    handleBulkDelete,
    handleBulkRun,
    handleBulkPause,
    handleBulkResume,
    handleBulkStop,
    handleBulkEnable,
    handleBulkDisable,
    handleBulkPin,
    handleBulkUnpin,
    handleBulkShare
  }
}
