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

    <div class="search-panel">
      <!-- 模式切换：用带文字的标签页区分「文件名」与「内容」搜索 -->
      <div class="search-modes">
        <div class="mode-tabs" role="tablist">
          <button
            class="mode-tab"
            :class="{ active: searchMode === 'name' }"
            role="tab"
            :aria-selected="searchMode === 'name'"
            @click="setSearchMode('name')"
            title="按文件名搜索（支持中文拼音）"
          >🔍 文件名</button>
          <button
            class="mode-tab"
            :class="{ active: searchMode === 'content' }"
            role="tab"
            :aria-selected="searchMode === 'content'"
            @click="setSearchMode('content')"
            title="按文件内容全文搜索（Ctrl+Shift+F）"
          >📝 内容</button>
        </div>
        <button
          class="refresh-btn"
          :class="{ spinning: isIndexing }"
          @click="$emit('refreshIndex')"
          title="刷新文件索引"
          :disabled="isIndexing"
        >↻</button>
      </div>

      <div class="search-box">
        <input
          ref="searchInputRef"
          v-model="activeQuery"
          type="text"
          class="search-input"
          :class="{ 'content-mode': searchMode === 'content' }"
          :placeholder="searchMode === 'name' ? '搜索文件名...' : '搜索文件内容...'"
          @keydown.esc="clearQuery"
        />
        <button
          class="case-btn"
          :class="{ active: caseSensitive && searchMode === 'content', 'is-hidden': searchMode === 'name' }"
          :disabled="searchMode === 'name'"
          :aria-hidden="searchMode === 'name'"
          :tabindex="searchMode === 'name' ? -1 : 0"
          @click="caseSensitive = !caseSensitive"
          :title="caseSensitive ? '区分大小写（点击关闭）' : '忽略大小写（点击开启）'"
        >Aa</button>
        <button v-if="hasQuery" class="search-clear" @click="clearQuery" title="清除">×</button>
      </div>
    </div>

    <div class="file-tree">
      <!-- 文件名搜索模式 -->
      <template v-if="searchMode === 'name'">
        <template v-if="isSearching">
          <div class="search-summary">
            <span class="summary-icon">🔍</span>
            <span>{{ searchResults.length }} 个文件名匹配</span>
            <span v-if="isIndexing" class="summary-loading" title="正在构建文件索引">↻</span>
          </div>
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
      </template>

      <!-- 内容全文搜索模式 -->
      <template v-else>
        <div v-if="!hasWorkspace" class="empty-text">请先打开文件夹或连接 WebDAV。</div>
        <div v-else-if="!contentQuery.trim()" class="empty-text">
          输入关键词，搜索工作区中所有文件的内容。双击结果可打开文件并跳转到匹配行。
        </div>
        <div v-else-if="isContentSearching && !hasContentSearched" class="loading-text">搜索中...</div>
        <template v-else>
          <div v-if="hasContentSearched" class="search-summary">
            <span class="summary-icon">📝</span>
            <span>{{ contentTotalMatches }} 处匹配 · {{ contentResults.length }} 个文件</span>
            <span v-if="isContentSearching" class="summary-loading" title="搜索中">↻</span>
            <span v-if="contentTruncated" class="summary-warn" title="结果过多，仅显示前一部分">已截断</span>
          </div>
          <div v-else class="loading-text">搜索中...</div>

          <div v-if="hasContentSearched && contentResults.length === 0" class="empty-text">
            无匹配内容
          </div>

          <div v-for="file in contentResults" :key="file.path" class="content-group">
            <div
              class="content-file"
              :class="{ active: file.path === currentFilePath }"
              :title="file.path"
              @dblclick="openContentFile(file)"
            >
              <span class="content-file-icon">📄</span>
              <span class="content-file-name">{{ file.name }}</span>
              <span class="content-file-count">{{ file.match_count }}</span>
            </div>
            <div
              v-for="match in file.matches"
              :key="file.path + ':' + match.line"
              class="content-match"
              :class="{ selected: isSelectedMatch(file.path, match.line) }"
              @click="selectedMatch = { path: file.path, line: match.line }"
              @dblclick="openContentMatch(file.path, match.line)"
            >
              <span class="content-line-no">{{ match.line }}</span>
              <span class="content-line-text">
                <span v-html="highlightContent(match.text)"></span>
                <span v-if="match.next_line" class="content-line-next">{{ match.next_line }}</span>
              </span>
            </div>
            <div v-if="file.match_count > file.matches.length" class="content-more">
              … 还有 {{ file.match_count - file.matches.length }} 处匹配未显示
            </div>
          </div>
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
import { reactive, ref, computed, watch, nextTick, onBeforeUnmount } from 'vue'
import { pinyin } from 'pinyin-pro'
import { ElMessageBox } from 'element-plus'
import type { FileEntry, SearchFileResult } from '../composables/useWorkspace'
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
  contentResults: SearchFileResult[]
  contentTotalMatches: number
  contentTruncated: boolean
  isContentSearching: boolean
  hasContentSearched: boolean
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
  searchContent: [payload: { query: string; caseSensitive: boolean }]
  openMatch: [payload: { path: string; line: number | null }]
}>()

const displayName = computed(() => {
  if (props.workspaceLabel) return props.workspaceLabel
  return '项目文件'
})

const hasWorkspace = computed(() => props.isWebDAV || !!props.workspaceDir)

// Search state
const searchQuery = ref('')
const contentQuery = ref('')
/** 'name' = 按文件名过滤；'content' = 工作区全文搜索 */
const searchMode = ref<'name' | 'content'>('name')
const caseSensitive = ref(false)
const searchInputRef = ref<HTMLInputElement | null>(null)
/** 当前选中的匹配项（单击高亮，双击打开） */
const selectedMatch = ref<{ path: string; line: number } | null>(null)

/** 两个模式各自保留自己的输入内容 */
const activeQuery = computed({
  get: () => (searchMode.value === 'name' ? searchQuery.value : contentQuery.value),
  set: (value: string) => {
    if (searchMode.value === 'name') {
      searchQuery.value = value
    } else {
      contentQuery.value = value
    }
  },
})

const hasQuery = computed(() => activeQuery.value.length > 0)

function clearQuery() {
  activeQuery.value = ''
}

/** 切换搜索模式（文件名 / 内容），两种模式各自保留输入内容 */
function setSearchMode(mode: 'name' | 'content') {
  if (searchMode.value === mode) {
    nextTick(() => searchInputRef.value?.focus())
    return
  }
  searchMode.value = mode
  selectedMatch.value = null
  nextTick(() => searchInputRef.value?.focus())
}

/** 供父组件调用：切到内容搜索并聚焦输入框（Ctrl+Shift+F） */
function focusContentSearch() {
  setSearchMode('content')
  nextTick(() => searchInputRef.value?.focus())
}

/** 供父组件调用：聚焦文件名搜索框 */
function focusFileNameSearch() {
  setSearchMode('name')
  nextTick(() => searchInputRef.value?.focus())
}

defineExpose({ focusContentSearch, focusFileNameSearch })

// 内容搜索防抖：停止输入 350ms 后再请求后端
const CONTENT_SEARCH_DEBOUNCE = 350
let contentSearchTimer: ReturnType<typeof setTimeout> | null = null

function scheduleContentSearch() {
  if (contentSearchTimer) clearTimeout(contentSearchTimer)
  if (searchMode.value !== 'content') return
  contentSearchTimer = setTimeout(() => {
    contentSearchTimer = null
    emit('searchContent', {
      query: contentQuery.value,
      caseSensitive: caseSensitive.value,
    })
  }, CONTENT_SEARCH_DEBOUNCE)
}

watch([contentQuery, caseSensitive, searchMode], () => {
  selectedMatch.value = null
  scheduleContentSearch()
})

// 切换工作区（本地目录 / WebDAV 后端）后，已有结果不再有效，需要重新搜索
watch(
  () => [props.workspaceDir, props.isWebDAV],
  () => {
    selectedMatch.value = null
    scheduleContentSearch()
  },
)

onBeforeUnmount(() => {
  if (contentSearchTimer) clearTimeout(contentSearchTimer)
})

function isSelectedMatch(path: string, line: number): boolean {
  return selectedMatch.value?.path === path && selectedMatch.value?.line === line
}

function openContentMatch(path: string, line: number) {
  emit('openMatch', { path, line })
}

function openContentFile(file: SearchFileResult) {
  emit('openMatch', { path: file.path, line: file.matches[0]?.line ?? null })
}

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

function handleSearchResultClick(path: string) {
  emit('selectFile', path)
}

/** 内容搜索结果里高亮命中的关键词（忽略大小写，只高亮第一处） */
function highlightContent(text: string): string {
  const query = contentQuery.value.trim()
  if (!query) return escapeHtml(text)

  const index = caseSensitive.value
    ? text.indexOf(query)
    : text.toLowerCase().indexOf(query.toLowerCase())

  if (index < 0) return escapeHtml(text)

  const end = index + query.length
  return escapeHtml(text.slice(0, index)) +
    '<mark>' + escapeHtml(text.slice(index, end)) + '</mark>' +
    escapeHtml(text.slice(end))
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

/* ---- 搜索面板：模式标签页 + 输入行 ---- */
.search-panel {
  padding: 0 8px 8px 8px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.search-modes {
  display: flex;
  /* 让刷新按钮与标签容器自动等高 */
  align-items: stretch;
  gap: 4px;
}

/* 刷新按钮做成与标签容器同款外观，避免看起来像一个游离的字形 */
.search-modes .refresh-btn {
  width: 28px;
  height: auto;
  border: 1px solid #383d47;
  border-radius: 5px;
  background-color: #23262d;
}

.search-modes .refresh-btn:hover:not(:disabled) {
  background-color: #2f343d;
  color: #d4d4d4;
}

.mode-tabs {
  flex: 1;
  min-width: 0;
  display: flex;
  gap: 2px;
  padding: 2px;
  background-color: #23262d;
  border: 1px solid #383d47;
  border-radius: 5px;
}

.mode-tab {
  flex: 1;
  min-width: 0;
  background: none;
  border: none;
  border-radius: 3px;
  /* 未选中也要看起来可点击，别像禁用态 */
  color: #a3abb8;
  font-size: 12px;
  font-family: inherit;
  padding: 4px 6px;
  cursor: pointer;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  transition: background-color 0.15s, color 0.15s;
}

.mode-tab:hover:not(.active) {
  color: #d4d4d4;
  background-color: #2f343d;
}

.mode-tab.active {
  background-color: #3a4763;
  color: #9dc0ff;
  font-weight: 600;
}

.search-box {
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
  padding: 6px 10px;
  font-size: 13px;
  outline: none;
  transition: border-color 0.2s;
}

.search-input::placeholder {
  color: #7a828e;
}

.search-input:focus {
  border-color: #5a8dee;
}

/* 清除按钮现在是 flex 子项，排在大小写/刷新按钮之前，避免与它们重叠 */
.search-clear {
  background: none;
  border: none;
  color: #888;
  font-size: 16px;
  cursor: pointer;
  padding: 0;
  width: 20px;
  height: 24px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
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

/* ---- 大小写开关（仅内容模式显示） ---- */
.case-btn {
  background: none;
  border: none;
  color: #888;
  cursor: pointer;
  padding: 0;
  width: 26px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  border-radius: 4px;
  font-size: 12px;
  font-family: monospace;
  line-height: 1;
  transition: color 0.2s, background-color 0.2s;
}

.case-btn:hover {
  color: #d4d4d4;
  background-color: #2a2d35;
}

.case-btn.active {
  color: #5a8dee;
  background-color: #2a2d35;
}

/* 文件名模式下隐藏 Aa，但保留占位，避免切换模式时输入框宽度跳动 */
.case-btn.is-hidden {
  visibility: hidden;
}

/* 内容模式下输入框聚焦用更亮的蓝色，和标签页呼应 */
.search-input.content-mode:focus {
  border-color: #7ab4ff;
}

/* ---- 全文搜索结果 ---- */
.search-summary {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 12px 8px 12px;
  font-size: 11px;
  color: #888;
  border-bottom: 1px solid #2e323b;
  margin-bottom: 4px;
}

.summary-icon {
  font-size: 11px;
  line-height: 1;
}

.summary-loading {
  color: #5a8dee;
  animation: spin 1s linear infinite;
}

.summary-warn {
  color: #ffa726;
}

.content-group {
  margin-bottom: 2px;
}

.content-file {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 12px;
  cursor: pointer;
  font-size: 12px;
  font-weight: 600;
  color: #cfd3da;
  background-color: #262a33;
  user-select: none;
}

.content-file:hover {
  background-color: #2f343f;
}

.content-file.active {
  background-color: #3a3f4b;
}

.content-file-icon {
  font-size: 11px;
  flex-shrink: 0;
}

.content-file-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.content-file-count {
  flex-shrink: 0;
  font-size: 10px;
  font-weight: normal;
  color: #8f97a3;
  background-color: #333944;
  border-radius: 8px;
  padding: 0 6px;
  line-height: 15px;
}

.content-match {
  display: flex;
  gap: 6px;
  padding: 3px 12px 3px 16px;
  cursor: pointer;
  font-size: 12px;
  font-family: monospace;
  color: #b6bcc7;
  user-select: none;
  border-left: 2px solid transparent;
}

.content-match:hover {
  background-color: #2a2e35;
}

.content-match.selected {
  background-color: #313743;
  border-left-color: #5a8dee;
}

.content-line-no {
  flex-shrink: 0;
  width: 26px;
  text-align: right;
  color: #626a76;
  user-select: none;
}

.content-line-text {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.content-line-text :deep(mark) {
  background-color: #5a8dee44;
  color: #7ab4ff;
  border-radius: 2px;
}

.content-line-next {
  display: block;
  color: #6d7481;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.content-more {
  padding: 2px 12px 4px 42px;
  font-size: 11px;
  color: #6d7481;
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
