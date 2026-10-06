import { ref, type Ref } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance } from 'element-plus'
import * as fileManagerApi from '../api/file_manager'
import { useFileClipboard } from './file/useFileClipboard'
import { useFileListState } from './file/useFileListState'
import { useFileTransfers } from './file/useFileTransfers'
import { useFileEditorSession } from './file/useFileEditorSession'
import {
  getRenamedPath,
  joinDirectoryPath,
  normalizeMoveTargetPath,
} from './file/fileOperationUtils'
import type { FileItem, FileTableRef } from './file/fileOperationTypes'

export type {
  Breadcrumb,
  FileItem,
  FileTableRef,
} from './file/fileOperationTypes'

export function useFileOperations(fileTableRef: Ref<FileTableRef | null>) {
  // Dialog States
  const createDialogVisible = ref(false)
  const createType = ref<'file' | 'directory'>('file')
  const creating = ref(false)
  const createForm = ref({ name: '' })

  const renameDialogVisible = ref(false)
  const renaming = ref(false)
  const renameForm = ref({ oldPath: '', oldName: '', newName: '' })

  const moveDialogVisible = ref(false)
  const movingFile = ref(false)
  const moveForm = ref<{ targetPath: string; items: FileItem[] }>({ targetPath: '', items: [] })

  const editorSession = useFileEditorSession()
  const deletingFiles = ref(false)

  // Download URL Dialog
  const downloadUrlDialogVisible = ref(false)
  const downloadingUrl = ref(false)
  const downloadUrlForm = ref({ url: '', filename: '' })

  const listState = useFileListState(fileTableRef)
  const {
    collapsedBreadcrumbs,
    currentPath,
    fileList,
    filteredFileList,
    goUp,
    handleSelectAll,
    handleSelectionChange,
    isSelected,
    loadContents,
    loadNode,
    loading,
    listError,
    retryLoad,
    navigate,
    searchQuery,
    selectedFiles,
    toggleSelection,
    clearSelection,
  } = listState

  const {
    clipboard,
    copyToClipboard,
    cutToClipboard,
    pasteFromClipboard,
    pasting,
    clearClipboard,
  } = useFileClipboard({
    clearSelection,
    currentPath,
    loadContents,
  })

  const {
    handleBatchDownload,
    handleDownload,
    extractArchive,
    imagePreviewVisible,
    imageUrl,
    performUpload,
    previewImage,
  } = useFileTransfers({
    currentPath,
    loading,
    loadContents,
  })

  // CRUD
  const deleteItem = async (item: FileItem) => {
    try {
      await ElMessageBox.confirm(`确定删除 ${item.name}?`, '警告', { type: 'warning' })
      await fileManagerApi.deleteItem(item.path)
      ElMessage.success('删除成功')
      loadContents(currentPath.value)
    } catch (e) { }
  }

  const batchDelete = async () => {
    if (deletingFiles.value || selectedFiles.value.length === 0) return
    const files = [...selectedFiles.value]
    const sourcePath = currentPath.value
    deletingFiles.value = true
    try {
      try {
        await ElMessageBox.confirm(`确定删除选中的 ${files.length} 项?`, '删除文件', {
          type: 'warning', confirmButtonText: '删除', cancelButtonText: '取消',
        })
      } catch { return }

      const deleted = new Set<string>()
      for (const file of files) {
        try {
          await fileManagerApi.deleteItem(file.path)
          deleted.add(file.path)
        } catch { /* Keep failed items selected for retry. */ }
      }
      if (deleted.size) {
        fileList.value = fileList.value.filter(file => !deleted.has(file.path))
        selectedFiles.value = selectedFiles.value.filter(file => !deleted.has(file.path))
        ElMessage.success(`已删除 ${deleted.size} 项`)
        if (currentPath.value === sourcePath && !loading.value) await loadContents(sourcePath)
      }
      const failedCount = files.length - deleted.size
      if (failedCount) ElMessage.warning(`${failedCount} 项删除失败，可重试`)
    } finally {
      deletingFiles.value = false
    }
  }

  const handleCreateItem = async (formRef: FormInstance | null) => {
    if (!formRef) return
    try {
      const valid = await formRef.validate()
      if (!valid) return
      creating.value = true
      const fullPath = joinDirectoryPath(currentPath.value, createForm.value.name)
      createType.value === 'file' ? await fileManagerApi.createFile(fullPath) : await fileManagerApi.createDirectory(fullPath)
      ElMessage.success('创建成功')
      createDialogVisible.value = false
      loadContents(currentPath.value)
    } finally { creating.value = false }
  }

  const handleRenameItem = async (formRef: FormInstance | null) => {
    if (!formRef) return
    try {
      const valid = await formRef.validate()
      if (!valid) return
      renaming.value = true
      const newPath = getRenamedPath(renameForm.value.oldPath, renameForm.value.newName)
      await fileManagerApi.renameItem(renameForm.value.oldPath, newPath)
      ElMessage.success('重命名成功')
      renameDialogVisible.value = false
      loadContents(currentPath.value)
    } finally { renaming.value = false }
  }

  const showRenameDialog = (item: FileItem) => {
    renameForm.value = { oldPath: item.path, oldName: item.name, newName: item.name }
    renameDialogVisible.value = true
  }

  const showMoveDialog = (items: FileItem[]) => {
    moveForm.value = { targetPath: currentPath.value, items: items.map(i => ({ ...i })) }
    moveDialogVisible.value = true
  }

  const executeMove = async (formRef: FormInstance | null) => {
    if (!formRef) return
    try {
      const valid = await formRef.validate()
      if (!valid) return
      movingFile.value = true

      let successCount = 0
      let failCount = 0

      for (const item of moveForm.value.items) {
        const targetPath = normalizeMoveTargetPath(moveForm.value.targetPath, item.name)
        if (targetPath === item.path) continue // Skip if moving to the same place

        try {
          await fileManagerApi.renameItem(item.path, targetPath)
          successCount++
        } catch (e) {
          failCount++
        }
      }

      if (successCount > 0) {
        ElMessage.success(`成功移动 ${successCount} 项`)
        moveDialogVisible.value = false
        loadContents(currentPath.value)
        clearSelection()
      }
      if (failCount > 0) {
        ElMessage.warning(`${failCount} 项移动失败`)
      }
    } finally {
      movingFile.value = false
    }
  }

  const copyDroppedFiles = async (items: FileItem[]) => {
    if (items.length === 0) return

    loading.value = true
    let successCount = 0
    let failCount = 0

    try {
      for (const item of items) {
        const targetPath = joinDirectoryPath(currentPath.value, item.name)
        if (targetPath === item.path) {
          failCount++
          continue
        }

        try {
          await fileManagerApi.copyItem(item.path, targetPath)
          successCount++
        } catch {
          failCount++
        }
      }

      if (successCount > 0) {
        ElMessage.success(`成功复制 ${successCount} 项`)
        await loadContents(currentPath.value)
      }
      if (failCount > 0) ElMessage.warning(`${failCount} 项复制失败`)
    } finally {
      loading.value = false
    }
  }

  const handleDownloadFromUrl = async (formRef: FormInstance | null) => {
    if (!formRef) return
    try {
      const valid = await formRef.validate()
      if (!valid) return
      downloadingUrl.value = true
      await fileManagerApi.downloadFromUrl(
        downloadUrlForm.value.url,
        currentPath.value,
        downloadUrlForm.value.filename || undefined
      )
      ElMessage.success('下载成功')
      downloadUrlDialogVisible.value = false
      loadContents(currentPath.value)
    } catch (e) {
    } finally {
      downloadingUrl.value = false
    }
  }

  return {
    loading, listError, retryLoad, fileList, currentPath, selectedFiles,
    searchQuery, filteredFileList,
    clipboard, pasting, clearClipboard,
    createDialogVisible, createType, creating, createForm,
    renameDialogVisible, renaming, renameForm,
    ...editorSession,
    imagePreviewVisible, imageUrl,
    collapsedBreadcrumbs,
    downloadUrlDialogVisible, downloadingUrl, downloadUrlForm,
    loadContents, navigate, goUp,
    handleSelectionChange, toggleSelection, isSelected, clearSelection, handleSelectAll,
    copyToClipboard, cutToClipboard, pasteFromClipboard,
    deleteItem, batchDelete, deletingFiles,
    handleCreateItem, handleRenameItem,
    performUpload, handleDownload, handleBatchDownload, extractArchive, previewImage,
    showRenameDialog,
    moveDialogVisible,
    movingFile,
    moveForm,
    showMoveDialog,
    executeMove,
    copyDroppedFiles,
    handleDownloadFromUrl,
    loadNode,
  }
}
