**🇨🇳 中文** | [🇬🇧 English](README-en.md)

# rust-note — AsciiDoc 笔记编辑器

基于 **Tauri** + **Vue 3** 的跨平台桌面笔记应用，使用 AsciiDoc 语法编写笔记，支持实时预览与原生 WebDAV 云端存储。

## 主要功能

### ✍️ AsciiDoc 语法支持

采用标准 AsciiDoc 语法编写富文本笔记，内置实时预览面板，编辑即所见：

- 标题、段落、列表（有序/无序）
- 表格、代码块（支持语法高亮）
- 引用、提示块（Note / Tip / Important / Warning / Caution）
- 图片、链接、锚点
- 完整兼容 Asciidoctor.js 渲染引擎

无需学习 Markdown 的变体方言，直接使用工业级文档语法。

### ☁️ 原生 WebDAV 支持

内置 WebDAV 客户端，无需安装额外同步软件即可将笔记存储在：

- **群晖 Synology** NAS — 通过 DSM 的 File Station WebDAV 服务开启
- **飞牛 Nas** 等支持 WebDAV 协议的私有 NAS 系统
- 任何标准 WebDAV 服务器（NextCloud、ownCloud、公共 WebDAV 等）

连接后所有读写操作直连 NAS，笔记文件实时保存到远端，数据始终保留在你自己手中。

### 📂 本地文件管理

- 打开任意本地文件夹作为工作空间
- 文件树浏览、新建、重命名、复制、删除
- 文件模糊搜索（支持拼音首字母检索中文文件名）

### 🎨 界面特性

- 暗色主题，护眼舒适
- 可拖拽调整编辑区 / 预览区比例
- 编辑器与预览区滚动同步
- 快捷键 `Ctrl+S` 保存，自动保存（30 秒）

## 快速开始

```bash
# 安装依赖
pnpm install

# 开发模式
pnpm tauri dev

# 构建发布包
pnpm tauri build
```

## 技术栈

| 层级 | 技术 |
|---|---|
| 桌面框架 | [Tauri 2](https://v2.tauri.app/) |
| 前端 | Vue 3 + TypeScript + Vite |
| AsciiDoc 渲染 | [Asciidoctor.js](https://github.com/asciidoctor/asciidoctor.js) |
| 语法高亮 | [highlight.js](https://highlightjs.org/) |
| WebDAV 客户端 | Rust (reqwest + quick-xml) |
| 中文拼音搜索 | [pinyin-pro](https://github.com/zh-lx/pinyin-pro) |

## 许可

MIT
