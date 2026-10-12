<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import { invoke } from '@tauri-apps/api/core'

const win = getCurrentWindow()

const props = defineProps<{
  showSettings: boolean
  showEmailSettings: boolean
  emailScanStatus: string // 'idle' | 'scanning' | 'success' | 'error'
}>()

const emit = defineEmits<{
  'toggle-settings': []
  'toggle-email-settings': []
}>()

async function handleMinimize() {
  try {
    await invoke('minimize_window')
  } catch (e) {
    console.error('Failed to minimize window:', e)
    await win.minimize()
  }
}

async function handleClose() {
  await win.hide()
}
</script>

<template>
  <div class="title-bar" data-tauri-drag-region>
    <span class="title-text" data-tauri-drag-region>TipTips</span>
    <div class="title-actions">
      <button class="icon-btn" :class="{ active: showSettings }" @click="emit('toggle-settings')" title="设置">
        <svg width="14" height="14" viewBox="0 0 14 14">
          <path d="M7 5.5a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3z" fill="none" stroke="currentColor" stroke-width="1.2" />
          <path d="M11.5 7c0-.3 0-.6-.1-.9l1-.8-1-1.7-1.2.4c-.4-.3-.9-.6-1.4-.7L8.5 2h-3l-.3 1.3c-.5.1-1 .4-1.4.7l-1.2-.4-1 1.7 1 .8c-.1.3-.1.6-.1.9s0 .6.1.9l-1 .8 1 1.7 1.2-.4c.4.3.9.6 1.4.7L5.5 12h3l.3-1.3c.5-.1 1-.4 1.4-.7l1.2.4 1-1.7-1-.8c.1-.3.1-.6.1-.9z" fill="none" stroke="currentColor" stroke-width="0.8" />
        </svg>
      </button>
      <button class="icon-btn email-btn" :class="[{ active: showEmailSettings }, emailScanStatus]" @click="emit('toggle-email-settings')" title="邮件扫描设置">
        <svg width="14" height="14" viewBox="0 0 14 14">
          <rect x="1.5" y="3" width="11" height="8" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.1" />
          <path d="M2 4.2L7 7.8L12 4.2" fill="none" stroke="currentColor" stroke-width="1.1" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
      </button>
      <button class="icon-btn minimize-btn" @click="handleMinimize" title="最小化">
        <svg width="14" height="14" viewBox="0 0 14 14">
          <path d="M2.5 7h9" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" />
        </svg>
      </button>
      <button class="icon-btn close-btn" @click="handleClose" title="关闭 (隐藏到托盘)">
        <svg width="14" height="14" viewBox="0 0 14 14">
          <path d="M3.5 3.5L10.5 10.5M10.5 3.5L3.5 10.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" />
        </svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
.title-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 14px;
  height: 36px;
  flex-shrink: 0;
  user-select: none;
}

.title-text {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  letter-spacing: 0.5px;
}

.title-actions {
  display: flex;
  gap: 4px;
}

.icon-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  transition: background 0.2s, color 0.2s;
}

.icon-btn:hover {
  background: var(--surface-2);
  color: var(--text-primary);
}

.icon-btn.active {
  background: var(--accent-glow);
  color: var(--accent);
}

.email-btn.scanning {
  color: var(--accent);
  animation: pulse 1.5s infinite;
}

.email-btn.success {
  color: #22c55e;
}

.email-btn.error {
  color: var(--danger);
}

@keyframes pulse {
  0% { opacity: 0.5; }
  50% { opacity: 1; }
  100% { opacity: 0.5; }
}

.close-btn:hover {
  background: var(--danger-bg);
  color: var(--danger);
}
</style>
