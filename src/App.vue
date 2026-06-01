<template>
  <div class="app-container">
    <!-- 顶部菜单栏 -->
    <header class="menu-bar">
      <div class="menu-items">
        <div class="menu-item" @click="toggleMenu('file')">
          <span class="menu-label">文件</span>
          <div v-if="activeMenu === 'file'" class="dropdown">
            <div class="dropdown-item" @click="createNewFile">新建文件</div>
            <div class="dropdown-item" @click="openFolder">打开本地文件夹</div>
            <div class="dropdown-item" @click="showWebDavDialog = true">连接 WebDAV...</div>
            <div class="dropdown-item" @click="saveFile">保存 <span class="shortcut-hint">Ctrl+S</span></div>
            <div class="dropdown-item" @click="exportFile">导出 HTML</div>
          </div>
        </div>
        <div class="menu-item" @click="toggleMenu('view')">
          <span class="menu-label">视图</span>
          <div v-if="activeMenu === 'view'" class="dropdown">
            <div class="dropdown-item" @click="toggleSidebar">切换侧边栏</div>
            <div class="dropdown-item" @click="togglePreview">切换预览</div>
          </div>
        </div>
        <div class="menu-item" @click="toggleMenu('help')">
          <span class="menu-label">帮助</span>
          <div v-if="activeMenu === 'help'" class="dropdown">
            <div class="dropdown-item" @click="openAsciiDocGuide">Asciidoc 语法指南</div>
            <div class="dropdown-item" @click="showAbout = true">关于</div>
          </div>
        </div>
      </div>
    </header>

    <div class="main-content">
      <!-- 左侧目录树 -->
      <Sidebar
        v-if="showSidebar"
        :files="ws.files.value"
        :current-file-path="ws.currentFilePath.value"
        :workspace-dir="ws.workspaceDir.value"
        :workspace-label="ws.workspaceLabel.value"
        :is-web-d-a-v="ws.isWebDAV.value"
        :is-loading="ws.isLoading.value"
        :error="ws.error.value"
        @select-file="ws.openFile"
        @expand-directory="ws.expandDirectory"
        @create-file="handleSidebarCreateFile"
        @create-directory="handleSidebarCreateDirectory"
        @delete-file="handleDeleteFile"
        @copy-file="handleCopyFile"
        @rename-file="handleRenameFile"
        @open-folder="openFolder"
        @clear-error="ws.error.value = null"
      />

      <!-- 主编辑区域 -->
      <main class="editor-container">
        <!-- 左侧编辑区 -->
        <div class="editor-panel" :style="{ width: editorWidth + '%' }">
          <div class="panel-header">
            <span class="panel-title">
              {{ ws.getFileName(ws.currentFilePath.value) }}
              <span v-if="ws.isDirty.value" class="dirty-indicator">*</span>
            </span>
          </div>
          <textarea
            v-model="currentContent"
            class="editor-textarea"
            placeholder="输入 Asciidoc 语法..."
            spellcheck="false"
          ></textarea>
        </div>

        <!-- 可拖拽分隔条 -->
        <div v-if="showPreview" class="divider" @mousedown="startDrag">
          <div class="divider-handle"></div>
        </div>

        <!-- 右侧预览区 -->
        <div v-if="showPreview" class="preview-panel" :style="{ width: (100 - editorWidth - 4) + '%' }">
          <div class="panel-header">
            <span class="panel-title">实时预览</span>
          </div>
          <div class="preview-content" v-html="renderedHtml"></div>
        </div>
      </main>
    </div>

    <!-- WebDAV 连接对话框 -->
    <WebDavDialog
      :visible="showWebDavDialog"
      :connecting="webDavConnecting"
      :error="webDavError"
      @close="showWebDavDialog = false; webDavError = null"
      @connect="handleWebDavConnect"
    />

    <!-- 关于对话框 -->
    <div v-if="showAbout" class="modal-overlay" @click="showAbout = false">
      <div class="modal-content about-modal" @click.stop>
        <div class="modal-header">
          <h2 class="modal-title">关于 rust-note</h2>
          <button @click="showAbout = false" class="close-btn">✕</button>
        </div>
        <div class="modal-body about-body">
          <p class="about-name"><strong>rust-note</strong></p>
          <p class="about-version">版本 0.1.0</p>
          <p class="about-desc">基于 Tauri + Vue 3 的 AsciiDoc 编辑器，支持实时预览与文件管理。</p>
          <div class="about-divider"></div>
          <p class="about-contact">
            开发者：Zhiming Zhang<br />
            邮箱：<a href="mailto:bearzzm@163.com" class="about-email">bearzzm@163.com</a>
          </p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import Asciidoctor from 'asciidoctor'
import hljs from 'highlight.js'
import Sidebar from './components/Sidebar.vue'
import WebDavDialog from './components/WebDavDialog.vue'
import { useWorkspace, type WebDavConfig } from './composables/useWorkspace'
import { openUrl } from '@tauri-apps/plugin-opener'

const asciidoctor = Asciidoctor()
const ws = useWorkspace()

const showSidebar = ref(true)
const showPreview = ref(true)
const activeMenu = ref<string | null>(null)
const editorWidth = ref(50)
const showAbout = ref(false)
const showWebDavDialog = ref(false)
const webDavConnecting = ref(false)
const webDavError = ref<string | null>(null)
const isDragging = ref(false)

// v-model on textarea needs a writable ref; we bind to ws.currentContent
const currentContent = computed({
  get: () => ws.currentContent.value,
  set: (value: string) => {
    ws.currentContent.value = value
  },
})

const renderedHtml = computed(() => {
  return renderAsciiDoc(ws.currentContent.value)
})

function renderAsciiDoc(content: string): string {
  if (!content) return ''
  return asciidoctor.convert(content, { safe: 'safe' }) as string
}

// 监听预览 HTML 变化，对代码块进行语法高亮
watch(renderedHtml, () => {
  nextTick(() => {
    document.querySelectorAll('.preview-content pre code').forEach((el) => {
      hljs.highlightElement(el as HTMLElement)
    })
  })
})

function toggleMenu(menu: string) {
  activeMenu.value = activeMenu.value === menu ? null : menu
}

function openAsciiDocGuide() {
  openUrl('https://docs.asciidoctor.org/asciidoc/latest/syntax-quick-reference/')
  activeMenu.value = null
}

function createNewFile() {
  if (!ws.workspaceDir.value) {
    ws.error.value = '请先打开本地文件夹再创建文件'
    return
  }
  const name = prompt('请输入文件名（以 .adoc 结尾）：', '新文档.adoc')
  if (name) {
    ws.createFile(ws.workspaceDir.value, name)
  }
}

function handleSidebarCreateFile(payload: { parentPath: string; name: string }) {
  ws.createFile(payload.parentPath, payload.name)
}

function handleSidebarCreateDirectory(payload: { parentPath: string; name: string }) {
  ws.createDirectory(payload.parentPath, payload.name)
}

function saveFile() {
  ws.saveCurrentFile()
}

function handleKeyDown(e: KeyboardEvent) {
  const mod = e.metaKey || e.ctrlKey
  if (mod && e.key === 's') {
    e.preventDefault()
    ws.saveCurrentFile()
  }
}

function openFolder() {
  ws.pickAndSetDirectory()
}

async function handleWebDavConnect(config: WebDavConfig) {
  webDavConnecting.value = true
  webDavError.value = null
  try {
    await ws.connectWebDAV(config)
    showWebDavDialog.value = false
  } catch (e) {
    webDavError.value = String(e)
  } finally {
    webDavConnecting.value = false
  }
}

function handleDeleteFile(path: string) {
  ws.deleteFile(path)
}

function handleCopyFile(sourcePath: string) {
  const normalized = sourcePath.replace(/\\/g, '/')
  const lastDot = normalized.lastIndexOf('.')
  const baseName = normalized.substring(normalized.lastIndexOf('/') + 1)
  let newBase: string
  if (lastDot > normalized.lastIndexOf('/')) {
    const namePart = baseName.substring(0, baseName.lastIndexOf('.'))
    const extPart = baseName.substring(baseName.lastIndexOf('.'))
    newBase = namePart + ' - 副本' + extPart
  } else {
    newBase = baseName + ' - 副本'
  }
  const destPath = normalized.substring(0, normalized.lastIndexOf('/') + 1) + newBase
  ws.copyEntry(sourcePath, destPath)
}

function handleRenameFile(payload: { path: string; newName: string }) {
  ws.renameEntry(payload.path, payload.newName)
}

function exportFile() {
  if (!ws.currentContent.value) return
  const html = renderedHtml.value
  const blob = new Blob([html], { type: 'text/html' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = ws.getFileName(ws.currentFilePath.value).replace(/\.adoc$/, '.html')
  a.click()
  URL.revokeObjectURL(url)
}

function toggleSidebar() {
  showSidebar.value = !showSidebar.value
}

function togglePreview() {
  showPreview.value = !showPreview.value
}

function startDrag(e: MouseEvent) {
  e.preventDefault()
  isDragging.value = true
}

function handleMouseMove(e: MouseEvent) {
  if (!isDragging.value) return
  const container = (e.target as HTMLElement).closest('.editor-container')
  if (!container) return
  const rect = container.getBoundingClientRect()
  const newWidth = ((e.clientX - rect.left) / rect.width) * 100
  editorWidth.value = Math.max(20, Math.min(80, newWidth))
}

function handleMouseUp() {
  isDragging.value = false
}

onMounted(() => {
  document.addEventListener('mousemove', handleMouseMove)
  document.addEventListener('mouseup', handleMouseUp)
  document.addEventListener('keydown', handleKeyDown)
  ws.init()
})

onUnmounted(() => {
  document.removeEventListener('mousemove', handleMouseMove)
  document.removeEventListener('mouseup', handleMouseUp)
  document.removeEventListener('keydown', handleKeyDown)
})
</script>

<style scoped>
/* -------------------------
   Global Layout and Reset
   ------------------------- */
.app-container {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100%;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
  background-color: #282c34;
  color: #e0e0e0;
}

/* -------------------------
   Top Menu Bar
   ------------------------- */
.menu-bar {
  display: flex;
  padding: 0 15px;
  background-color: #3c4251;
  border-bottom: 1px solid #444;
}

.menu-items {
  display: flex;
  flex-direction: row;
}

.menu-item {
  position: relative;
  cursor: pointer;
  padding: 8px 10px;
  display: flex;
  align-items: center;
  font-size: 14px;
}

.menu-label {
  font-weight: bold;
}

.dropdown {
  position: absolute;
  top: 100%;
  left: 0;
  background-color: #3c4251;
  box-shadow: 0 4px 8px rgba(0, 0, 0, 0.2);
  z-index: 100;
  display: flex;
  flex-direction: column;
  min-width: 150px;
}

.dropdown-item {
  padding: 8px 15px;
  cursor: pointer;
}

.dropdown-item:hover {
  background-color: #4d5667;
}

/* --------------------------
   Main Content Layout (Sidebar + Editor)
   --------------------------- */
.main-content {
  display: flex;
  flex-direction: row;
  flex-grow: 1;
  overflow: hidden;
}

/* -------------------------
   Editor Area (The main content split)
   --------------------------- */
.editor-container {
  flex-grow: 1;
  display: flex;
  flex-direction: row;
  overflow: hidden;
}

.panel-header {
  background-color: #333;
  padding: 10px 15px;
  flex-shrink: 0;
}

.panel-title {
  font-weight: bold;
}

.dirty-indicator {
  color: #ffa726;
  font-weight: bold;
  margin-left: 2px;
}

.shortcut-hint {
  color: #888;
  font-size: 11px;
  margin-left: 8px;
  float: right;
}

/* Editor Panel */
.editor-panel {
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  overflow: hidden;
}

/* Editor Textarea */
.editor-textarea {
  flex: 1;
  width: calc(100%);
  min-height: 0;
  border: none;
  outline: none;
  padding: 12px 12px 12px 15px;
  background-color: #1e1e1e;
  color: #d4d4d4;
  font-family: monospace;
  font-size: 14px;
  line-height: 1.6;
  resize: none;
  overflow: auto;
  box-sizing: border-box;
  scrollbar-gutter: stable;
}

/* Divider handle for resizing */
.divider {
  cursor: col-resize;
  background-color: #444;
  flex-shrink: 0;
  width: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.divider-handle {
  width: 4px;
  height: 40px;
  cursor: col-resize;
  background-color: #666;
  border-radius: 2px;
}

/* Preview Panel */
.preview-panel {
  display: flex;
  flex-direction: column;
  flex-grow: 1;
  background-color: #1a1a2e;
  box-shadow: -2px 0 5px rgba(0, 0, 0, 0.3);
}

.preview-content {
  flex-grow: 1;
  padding: 20px;
  overflow-y: auto;
  color: #d4d4d4;
}

/* -------------------------
   Modal
   ------------------------- */
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background-color: rgba(0, 0, 0, 0.5);
  display: flex;
  justify-content: center;
  align-items: center;
  z-index: 1000;
}

.modal-content {
  background-color: #3c4251;
  padding: 20px;
  border-radius: 8px;
  width: 80%;
  max-width: 420px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid #444;
  padding-bottom: 10px;
  margin-bottom: 20px;
}

.modal-title {
  font-size: 16px;
  color: #e0e0e0;
}

.close-btn {
  background: none;
  border: none;
  color: #ccc;
  font-size: 20px;
  cursor: pointer;
}

.close-btn:hover {
  color: #fff;
}

.about-body {
  text-align: center;
  color: #d4d4d4;
}

.about-name {
  font-size: 18px;
  margin-bottom: 4px;
}

.about-version {
  font-size: 13px;
  color: #888;
  margin-bottom: 12px;
}

.about-desc {
  font-size: 13px;
  color: #aaa;
  line-height: 1.5;
}

.about-divider {
  height: 1px;
  background-color: #444;
  margin: 16px 0;
}

.about-contact {
  font-size: 13px;
  color: #bbb;
  line-height: 1.8;
}

.about-email {
  color: #6db3f8;
  text-decoration: none;
}

.about-email:hover {
  text-decoration: underline;
}

</style>

<!-- Non-scoped styles for AsciiDoc rendered HTML (v-html content) -->
<style>
/* -------------------------
   Asciidoc Rendering Styles (For Preview)
   ------------------------------- */
.preview-content h1,
.preview-content h2,
.preview-content h3,
.preview-content h4,
.preview-content h5,
.preview-content h6 {
  color: #e0e0e0;
}

.preview-content p,
.preview-content li,
.preview-content td,
.preview-content th {
  color: #d4d4d4;
}

.preview-content a {
  color: #6db3f8;
}

.preview-content table {
  border-collapse: collapse;
  margin: 15px 0;
}

.preview-content th,
.preview-content td {
  border: 1px solid #555;
  padding: 8px 12px;
}

.preview-content th {
  background-color: #2a2a3e;
}

.preview-content hr {
  border-color: #444;
}

.admonition {
  padding: 10px 20px;
  margin: 15px 0;
  border-radius: 4px;
  border-left: 5px solid;
}

.admonition-title {
  font-weight: bold;
  display: block;
  margin-bottom: 5px;
}

.admonition-content {
  padding-left: 10px;
}

.admonition.note {
  border-color: #42a5f5;
  background-color: #1a2332;
  color: #bbdefb;
}

.admonition.tip {
  border-color: #66bb6a;
  background-color: #1a2e1f;
  color: #c8e6c9;
}

.admonition.important {
  border-color: #ffa726;
  background-color: #2e2618;
  color: #ffe0b2;
}

.admonition.warning {
  border-color: #ffd54f;
  background-color: #2e2a18;
  color: #fff9c4;
}

.admonition.caution {
  border-color: #ce93d8;
  background-color: #261e2e;
  color: #e1bee7;
}

.quote {
  border-left: 4px solid #666;
  padding-left: 20px;
  margin: 15px 0;
  color: #aaa;
}

.code-block pre {
  background-color: #2d2d2d;
  padding: 15px;
  border-radius: 5px;
  overflow-x: auto;
}

.code-block code {
  font-family: monospace;
  color: #ffffff;
}

/* Font-based icon fallback using emoji (no Font Awesome dependency) */
.admonitionblock td.icon i.fa {
  font-style: normal;
  font-size: 1.2em;
}

.admonitionblock td.icon i.fa::before {
  display: inline;
}

.admonitionblock td.icon .icon-tip::before {
  content: '💡';
}
.admonitionblock td.icon .icon-note::before {
  content: 'ℹ️';
}
.admonitionblock td.icon .icon-important::before {
  content: '❗';
}
.admonitionblock td.icon .icon-warning::before {
  content: '⚠️';
}
.admonitionblock td.icon .icon-caution::before {
  content: '🔥';
}
.admonitionblock td.icon .icon-bulb::before {
  content: '💡';
}
.admonitionblock td.icon .icon-info::before {
  content: 'ℹ️';
}
.admonitionblock td.icon .icon-exclamation::before {
  content: '❗';
}
.admonitionblock td.icon .icon-fire::before {
  content: '🔥';
}

/* Hide Font Awesome icon font text (empty boxes) */
.admonitionblock td.icon i.fa {
  font-family: inherit;
}

/* Custom dark thin scrollbar */
::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background-color: #555;
  border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
  background-color: #777;
}

::-webkit-scrollbar-corner {
  background: transparent;
}

/* Firefox scrollbar */
* {
  scrollbar-width: thin;
  scrollbar-color: #555 transparent;
}
</style>
