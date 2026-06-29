<template>
  <aside class="sidebar">
    <div class="sidebar-header">
      <span class="sidebar-title">
        <span role="img" :aria-label="isWebDAV ? 'WebDAV' : 'Local'">{{ isWebDAV ? '🌐' : '📁' }}</span>
        {{ displayName }}
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

    <div class="search-box">
      <button
        class="refresh-btn"
        :class="{ spinning: isIndexing }"
        @click="$emit('refreshIndex')"
        title="刷新文件索引"
        :disabled="isIndexing"
      >↻</button>
      <input
        v-model="searchQuery"
        type="text"
        class="search-input"
        placeholder="搜索文件..."
        @keydown.esc="clearSearch"
      />
      <button v-if="isSearching" class="search-clear" @click="clearSearch" title="清除搜索">×</button>
    </div>

    <div class="file-tree">
      <!-- Search results mode -->
      <template v-if="isSearching">
        <div v-if="searchResults.length === 0" class="empty-text">无匹配文件</div>
        <div
          v-for="result in searchResults"
          :key="result.entry.path"
          class="search-result-item"
          :class="{ active: result.entry.path === currentFilePath }"
          @click="handleSearchResultClick(result.entry.path)"
        >
          <span class="result-name" v-html="highlightMatch(result.entry.name, searchQuery.trim())"></span>
          <span v-if="result.relativePath" class="result-path">{{ result.relativePath }}</span>
        </div>
      </template>

      <!-- Normal tree mode -->
      <template v-else>
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
import { pinyin } from 'pinyin-pro'
import { ElMessageBox } from 'element-plus'
import type { FileEntry } from '../composables/useWorkspace'
import FileTreeNode from './FileTreeNode.vue'

const props = defineProps<{
  files: FileEntry[]
  currentFilePath: string | null
  workspaceDir: string | null
  workspaceLabel: string
  isWebDAV: boolean
  isLoading: boolean
  error: string | null
  fileIndex: FileEntry[]
  isIndexing: boolean
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
  refreshIndex: []
}>()

const displayName = computed(() => {
  if (props.workspaceLabel) return props.workspaceLabel
  return '项目文件'
})

// Search state
const searchQuery = ref('')

interface SearchResult {
  entry: FileEntry
  relativePath: string
}

function fuzzyMatch(name: string, query: string): boolean {
  const lowerName = name.toLowerCase()
  const lowerQuery = query.toLowerCase()
  // Direct substring match
  if (lowerName.includes(lowerQuery)) return true
  // Pinyin match: convert Chinese chars to pinyin and try again
  const pinyinName = pinyin(name, { toneType: 'none', type: 'array' }).join('').toLowerCase()
  if (pinyinName.includes(lowerQuery)) return true
  // Also try with spaces between pinyin syllables
  const pinyinSpaced = pinyin(name, { toneType: 'none', type: 'array' }).join(' ').toLowerCase()
  if (pinyinSpaced.includes(lowerQuery)) return true
  return false
}

function getRelativePath(entryPath: string): string {
  let rel: string
  if (props.isWebDAV) {
    rel = entryPath
  } else if (props.workspaceDir) {
    const normalized = entryPath.replace(/\\/g, '/')
    const prefix = props.workspaceDir.replace(/\\/g, '/').replace(/\/$/, '') + '/'
    rel = normalized.startsWith(prefix) ? normalized.slice(prefix.length) : entryPath
  } else {
    rel = entryPath
  }
  // Return parent directory only (filename is already shown separately)
  const lastSlash = rel.lastIndexOf('/')
  return lastSlash > 0 ? rel.slice(0, lastSlash) : ''
}

const searchResults = computed<SearchResult[]>(() => {
  const q = searchQuery.value.trim()
  if (!q) return []
  return props.fileIndex
    .filter(entry => !entry.is_dir && fuzzyMatch(entry.name, q))
    .map(entry => ({ entry, relativePath: getRelativePath(entry.path) }))
})

const isSearching = computed(() => searchQuery.value.trim().length > 0)

function clearSearch() {
  searchQuery.value = ''
}

function handleSearchResultClick(path: string) {
  emit('selectFile', path)
}

function highlightMatch(text: string, query: string): string {
  if (!query) return escapeHtml(text)
  const lowerText = text.toLowerCase()
  const lowerQuery = query.toLowerCase()
  const idx = lowerText.indexOf(lowerQuery)
  if (idx >= 0) {
    return escapeHtml(text.slice(0, idx)) +
      '<mark>' + escapeHtml(text.slice(idx, idx + query.length)) + '</mark>' +
      escapeHtml(text.slice(idx + query.length))
  }
  return escapeHtml(text)
}

function escapeHtml(str: string): string {
  return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
}

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

async function handleRootNewFile() {
  showAddMenu.value = false
  try {
    const { value } = await ElMessageBox.prompt('请输入文件名：', '新建文件', {
      inputValue: '新文档.adoc',
      confirmButtonText: '确定',
      cancelButtonText: '取消',
    })
    if (value) {
      emit('createFile', { parentPath: props.workspaceDir || '', name: value })
    }
  } catch { /* cancelled */ }
}

async function handleRootNewDirectory() {
  showAddMenu.value = false
  try {
    const { value } = await ElMessageBox.prompt('请输入目录名：', '新建文件夹', {
      inputValue: '新目录',
      confirmButtonText: '确定',
      cancelButtonText: '取消',
    })
    if (value) {
      emit('createDirectory', { parentPath: props.workspaceDir || '', name: value })
    }
  } catch { /* cancelled */ }
}

async function handleNewFile() {
  if (!contextMenu.entry) return
  const parentPath = getParentPathForEntry(contextMenu.entry)
  closeContextMenu()
  try {
    const { value } = await ElMessageBox.prompt('请输入文件名：', '新建文件', {
      inputValue: '新文档.adoc',
      confirmButtonText: '确定',
      cancelButtonText: '取消',
    })
    if (value) {
      emit('createFile', { parentPath, name: value })
    }
  } catch { /* cancelled */ }
}

async function handleNewDirectory() {
  if (!contextMenu.entry || !contextMenu.entry.is_dir) return
  // Save entry reference before closing context menu
  const entry = contextMenu.entry
  closeContextMenu()
  try {
    const { value } = await ElMessageBox.prompt('请输入目录名：', '新建文件夹', {
      inputValue: '新目录',
      confirmButtonText: '确定',
      cancelButtonText: '取消',
    })
    if (value) {
      emit('createDirectory', { parentPath: entry.path, name: value })
    }
  } catch { /* cancelled */ }
}

async function handleRename() {
  if (!contextMenu.entry) return
  // Save references before closing context menu
  const entry = contextMenu.entry
  const entryName = entry.name
  const entryPath = entry.path
  closeContextMenu()
  try {
    const { value } = await ElMessageBox.prompt('请输入新名称：', '重命名', {
      inputValue: entryName,
      confirmButtonText: '确定',
      cancelButtonText: '取消',
    })
    if (value && value !== entryName) {
      emit('renameFile', { path: entryPath, newName: value })
    }
  } catch { /* cancelled */ }
}

function handleCopy() {
  if (!contextMenu.entry) return
  emit('copyFile', contextMenu.entry.path)
  closeContextMenu()
}

async function handleDelete() {
  if (!contextMenu.entry) return
  // Save references before closing context menu
  const entry = contextMenu.entry
  const typeLabel = entry.is_dir ? '目录' : '文件'
  const entryName = entry.name
  const entryPath = entry.path
  closeContextMenu()
  try {
    await ElMessageBox.confirm(
      `确定要删除${typeLabel} "${entryName}" 吗？此操作不可恢复。`,
      '确认删除',
      { confirmButtonText: '删除', cancelButtonText: '取消', type: 'warning' },
    )
    emit('deleteFile', entryPath)
  } catch { /* cancelled */ }
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

.search-box {
  position: relative;
  padding: 0 8px 8px 8px;
  display: flex;
  align-items: center;
  gap: 4px;
}

.refresh-btn {
  background: none;
  border: none;
  color: #888;
  font-size: 16px;
  cursor: pointer;
  padding: 0;
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  border-radius: 4px;
  transition: color 0.2s, background-color 0.2s;
  line-height: 1;
}

.refresh-btn:hover:not(:disabled) {
  color: #d4d4d4;
  background-color: #2a2d35;
}

.refresh-btn:disabled {
  cursor: default;
}

.refresh-btn.spinning {
  animation: spin 1s linear infinite;
  color: #5a8dee;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.search-input {
  flex: 1;
  min-width: 0;
  background-color: #2a2d35;
  border: 1px solid #444;
  border-radius: 4px;
  color: #d4d4d4;
  padding: 6px 28px 6px 10px;
  font-size: 13px;
  outline: none;
  transition: border-color 0.2s;
}

.search-input::placeholder {
  color: #666;
}

.search-input:focus {
  border-color: #5a8dee;
}

.search-clear {
  position: absolute;
  right: 12px;
  top: 50%;
  transform: translateY(-50%);
  background: none;
  border: none;
  color: #888;
  font-size: 16px;
  cursor: pointer;
  padding: 0 4px;
  line-height: 1;
}

.search-clear:hover {
  color: #d4d4d4;
}

.search-result-item {
  display: flex;
  flex-direction: column;
  padding: 6px 12px;
  cursor: pointer;
  transition: background-color 0.1s;
  font-size: 13px;
  user-select: none;
}

.search-result-item:hover {
  background-color: #2a2e35;
}

.search-result-item.active {
  background-color: #3a3f4b;
}

.result-name {
  color: #d4d4d4;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.result-name :deep(mark) {
  background-color: #5a8dee44;
  color: #7ab4ff;
  border-radius: 2px;
  padding: 0 1px;
}

.result-path {
  font-size: 11px;
  color: #666;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-top: 2px;
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
