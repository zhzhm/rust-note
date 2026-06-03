<template>
  <div>
    <div
      class="tree-item"
      :class="{ active: entry.path === currentPath, 'is-dir': entry.is_dir }"
      :style="{ paddingLeft: (depth * 16 + 12) + 'px' }"
      @click="handleClick"
      @contextmenu.stop="handleContextMenu"
    >
      <span v-if="entry.is_dir" class="expand-arrow">
        {{ entry.children != null ? '▼' : '▶' }}
      </span>
      <span class="file-name">{{ entry.name }}</span>
    </div>
    <template v-if="entry.is_dir && entry.children && entry.children.length > 0">
      <FileTreeNode
        v-for="child in entry.children"
        :key="child.path"
        :entry="child"
        :current-path="currentPath"
        :depth="depth + 1"
        @select="(path) => $emit('select', path)"
        @expand="(entry) => $emit('expand', entry)"
        @contextmenu="(payload) => $emit('contextmenu', payload)"
      />
    </template>
  </div>
</template>

<script setup lang="ts">
import type { FileEntry } from '../composables/useWorkspace'

const props = defineProps<{
  entry: FileEntry
  currentPath: string | null
  depth: number
}>()

const emit = defineEmits<{
  select: [path: string]
  expand: [entry: FileEntry]
  contextmenu: [payload: { entry: FileEntry; event: MouseEvent }]
}>()

function handleClick() {
  if (props.entry.is_dir) {
    emit('expand', props.entry)
  } else {
    emit('select', props.entry.path)
  }
}

function handleContextMenu(e: MouseEvent) {
  e.preventDefault()
  emit('contextmenu', { entry: props.entry, event: e })
}
</script>

<style scoped>
.tree-item {
  display: flex;
  align-items: center;
  padding: 6px 12px 6px 12px;
  cursor: pointer;
  transition: background-color 0.1s;
  font-size: 13px;
  user-select: none;
}

.tree-item:hover {
  background-color: #2a2e35;
}

.tree-item.active {
  background-color: #3a3f4b;
}

.tree-item.is-dir {
  font-weight: 500;
}

.expand-arrow {
  width: 14px;
  font-size: 10px;
  color: #888;
  flex-shrink: 0;
}

.file-name {
  flex-grow: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
