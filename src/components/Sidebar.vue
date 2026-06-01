<template>
  <aside class="sidebar">
    <div class="sidebar-header">
      <span class="sidebar-title">
        <span role="img" aria-label="Project files">📁</span>
        {{ folderName }}
      </span>
      <div class="header-actions">
        <div class="add-btn-wrapper">
          <button
            class="action-btn"
            @click.stop="toggleAddMenu"
            title="新建"
          >
            +
          </button>
          <div v-if="showAddMenu" class="add-menu">
            <div class="add-menu-item" @click.stop="handleRootNewFile">
              <span class="context-menu-icon">📄</span>
              <span>新建文件</span>
            </div>
            <div class="add-menu-item" @click.stop="handleRootNewDirectory">
              <span class="context-menu-icon">📁</span>
              <span>新建文件夹</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Click-outside overlay for add menu -->
    <div
      v-if="showAddMenu"
      class="context-menu-overlay"
      @click="showAddMenu = false"
      @contextmenu.prevent="showAddMenu = false"
    ></div>

    <div v-if="error" class="sidebar-error" @click="$emit('clearError')">
      {{ error }}
    </div>

    <div class="file-tree">
      <div v-if="isLoading" class="loading-text">加载中...</div>
      <div v-else-if="files.length === 0 && !isLoading" class="empty-text">
        暂无文件。请通过 文件 → 打开文件夹 选择工作目录，或点击 + 新建文件。
      </div>
      <template v-else>
        <FileTreeNode
          v-for="entry in files"
          :key="entry.path"
          :entry="entry"
          :current-path="currentFilePath"
          :depth="0"
          @select="(path: string) => $emit('selectFile', path)"
          @expand="(entry: FileEntry) => $emit('expandDirectory', entry)"
          @contextmenu="showContextMenu"
        />
      </template>
    </div>

    <!-- Context Menu -->
    <Teleport to="body">
      <div
        v-if="contextMenu.visible"
        class="context-menu-overlay"
        @click="closeContextMenu"
        @contextmenu.prevent="closeContextMenu"
      >
        <div
          class="context-menu"
          :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }"
          @click.stop
        >
          <div class="context-menu-item" @click="handleNewFile">
            <span class="context-menu-icon">📄</span>
            <span>新建文件</span>
          </div>
          <div
            v-if="contextMenu.entry?.is_dir"
            class="context-menu-item"
            @click="handleNewDirectory"
          >
            <span class="context-menu-icon">📁</span>
            <span>新建子目录</span>
          </div>
          <div class="context-menu-separator"></div>
          <div class="context-menu-item" @click="handleRename">
            <span class="context-menu-icon">✏️</span>
            <span>重命名</span>
          </div>
          <div class="context-menu-item" @click="handleCopy">
            <span class="context-menu-icon">📋</span>
            <span>复制</span>
          </div>
          <div class="context-menu-separator"></div>
          <div class="context-menu-item context-menu-danger" @click="handleDelete">
            <span class="context-menu-icon">🗑️</span>
            <span>删除</span>
          </div>
        </div>
      </div>
    </Teleport>
  </aside>
</template>

<script setup lang="ts">
import { reactive, ref, computed } from 'vue'
import type { FileEntry } from '../composables/useWorkspace'
import FileTreeNode from './FileTreeNode.vue'

const props = defineProps<{
  files: FileEntry[]
  currentFilePath: string | null
  workspaceDir: string | null
  isLoading: boolean
  error: string | null
}>()

const emit = defineEmits<{
  selectFile: [path: string]
  expandDirectory: [entry: FileEntry]
  createFile: [payload: { parentPath: string; name: string }]
  createDirectory: [payload: { parentPath: string; name: string }]
  deleteFile: [path: string]
  copyFile: [sourcePath: string]
  renameFile: [payload: { path: string; newName: string }]
  openFolder: []
  clearError: []
}>()

const folderName = computed(() => {
  if (!props.workspaceDir) return '项目文件'
  const parts = props.workspaceDir.replace(/\\/g, '/').split('/')
  return parts[parts.length - 1] || props.workspaceDir
})

// Context menu state
const contextMenu = reactive<{
  visible: boolean
  x: number
  y: number
  entry: FileEntry | null
}>({
  visible: false,
  x: 0,
  y: 0,
  entry: null,
})

function showContextMenu(payload: { entry: FileEntry; event: MouseEvent }) {
  const menuWidth = 180
  const menuHeight = 220
  let x = payload.event.clientX
  let y = payload.event.clientY

  if (x + menuWidth > window.innerWidth) {
    x = window.innerWidth - menuWidth - 8
  }
  if (y + menuHeight > window.innerHeight) {
    y = window.innerHeight - menuHeight - 8
  }

  contextMenu.visible = true
  contextMenu.x = x
  contextMenu.y = y
  contextMenu.entry = payload.entry
}

function closeContextMenu() {
  contextMenu.visible = false
  contextMenu.entry = null
}

function getParentPathForEntry(entry: FileEntry): string {
  if (entry.is_dir) return entry.path
  const normalized = entry.path.replace(/\\/g, '/')
  const lastSlash = normalized.lastIndexOf('/')
  return lastSlash >= 0 ? normalized.substring(0, lastSlash) : (props.workspaceDir || '')
}

// Add menu state
const showAddMenu = ref(false)

function toggleAddMenu() {
  showAddMenu.value = !showAddMenu.value
}

function handleRootNewFile() {
  showAddMenu.value = false
  const name = prompt('请输入文件名：', '新文档.adoc')
  if (name && props.workspaceDir) {
    emit('createFile', { parentPath: props.workspaceDir, name })
  }
}

function handleRootNewDirectory() {
  showAddMenu.value = false
  const name = prompt('请输入目录名：', '新目录')
  if (name && props.workspaceDir) {
    emit('createDirectory', { parentPath: props.workspaceDir, name })
  }
}

function handleNewFile() {
  if (!contextMenu.entry) return
  const parentPath = getParentPathForEntry(contextMenu.entry)
  const name = prompt('请输入文件名：', '新文档.adoc')
  if (name) {
    emit('createFile', { parentPath, name })
  }
  closeContextMenu()
}

function handleNewDirectory() {
  if (!contextMenu.entry || !contextMenu.entry.is_dir) return
  const name = prompt('请输入目录名：', '新目录')
  if (name) {
    emit('createDirectory', { parentPath: contextMenu.entry.path, name })
  }
  closeContextMenu()
}

function handleRename() {
  if (!contextMenu.entry) return
  const newName = prompt('请输入新名称：', contextMenu.entry.name)
  if (newName && newName !== contextMenu.entry.name) {
    emit('renameFile', { path: contextMenu.entry.path, newName })
  }
  closeContextMenu()
}

function handleCopy() {
  if (!contextMenu.entry) return
  emit('copyFile', contextMenu.entry.path)
  closeContextMenu()
}

function handleDelete() {
  if (!contextMenu.entry) return
  const typeLabel = contextMenu.entry.is_dir ? '目录' : '文件'
  if (confirm(`确定要删除${typeLabel} "${contextMenu.entry.name}" 吗？此操作不可恢复。`)) {
    emit('deleteFile', contextMenu.entry.path)
  }
  closeContextMenu()
}
</script>

<style scoped>
.sidebar {
  flex-shrink: 0;
  width: 240px;
  background-color: #22262e;
  border-right: 1px solid #333;
  display: flex;
  flex-direction: column;
  padding: 10px 0;
}

.sidebar-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 12px 10px 12px;
  border-bottom: 1px solid #333;
  margin-bottom: 10px;
}

.sidebar-title {
  font-weight: bold;
  font-size: 14px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.header-actions {
  display: flex;
  gap: 4px;
  flex-shrink: 0;
}

.add-btn-wrapper {
  position: relative;
}

.action-btn {
  background-color: #4caf50;
  color: white;
  border: none;
  border-radius: 50%;
  width: 26px;
  height: 26px;
  font-size: 16px;
  cursor: pointer;
  transition: background-color 0.2s;
  display: flex;
  align-items: center;
  justify-content: center;
  line-height: 1;
}

.action-btn:hover {
  background-color: #45a049;
}

.add-menu {
  position: absolute;
  top: 100%;
  right: 0;
  margin-top: 4px;
  background-color: #2a2d35;
  border: 1px solid #444;
  border-radius: 6px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
  padding: 4px 0;
  min-width: 150px;
  z-index: 10001;
  font-size: 13px;
}

.add-menu-item {
  display: flex;
  align-items: center;
  padding: 7px 14px;
  cursor: pointer;
  color: #d4d4d4;
  transition: background-color 0.15s;
  white-space: nowrap;
}

.add-menu-item:hover {
  background-color: #3a3f4b;
}

.sidebar-error {
  background-color: #5c1a1a;
  color: #ff8a8a;
  padding: 8px 12px;
  margin: 0 8px 8px 8px;
  border-radius: 4px;
  font-size: 12px;
  cursor: pointer;
  word-break: break-word;
}

.file-tree {
  overflow-y: auto;
  flex-grow: 1;
  padding: 0;
}

.loading-text,
.empty-text {
  padding: 12px 15px;
  font-size: 13px;
  color: #888;
}
</style>

<!-- Non-scoped styles for context menu (Teleport to body) -->
<style>
.context-menu-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  background: transparent;
}

.context-menu {
  position: fixed;
  z-index: 10000;
  background-color: #2a2d35;
  border: 1px solid #444;
  border-radius: 6px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
  padding: 4px 0;
  min-width: 170px;
  font-size: 13px;
}

.context-menu-item {
  display: flex;
  align-items: center;
  padding: 7px 14px;
  cursor: pointer;
  color: #d4d4d4;
  transition: background-color 0.15s;
}

.context-menu-item:hover {
  background-color: #3a3f4b;
}

.context-menu-icon {
  margin-right: 10px;
  font-size: 14px;
  width: 18px;
  text-align: center;
}

.context-menu-separator {
  height: 1px;
  background-color: #444;
  margin: 4px 8px;
}

.context-menu-danger {
  color: #ff6b6b;
}

.context-menu-danger:hover {
  background-color: #4a2020;
}
</style>
