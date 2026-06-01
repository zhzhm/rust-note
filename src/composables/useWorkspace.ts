import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

export interface FileEntry {
  name: string
  path: string
  is_dir: boolean
  children?: FileEntry[] | null
}

export interface WebDavConfig {
  url: string
  username: string
  password: string
  base_path: string | null
}

export interface Settings {
  workspace_dir: string | null
  backend_type: string
  webdav: WebDavConfig | null
}

export function useWorkspace() {
  const settings = ref<Settings>({
    workspace_dir: null,
    backend_type: 'Local',
    webdav: null,
  })
  const workspaceDir = computed(() => settings.value.workspace_dir)
  const backendType = computed(() => settings.value.backend_type)
  const isWebDAV = computed(() => settings.value.backend_type === 'WebDAV')
  const isLocal = computed(() => settings.value.backend_type === 'Local')

  const workspaceLabel = computed(() => {
    if (isWebDAV.value && settings.value.webdav) {
      return settings.value.webdav.url
    }
    if (workspaceDir.value) {
      const parts = workspaceDir.value.replace(/\\/g, '/').split('/')
      return parts[parts.length - 1] || workspaceDir.value
    }
    return '项目文件'
  })

  const files = ref<FileEntry[]>([])
  const currentFilePath = ref<string | null>(null)
  const currentContent = ref('')
  const lastSavedContent = ref('')
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  const isDirty = computed(() => currentContent.value !== lastSavedContent.value)

  const demoFiles: FileEntry[] = [
    { name: '欢迎文档.adoc', path: '__demo__/欢迎文档.adoc', is_dir: false, children: null },
    { name: '学习笔记.adoc', path: '__demo__/学习笔记.adoc', is_dir: false, children: null },
  ]

  const demoContents: Record<string, string> = {
    '__demo__/欢迎文档.adoc': `== 欢迎使用 Asciidoc 编辑器

这是一个功能完整的 Asciidoc 编辑器，支持实时预览。

=== 功能特点

* 实时预览 - 编辑同时查看效果
* 多文件管理 - 轻松切换不同文档

=== 示例代码

[source,javascript]
----
function greeting(name) {
  return 'Hello, ' + name + '!'
}
----

[NOTE]
这是一条提示信息。

----
祝你写作愉快！
`,
    '__demo__/学习笔记.adoc': `== 学习笔记

[quote, 孔子]
====
学而不思则罔，思而不学则殆。
====

=== 学习计划

. 学习 Asciidoc 语法
. 掌握常用标记

[IMPORTANT]
请确保语法正确。
`,
  }

  function hasWorkspace(): boolean {
    if (isWebDAV.value && settings.value.webdav) return true
    if (isLocal.value && workspaceDir.value) return true
    return false
  }

  async function init() {
    try {
      const loadedSettings = await invoke<Settings>('load_settings')
      settings.value = loadedSettings

      if (hasWorkspace()) {
        await loadWorkspace()
        if (files.value.length > 0) {
          const firstFile = findFirstFile(files.value)
          if (firstFile) {
            await openFile(firstFile.path)
          }
        }
      } else {
        files.value = [...demoFiles]
        currentFilePath.value = demoFiles[0].path
        currentContent.value = demoContents[demoFiles[0].path] || ''
        lastSavedContent.value = currentContent.value
      }
    } catch (e) {
      error.value = `初始化失败: ${e}`
      files.value = [...demoFiles]
      currentFilePath.value = demoFiles[0].path
      currentContent.value = demoContents[demoFiles[0].path] || ''
      lastSavedContent.value = currentContent.value
    }
  }

  function findFirstFile(entries: FileEntry[]): FileEntry | null {
    for (const entry of entries) {
      if (!entry.is_dir) return entry
      if (entry.children && entry.children.length > 0) {
        const found = findFirstFile(entry.children)
        if (found) return found
      }
    }
    return null
  }

  async function pickAndSetDirectory() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: '选择本地仓库目录',
      })

      if (!selected) return

      const dirPath = selected as string
      const newSettings: Settings = {
        workspace_dir: dirPath,
        backend_type: 'Local',
        webdav: null,
      }
      await invoke('save_settings', { settings: newSettings })
      settings.value = newSettings
      await loadWorkspace()

      if (files.value.length > 0) {
        const firstFile = findFirstFile(files.value)
        if (firstFile) {
          await openFile(firstFile.path)
        }
      }
    } catch (e) {
      error.value = `打开目录失败: ${e}`
    }
  }

  async function connectWebDAV(config: WebDavConfig) {
    error.value = null
    isLoading.value = true

    try {
      const newSettings = await invoke<Settings>('connect_webdav', { config })
      settings.value = newSettings
      await loadWorkspace()

      if (files.value.length > 0) {
        const firstFile = findFirstFile(files.value)
        if (firstFile) {
          await openFile(firstFile.path)
        }
      }
    } catch (e) {
      error.value = `连接 WebDAV 失败: ${e}`
      throw e
    } finally {
      isLoading.value = false
    }
  }

  async function loadWorkspace() {
    if (!hasWorkspace()) return

    isLoading.value = true
    error.value = null

    try {
      const path = isWebDAV.value ? '' : (workspaceDir.value || '')
      const entries = await invoke<FileEntry[]>('list_directory', { path })
      files.value = entries
    } catch (e) {
      error.value = `加载文件列表失败: ${e}`
      files.value = []
    } finally {
      isLoading.value = false
    }
  }

  async function expandDirectory(entry: FileEntry) {
    if (!entry.is_dir) return

    if (entry.children && entry.children.length > 0) {
      entry.children = []
      return
    }

    try {
      const children = await invoke<FileEntry[]>('list_directory', { path: entry.path })
      entry.children = children
    } catch (e) {
      error.value = `展开目录失败: ${e}`
    }
  }

  async function openFile(path: string) {
    if (path.startsWith('__demo__/')) {
      currentFilePath.value = path
      currentContent.value = demoContents[path] || ''
      lastSavedContent.value = currentContent.value
      return
    }

    error.value = null

    try {
      const content = await invoke<string>('read_file', { path })
      currentFilePath.value = path
      currentContent.value = content
      lastSavedContent.value = content
    } catch (e) {
      error.value = `打开文件失败: ${e}`
    }
  }

  async function saveCurrentFile() {
    if (!currentFilePath.value) return
    if (currentFilePath.value.startsWith('__demo__/')) {
      error.value = '请先打开本地文件夹或连接 WebDAV'
      return
    }

    error.value = null

    try {
      await invoke('write_file', {
        path: currentFilePath.value,
        content: currentContent.value,
      })
      lastSavedContent.value = currentContent.value
    } catch (e) {
      error.value = `保存文件失败: ${e}`
    }
  }

  async function createFile(parentPath: string, name: string) {
    if (!hasWorkspace()) {
      error.value = '请先打开文件夹或连接 WebDAV'
      return
    }

    error.value = null

    try {
      await invoke('create_file', { parentPath, name })
      await refreshFileTree()
    } catch (e) {
      error.value = `创建文件失败: ${e}`
    }
  }

  async function createDirectory(parentPath: string, name: string) {
    if (!hasWorkspace()) {
      error.value = '请先打开文件夹或连接 WebDAV'
      return
    }

    error.value = null

    try {
      await invoke('create_directory', { parentPath, name })
      await refreshFileTree()
    } catch (e) {
      error.value = `创建目录失败: ${e}`
    }
  }

  async function deleteFile(path: string) {
    error.value = null

    try {
      await invoke('delete_file', { path })
      await refreshFileTree()

      if (currentFilePath.value === path) {
        currentFilePath.value = null
        currentContent.value = ''
        lastSavedContent.value = ''
      }
    } catch (e) {
      error.value = `删除失败: ${e}`
    }
  }

  async function copyEntry(sourcePath: string, destPath: string) {
    error.value = null

    try {
      await invoke('copy_entry', { sourcePath, destPath })
      await refreshFileTree()
    } catch (e) {
      error.value = `复制失败: ${e}`
    }
  }

  async function renameEntry(path: string, newName: string) {
    error.value = null

    try {
      const result = await invoke<FileEntry>('rename_entry', { path, newName })
      await refreshFileTree()

      if (currentFilePath.value === path) {
        currentFilePath.value = result.path
      }
    } catch (e) {
      error.value = `重命名失败: ${e}`
    }
  }

  async function refreshFileTree() {
    await loadWorkspace()
  }

  function getParentPath(entryPath: string): string {
    const normalized = entryPath.replace(/\\/g, '/')
    const lastSlash = normalized.lastIndexOf('/')
    return lastSlash >= 0 ? normalized.substring(0, lastSlash) : workspaceDir.value || ''
  }

  function getFileName(path: string | null): string {
    if (!path) return '未命名'
    if (path.startsWith('__demo__/')) {
      return path.replace('__demo__/', '')
    }
    const parts = path.replace(/\\/g, '/').split('/')
    return parts[parts.length - 1] || '未命名'
  }

  function isDemoMode(): boolean {
    return currentFilePath.value !== null && currentFilePath.value.startsWith('__demo__/')
  }

  return {
    // State
    settings,
    workspaceDir,
    backendType,
    isWebDAV,
    isLocal,
    workspaceLabel,
    files,
    currentFilePath,
    currentContent,
    lastSavedContent,
    isDirty,
    isLoading,
    error,
    // Methods
    init,
    pickAndSetDirectory,
    connectWebDAV,
    loadWorkspace,
    expandDirectory,
    openFile,
    saveCurrentFile,
    createFile,
    createDirectory,
    deleteFile,
    copyEntry,
    renameEntry,
    getFileName,
    getParentPath,
    isDemoMode,
    refreshFileTree,
  }
}
