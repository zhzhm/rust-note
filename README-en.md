[🇨🇳 中文](README.md) | **🇬🇧 English**

# rust-note — AsciiDoc Note Editor

A cross-platform desktop note-taking app built with **Tauri** + **Vue 3**. Write notes in AsciiDoc syntax with live preview and native WebDAV cloud storage.

## Features

### ✍️ AsciiDoc Syntax

Write rich-text notes using the standard AsciiDoc syntax with a built-in live preview panel — see the rendered result as you type:

- Headings, paragraphs, lists (ordered / unordered)
- Tables, code blocks (with syntax highlighting)
- Blockquotes, admonitions (Note / Tip / Important / Warning / Caution)
- Images, links, anchors
- Fully compatible with the Asciidoctor.js rendering engine

No need to learn Markdown flavor variants — use an industry-grade document syntax directly.

### ☁️ Native WebDAV Support

Built-in WebDAV client — no extra sync software required. Store your notes on:

- **Synology NAS** — enable via DSM File Station WebDAV service
- **Feiniu (Fnos) Nas** and other private NAS systems with WebDAV support
- Any standard WebDAV server (NextCloud, ownCloud, public WebDAV, etc.)

Once connected, all read/write operations go directly to your NAS. Notes are saved to the remote server in real time, and your data stays in your own hands.

### 📂 Local File Management

- Open any local folder as your workspace
- File tree: browse, create, rename, copy, delete
- Fuzzy file search (with pinyin-initial support for Chinese filenames)

### 🎨 Interface

- Dark theme, easy on the eyes
- Draggable split between editor and preview panels
- Scroll sync between editor and preview
- Keyboard shortcut `Ctrl+S` to save, auto-save (30-second interval)

## Quick Start

```bash
# Install dependencies
pnpm install

# Development mode
pnpm tauri dev

# Build for production
pnpm tauri build
```

## Tech Stack

| Layer | Technology |
|---|---|
| Desktop framework | [Tauri 2](https://v2.tauri.app/) |
| Frontend | Vue 3 + TypeScript + Vite |
| AsciiDoc rendering | [Asciidoctor.js](https://github.com/asciidoctor/asciidoctor.js) |
| Syntax highlighting | [highlight.js](https://highlightjs.org/) |
| WebDAV client | Rust (reqwest + quick-xml) |
| Chinese pinyin search | [pinyin-pro](https://github.com/zh-lx/pinyin-pro) |

## License

MIT
