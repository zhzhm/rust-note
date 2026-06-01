<template>
  <Teleport to="body">
    <div v-if="visible" class="dialog-overlay" @click="$emit('close')">
      <div class="dialog-content" @click.stop>
        <div class="dialog-header">
          <h2>连接 WebDAV</h2>
          <button class="close-btn" @click="$emit('close')">✕</button>
        </div>
        <div class="dialog-body">
          <div class="form-group">
            <label>服务器地址</label>
            <input
              v-model="form.url"
              type="text"
              placeholder="https://example.com/remote.php/dav/files/user/"
              class="form-input"
            />
          </div>
          <div class="form-group">
            <label>用户名</label>
            <input
              v-model="form.username"
              type="text"
              placeholder="用户名"
              class="form-input"
            />
          </div>
          <div class="form-group">
            <label>密码</label>
            <input
              v-model="form.password"
              type="password"
              placeholder="密码"
              class="form-input"
            />
          </div>
          <div class="form-group">
            <label>基础路径（可选）</label>
            <input
              v-model="form.basePath"
              type="text"
              placeholder="如: notes，留空表示根目录"
              class="form-input"
            />
          </div>

          <div v-if="error" class="dialog-error">{{ error }}</div>

          <div class="dialog-actions">
            <button class="btn-cancel" @click="$emit('close')">取消</button>
            <button class="btn-connect" :disabled="connecting" @click="handleConnect">
              {{ connecting ? '连接中...' : '连接' }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { reactive } from 'vue'
import type { WebDavConfig } from '../composables/useWorkspace'

defineProps<{
  visible: boolean
  connecting: boolean
  error: string | null
}>()

const emit = defineEmits<{
  close: []
  connect: [config: WebDavConfig]
}>()

const form = reactive({
  url: '',
  username: '',
  password: '',
  basePath: '',
})

function handleConnect() {
  if (!form.url || !form.username || !form.password) {
    return
  }
  emit('connect', {
    url: form.url,
    username: form.username,
    password: form.password,
    base_path: form.basePath || null,
  })
}
</script>

<style scoped>
.dialog-overlay {
  position: fixed;
  inset: 0;
  background-color: rgba(0, 0, 0, 0.5);
  display: flex;
  justify-content: center;
  align-items: center;
  z-index: 1000;
}

.dialog-content {
  background-color: #3c4251;
  border-radius: 8px;
  width: 90%;
  max-width: 460px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
}

.dialog-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid #444;
}

.dialog-header h2 {
  margin: 0;
  font-size: 16px;
  color: #e0e0e0;
}

.close-btn {
  background: none;
  border: none;
  color: #ccc;
  font-size: 18px;
  cursor: pointer;
}

.close-btn:hover {
  color: #fff;
}

.dialog-body {
  padding: 20px;
}

.form-group {
  margin-bottom: 14px;
}

.form-group label {
  display: block;
  margin-bottom: 4px;
  font-size: 13px;
  color: #bbb;
}

.form-input {
  width: 100%;
  padding: 8px 12px;
  background-color: #2a2d35;
  border: 1px solid #555;
  border-radius: 4px;
  color: #e0e0e0;
  font-size: 14px;
  outline: none;
  box-sizing: border-box;
}

.form-input:focus {
  border-color: #6db3f8;
}

.form-input::placeholder {
  color: #777;
}

.dialog-error {
  background-color: #5c1a1a;
  color: #ff8a8a;
  padding: 8px 12px;
  border-radius: 4px;
  font-size: 13px;
  margin-bottom: 14px;
  word-break: break-word;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 6px;
}

.btn-cancel {
  padding: 8px 20px;
  background-color: transparent;
  border: 1px solid #555;
  border-radius: 4px;
  color: #ccc;
  cursor: pointer;
  font-size: 14px;
}

.btn-cancel:hover {
  background-color: #4a4f5a;
}

.btn-connect {
  padding: 8px 24px;
  background-color: #4caf50;
  border: none;
  border-radius: 4px;
  color: white;
  cursor: pointer;
  font-size: 14px;
}

.btn-connect:hover:not(:disabled) {
  background-color: #45a049;
}

.btn-connect:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
</style>
