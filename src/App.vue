<template>
  <div class="app-container">
    <!-- 顶部菜单栏 -->
    <header class="menu-bar">
      <div class="menu-items">
        <div class="menu-item" @click="toggleMenu('file')">
          <span class="menu-label">文件</span>
          <div v-if="activeMenu === 'file'" class="dropdown">
            <div class="dropdown-item" @click="createNewFile">新建文件</div >
            <div class="dropdown-item" @click="saveFile">保存</div >
            <div class="dropdown-item" @click="exportFile">导出 HTML</div >
          </div>
        </div>
        <div class="menu-item" @click="toggleMenu('view')">
          <span class="menu-label">视图</span>
          <div v-if="activeMenu === 'view'" class="dropdown">
            <div class="dropdown-item" @click="toggleSidebar">切换侧边栏</div >
            <div class="dropdown-item" @click="togglePreview">切换预览</div >
          </div>
        </div>
        <div class="menu-item" @click="toggleMenu('help')">
          <span class="menu-label">帮助</span>
          <div v-if="activeMenu === 'help'" class="dropdown">
            <div class="dropdown-item" @click="showGuide = true">Asciidoc 语法指南</div >
          </div>
        </div>
      </div>
    </header>

    <div class="main-content">
      <!-- 左侧目录树 -->
      <aside class="sidebar" v-if="showSidebar">
        <div class="sidebar-header">
          <span role="img" aria-label="Project files">📁 项目文件</span>
          <button class="add-btn" @click="createNewFile" title="新建文件">+</button>
        </div>
        <div class="file-tree">
          <div 
            v-for="(file, index) in files" 
            :key="index"
            class="file-item"
            :class="{ active: currentFile === index }"
            @click="selectFile(index)"
          >
            <span class="file-icon" role="img" aria-label="File icon">📄</span>
            <span class="file-name">{{ file.name }}</span>
          </div>
        </div>
      </aside>

      <!-- 主编辑区域 -->
      <main class="editor-container">
        <!-- 左侧编辑区 -->
        <div class="editor-panel" :style="{ width: editorWidth + '%' }">
          <div class="panel-header">
            <span class="panel-title">{{ files[currentFile]?.name || '未命名' }}</span>
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
    </div >

    <!-- 模态框: 语法指南 -->
    <div v-if="showGuide" class="modal-overlay" @click="showGuide = false">
      <div class="modal-content" @click.stop>
        <div class="modal-header">
          <h2 class="modal-title">Asciidoc 语法指南</h2>
          <button @click="showGuide = false" class="close-btn">✕</button>
        </div>
        <div class="modal-body">
          <section>
            <h3 class="section-title">标题</h3 >
            <pre><code class="asciidoc-code">== 一级标题
=== 二级标题
==== 三级标题</code></pre>
          </section>
          <section>
            <h3 class="section-title">列表</h3 >
            <pre><code class="asciidoc-code">* 无序列表项
1. 有序列表项</code></pre>
          </section>
          <section>
            <h3 class="section-title">代码块</h3 >
            <pre><code class="asciidoc-code">====
function hello() {}
====</code></pre>
          </section>
          <section>
            <h3 class="section-title">提示框</h3 >
            <pre><code class="asciidoc-code">[NOTE]
这是提示信息。</code></pre>
          </section>
          <section>
            <h3 class="section-title">水平线</h3 >
            <pre><code class="asciidoc-code">----</code></pre>
          </section>
        </div >
      </div>
    </div >
  </div >
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import Asciidoctor from 'asciidoctor'
import hljs from 'highlight.js'

const asciidoctor = Asciidoctor()

interface FileItem {
  name: string
  content: string
}

const showSidebar = ref(true)
const showPreview = ref(true)
const activeMenu = ref<string | null>(null)
const currentFile = ref(0)
const showGuide = ref(false)
const editorWidth = ref(50)
const isDragging = ref(false)

const files = ref<FileItem[]>([
  {
    name: '欢迎文档.adoc',
    content: `== 欢迎使用 Asciidoc 编辑器

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
`
  },
  {
    name: '学习笔记.adoc',
    content: `== 学习笔记

[quote, 孔子]
====
学而不思则罔，思而不学则殆。
====

=== 学习计划

. 学习 Asciidoc 语法
. 掌握常用标记

[IMPORTANT]
请确保语法正确。
`
  }
])

const currentContent = computed({
  get: () => files.value[currentFile.value]?.content || '',
  set: (value: string) => {
    if (files.value.length > 0) {
      files.value[currentFile.value].content = value
    }
  }
})

const renderedHtml = computed(() => {
  return renderAsciiDoc(currentContent.value)
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

function createNewFile() {
  const name = prompt('请输入文件名（以 .adoc 结尾）：', '新文档.adoc')
  if (name) {
    files.value.push({ name, content: '' })
    currentFile.value = files.value.length - 1
  }
}

function saveFile() {
  if (files.value.length === 0) return
  const file = files.value[currentFile.value]
  const blob = new Blob([file.content], { type: 'text/plain' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = file.name
  a.click()
  URL.revokeObjectURL(url)
}

function exportFile() {
  if (files.value.length === 0) return
  const html = renderedHtml.value
  const blob = new Blob([html], { type: 'text/html' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = files.value[currentFile.value].name.replace(/\.adoc$/, '.html')
  a.click()
  URL.revokeObjectURL(url)
}

function toggleSidebar() {
  showSidebar.value = !showSidebar.value
}

function togglePreview() {
  showPreview.value = !showPreview.value
}

function selectFile(index: number) {
  currentFile.value = index
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
})

onUnmounted(() => {
  document.removeEventListener('mousemove', handleMouseMove)
  document.removeEventListener('mouseup', handleMouseUp)
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
  padding: 10px 15px;
  display: flex;
  align-items: center;
  font-size: 14px;
}

.menu-label {
  font-weight: bold;
  margin-right: 15px;
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

/* Sidebar (Directory Tree) */
.sidebar {
  flex-shrink: 0;
  width: 220px;
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
  padding: 0 15px 10px 15px;
  border-bottom: 1px solid #333;
  margin-bottom: 10px;
}

.sidebar-header span {
  font-weight: bold;
  font-size: 16px;
}

.add-btn {
  background-color: #4CAF50;
  color: white;
  border: none;
  border-radius: 50%;
  width: 30px;
  height: 30px;
  font-size: 18px;
  cursor: pointer;
  transition: background-color 0.2s;
}

.add-btn:hover {
  background-color: #45a049;
}

.file-tree {
  overflow-y: auto;
  flex-grow: 1;
  padding: 0 15px;
}

.file-item {
  display: flex;
  align-items: center;
  padding: 8px 0;
  cursor: pointer;
  transition: background-color 0.1s;
}

.file-item:hover {
  background-color: #2a2e35;
}

.file-item.active {
  background-color: #4d5667;
  padding-left: 5px;
}

.file-icon {
  margin-right: 10px;
  color: #aaa;
}

.file-name {
  flex-grow: 1;
  font-size: 14px;
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

/* Editor Panel */
.editor-panel {
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
}

/* Editor Textarea */
.editor-textarea {
  flex-grow: 1;
  width: 100%;
  min-height: 0;
  border: none;
  outline: none;
  padding: 15px;
  background-color: #1e1e1e;
  color: #d4d4d4;
  font-family: monospace;
  font-size: 14px;
  resize: none;
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
  background-color: #ffffff;
  box-shadow: -2px 0 5px rgba(0, 0, 0, 0.1);
}

.preview-content {
  flex-grow: 1;
  padding: 20px;
  overflow-y: auto;
  color: #333;
}

/* -------------------------
   Modal (Guide)
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
  max-width: 800px;
  max-height: 90vh;
  overflow-y: auto;
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid #444;
  padding-bottom: 10px;
  margin-bottom: 20px;
}

.close-btn {
  background: none;
  border: none;
  color: #ccc;
  font-size: 20px;
  cursor: pointer;
}

/* -------------------------
   Asciidoc Rendering Styles (For Preview)
   ------------------------------- */
.admonition {
  padding: 10px 20px;
  margin: 15px 0;
  border-radius: 4px;
  border-left: 5px solid;
  background-color: #f9f9f9;
}

.admonition-title {
  font-weight: bold;
  display: block;
  margin-bottom: 5px;
}

.admonition-content {
  padding-left: 10px;
  color: #333;
}

.admonition.note {
  border-color: #2196F3;
  background-color: #e3f2fd;
}
.admonition.tip {
  border-color: #4CAF50;
  background-color: #e8f5e9;
}
.admonition.important {
  border-color: #FF9800;
  background-color: #ffe0b2;
}
.admonition.warning {
  border-color: #FFC107;
  background-color: #fff9c4;
}
.admonition.caution {
  border-color: #9C27B0;
  background-color: #f3e5f5;
}

.quote {
  border-left: 4px solid #ccc;
  padding-left: 20px;
  margin: 15px 0;
  color: #555;
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
</style>