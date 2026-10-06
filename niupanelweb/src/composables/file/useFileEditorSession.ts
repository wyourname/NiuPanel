import { computed, onScopeDispose, ref } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import * as files from '@/api/file_manager'
import type { FileItem } from './fileOperationTypes'

export function useFileEditorSession() {
  const editFileDialogVisible = ref(false)
  const currentFile = ref<FileItem | null>(null)
  const fileContent = ref('')
  const savedFileContent = ref('')
  const loadingFile = ref(false)
  const fileLoadError = ref('')
  const savingFile = ref(false)
  const fileIsDirty = computed(() => !loadingFile.value && !fileLoadError.value && fileContent.value !== savedFileContent.value)
  let readVersion = 0
  let closing: Promise<boolean> | null = null

  const loadFileContent = async () => {
    const file = currentFile.value
    if (!file) return
    const version = ++readVersion
    loadingFile.value = true
    fileLoadError.value = ''
    try {
      const response = await files.readFileContent(file.path)
      if (version !== readVersion) return
      fileContent.value = response.data ?? ''
      savedFileContent.value = fileContent.value
    } catch {
      if (version === readVersion) fileLoadError.value = '文件读取失败，请重试。'
    } finally {
      if (version === readVersion) loadingFile.value = false
    }
  }

  const saveFileContent = async (): Promise<boolean> => {
    const file = currentFile.value
    if (!file || loadingFile.value || fileLoadError.value || savingFile.value) return false
    if (!fileIsDirty.value) return true
    // Keep the submitted snapshot: typing while a save is in flight must remain dirty.
    const content = fileContent.value
    savingFile.value = true
    try {
      await files.writeFileContent(file.path, content)
      savedFileContent.value = content
      ElMessage.success('保存成功')
      return true
    } catch {
      return false
    } finally {
      savingFile.value = false
    }
  }

  const confirmCloseEditor = (): Promise<boolean> => {
    if (closing) return closing
    closing = (async () => {
      if (savingFile.value) {
        ElMessage.info('文件正在保存，请稍候')
        return false
      }
      if (!fileIsDirty.value) return true
      try {
        await ElMessageBox.confirm(`“${currentFile.value?.name}”有未保存的修改，是否保存后关闭？`, '保存文件', {
          confirmButtonText: '保存并关闭', cancelButtonText: '放弃修改',
          distinguishCancelAndClose: true, closeOnClickModal: false,
          customClass: 'file-editor-unsaved-message',
        })
        return await saveFileContent() && !fileIsDirty.value
      } catch (action) {
        return action === 'cancel'
      }
    })().finally(() => { closing = null })
    return closing
  }

  const closeFileEditor = async () => {
    if (!await confirmCloseEditor()) return false
    ++readVersion
    editFileDialogVisible.value = false
    return true
  }

  const showEditFileDialog = async (file: FileItem) => {
    if (editFileDialogVisible.value && !await confirmCloseEditor()) return
    currentFile.value = file
    fileContent.value = ''
    savedFileContent.value = ''
    editFileDialogVisible.value = true
    await loadFileContent()
  }

  const beforeUnload = (event: BeforeUnloadEvent) => {
    if (!editFileDialogVisible.value || !fileIsDirty.value) return
    event.preventDefault()
    event.returnValue = ''
  }
  window.addEventListener('beforeunload', beforeUnload)
  onScopeDispose(() => {
    ++readVersion
    window.removeEventListener('beforeunload', beforeUnload)
  })

  return { editFileDialogVisible, currentFile, fileContent, savedFileContent, loadingFile,
    fileLoadError, savingFile, fileIsDirty, loadFileContent, showEditFileDialog,
    saveFileContent, confirmCloseEditor, closeFileEditor }
}
