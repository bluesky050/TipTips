<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed, watch } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { invoke } from '@tauri-apps/api/core'
import TitleBar from './components/TitleBar.vue'
import PlannerInput from './components/PlannerInput.vue'
import PlannerItem from './components/PlannerItem.vue'
import ViewSwitcher from './components/ViewSwitcher.vue'
import CalendarView from './components/CalendarView.vue'
import SettingsPanel from './components/SettingsPanel.vue'
import EmailSettings from './components/EmailSettings.vue'
import FairSyncBar from './components/FairSyncBar.vue'
import { useSeuFairs } from './composables/useSeuFairs'
import { usePlanner } from './composables/usePlanner'
import { useEmailScan } from './composables/useEmailScan'

const { items, loading, loadItems, addItem, updateItem, deleteItem, toggleItem, checkRollover, reorderItems } =
  usePlanner()

const { scanStatus, setupListeners, cleanupListeners } = useEmailScan()
const { loadStatus: loadFairStatus, refresh: refreshFairs, checkDayBoundary: checkFairDayBoundary } = useSeuFairs()

const showResetNotice = ref(false)
const viewMode = ref<'list' | 'calendar'>('list')
const showSettings = ref(false)
const showEmailSettings = ref(false)
const opacity = ref(0.45)

// Apply opacity to CSS variables dynamically
watch(opacity, (val) => {
  applyOpacity(val)
  scheduleSaveConfig()
}, { immediate: false })

function applyOpacity(val: number) {
  document.documentElement.style.setProperty('--bg', `rgba(245, 247, 250, ${val})`)
  document.documentElement.style.setProperty('--surface', `rgba(255, 255, 255, ${val * 0.5})`)
  document.documentElement.style.setProperty('--surface-2', `rgba(255, 255, 255, ${val * 0.8})`)
  const isDark = window.matchMedia('(prefers-color-scheme: dark)').matches
  if (isDark) {
    document.documentElement.style.setProperty('--bg', `rgba(30, 30, 40, ${val})`)
    document.documentElement.style.setProperty('--surface', `rgba(255, 255, 255, ${val * 0.08})`)
    document.documentElement.style.setProperty('--surface-2', `rgba(255, 255, 255, ${val * 0.15})`)
  }
}

const sortedItems = computed(() => {
  return [...items.value].sort((a, b) => {
    if (a.done !== b.done) return a.done ? 1 : -1
    return a.sort_order - b.sort_order
  })
})

const hasItems = computed(() => items.value.length > 0)

// For move up/down buttons: compute position within undone group
const undoneItems = computed(() => sortedItems.value.filter(it => !it.done && it.source !== 'seu'))
const undoneCount = computed(() => undoneItems.value.length)

function getUndoneIndex(id: string): number {
  return undoneItems.value.findIndex(it => it.id === id)
}

// Move item up/down to adjust priority
function moveItemUp(id: string) {
  const undone = undoneItems.value
  const idx = undone.findIndex(it => it.id === id)
  if (idx <= 0) return
  const newOrder = [...undone]
  ;[newOrder[idx - 1], newOrder[idx]] = [newOrder[idx], newOrder[idx - 1]]
  const doneItems = sortedItems.value.filter(it => it.done)
  const fullOrder = [...newOrder, ...doneItems]
  applyNewOrder(fullOrder)
}

function moveItemDown(id: string) {
  const undone = undoneItems.value
  const idx = undone.findIndex(it => it.id === id)
  if (idx < 0 || idx >= undone.length - 1) return
  const newOrder = [...undone]
  ;[newOrder[idx + 1], newOrder[idx]] = [newOrder[idx], newOrder[idx + 1]]
  const doneItems = sortedItems.value.filter(it => it.done)
  const fullOrder = [...newOrder, ...doneItems]
  applyNewOrder(fullOrder)
}

function applyNewOrder(ordered: import('./types').PlannerItem[]) {
  const orderedIds = ordered.map(it => it.id)
  reorderItems(orderedIds)
  ordered.forEach((item, i) => {
    const local = items.value.find(it => it.id === item.id)
    if (local) local.sort_order = i
  })
}

let rolloverInterval: ReturnType<typeof setInterval> | null = null
let saveTimeout: ReturnType<typeof setTimeout> | null = null

onMounted(async () => {
  try {
    const wasCorrupt = await invoke<boolean>('was_data_corrupt')
    if (wasCorrupt) showResetNotice.value = true
  } catch (e) {
    console.error('Failed to check corrupt:', e)
  }

  // Restore config
  try {
    const cfg = await invoke<{ view_mode: string; opacity: number }>('get_window_config')
    if (cfg.view_mode === 'calendar') viewMode.value = 'calendar'
    if (cfg.opacity) {
      opacity.value = cfg.opacity
      applyOpacity(cfg.opacity)
    }
  } catch (e) {
    console.error('Failed to get config:', e)
  }

  await loadItems()
  await loadFairStatus()
  void refreshFairs()

  // Setup email scan listeners
  setupListeners(() => {
    loadItems()
  })

  rolloverInterval = setInterval(() => {
    checkRollover()
    checkFairDayBoundary()
  }, 60000)

  const win = getCurrentWindow()
  win.onResized(() => scheduleSaveConfig())
  win.onMoved(() => scheduleSaveConfig())
})

onUnmounted(() => {
  cleanupListeners()
  if (rolloverInterval) clearInterval(rolloverInterval)
  if (saveTimeout) clearTimeout(saveTimeout)
})

function switchView(mode: 'list' | 'calendar') {
  viewMode.value = mode
  scheduleSaveConfig()
}

function toggleSettings() {
  showSettings.value = !showSettings.value
  if (showSettings.value) showEmailSettings.value = false
}

function closeSettings() {
  showSettings.value = false
}

function toggleEmailSettings() {
  showEmailSettings.value = !showEmailSettings.value
  if (showEmailSettings.value) showSettings.value = false
}

function closeEmailSettings() {
  showEmailSettings.value = false
}

function updateOpacity(val: number) {
  opacity.value = val
}

function scheduleSaveConfig() {
  if (saveTimeout) clearTimeout(saveTimeout)
  saveTimeout = setTimeout(saveWindowConfig, 500)
}

async function saveWindowConfig() {
  try {
    const win = getCurrentWindow()
    const pos = await win.outerPosition()
    const size = await win.outerSize()
    await invoke('save_window_config', {
      x: pos.x,
      y: pos.y,
      width: size.width,
      height: size.height,
      viewMode: viewMode.value,
      opacity: opacity.value,
    })
  } catch (e) {
    console.error('Failed to save window config:', e)
  }
}
</script>

<template>
  <div class="app" @click.self="(showSettings && closeSettings()) || (showEmailSettings && closeEmailSettings())">
    <TitleBar
      :show-settings="showSettings"
      :show-email-settings="showEmailSettings"
      :email-scan-status="scanStatus"
      @toggle-settings="toggleSettings"
      @toggle-email-settings="toggleEmailSettings"
    />

    <SettingsPanel
      v-if="showSettings"
      :opacity="opacity"
      @update:opacity="updateOpacity"
      @close="closeSettings"
    />

    <EmailSettings
      v-if="showEmailSettings"
      @close="closeEmailSettings"
    />

    <template v-if="!showEmailSettings">
      <ViewSwitcher :mode="viewMode" @switch="switchView" />
      <FairSyncBar />

      <Transition name="view-slide" mode="out-in">
        <!-- List view -->
        <div v-if="viewMode === 'list'" key="list" class="list-view">
          <PlannerInput @add="addItem" />

          <TransitionGroup
            v-if="hasItems"
          name="item"
          tag="div"
          class="item-list"
        >
          <PlannerItem
            v-for="item in sortedItems"
            :key="item.id"
            :item="item"
            :can-move-up="item.source !== 'seu' && !item.done && undoneCount > 1 && getUndoneIndex(item.id) > 0"
            :can-move-down="item.source !== 'seu' && !item.done && undoneCount > 1 && getUndoneIndex(item.id) < undoneCount - 1"
            @toggle="toggleItem"
            @update="updateItem"
            @delete="deleteItem"
            @move-up="moveItemUp"
            @move-down="moveItemDown"
          />
        </TransitionGroup>

        <div v-else-if="!loading" class="empty-state">
          <svg width="48" height="48" viewBox="0 0 48 48" class="empty-icon">
            <rect x="8" y="10" width="32" height="30" rx="4" fill="none" stroke="currentColor" stroke-width="2" />
            <path d="M14 20h20M14 26h20M14 32h12" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
          </svg>
          <p class="empty-text">还没有规划</p>
          <p class="empty-hint">在上方输入框添加你的第一条今日规划</p>
        </div>
      </div>

      <!-- Calendar view -->
      <CalendarView v-else key="calendar" />
    </Transition>
    </template>

    <Transition name="fade">
      <div v-if="showResetNotice" class="reset-notice" @click="showResetNotice = false">
        数据文件异常，已重置为空
      </div>
    </Transition>
  </div>
</template>

<style>
:root {
  --bg: rgba(245, 247, 250, 0.45);
  --surface: rgba(255, 255, 255, 0.25);
  --surface-2: rgba(255, 255, 255, 0.4);
  --text-primary: #1a1a2e;
  --text-secondary: #545b70;
  --text-tertiary: #72788b;
  --border: rgba(0, 0, 0, 0.06);
  --border-strong: #c8ccd8;
  --accent: #be123c;
  --on-accent: #ffffff;
  --accent-glow: rgba(190, 18, 60, 0.12);
  --card-bg: #fff1f2;
  --card-hover: #ffe4e6;
  --card-border: #fda4af;
  --card-ink: #321c26;
  --card-muted: #76525d;
  --card-quiet: #785b67;
  --fair-bg: #fffbeb;
  --fair-hover: #fef3c7;
  --fair-border: #fcd34d;
  --fair-ink: #422006;
  --fair-muted: #785123;
  --fair-quiet: #795a32;
  --fair-accent: #92400e;
  --fair-glow: rgba(146, 64, 14, 0.12);
  --danger: #ef4444;
  --danger-bg: rgba(239, 68, 68, 0.1);
  --radius: 12px;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

html,
body {
  height: 100%;
  overflow: hidden;
  font-family: 'Segoe UI Variable Text', 'Segoe UI', 'Microsoft YaHei UI', 'PingFang SC', sans-serif;
  font-size: 14px;
  line-height: 1.6;
  font-variant-numeric: tabular-nums;
  -webkit-font-smoothing: antialiased;
}

button, input, textarea, select {
  font-family: inherit;
}

.fair-theme {
  --card-bg: var(--fair-bg);
  --card-hover: var(--fair-hover);
  --card-border: var(--fair-border);
  --card-ink: var(--fair-ink);
  --card-muted: var(--fair-muted);
  --card-quiet: var(--fair-quiet);
  --accent: var(--fair-accent);
  --accent-glow: var(--fair-glow);
}

button:focus-visible, summary:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 3px;
}

#app {
  height: 100%;
}

.app {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--bg);
  backdrop-filter: blur(20px) saturate(180%);
  -webkit-backdrop-filter: blur(20px) saturate(180%);
  border-radius: var(--radius);
  overflow: hidden;
  position: relative;
}

.list-view {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
}

.item-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  flex: 1;
  overflow-y: auto;
  padding: 8px 12px;
}

.item-list::-webkit-scrollbar {
  width: 4px;
}

.item-list::-webkit-scrollbar-thumb {
  background: var(--border-strong);
  border-radius: 2px;
}

/* Empty state */
.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--text-tertiary);
  padding: 20px;
}

.empty-icon {
  opacity: 0.4;
}

.empty-text {
  font-size: 15px;
  font-weight: 500;
  color: var(--text-secondary);
}

.empty-hint {
  font-size: 12px;
  color: var(--text-tertiary);
}

/* Reset notice */
.reset-notice {
  position: absolute;
  bottom: 12px;
  left: 50%;
  transform: translateX(-50%);
  background: var(--danger-bg);
  color: var(--danger);
  font-size: 12px;
  padding: 6px 14px;
  border-radius: 8px;
  cursor: pointer;
  white-space: nowrap;
}

/* View transition */
.view-slide-enter-active,
.view-slide-leave-active {
  transition: opacity 0.25s, transform 0.25s;
}

.view-slide-enter-from {
  opacity: 0;
  transform: translateY(8px);
}

.view-slide-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}

/* Item animations */
.item-enter-active,
.item-leave-active {
  transition: all 0.35s cubic-bezier(0.4, 0, 0.2, 1);
}

.item-enter-from {
  opacity: 0;
  transform: translateX(-20px);
}

.item-leave-to {
  opacity: 0;
  transform: translateX(20px);
}

.item-move {
  transition: transform 0.35s cubic-bezier(0.4, 0, 0.2, 1);
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

@media (prefers-color-scheme: dark) {
  :root {
    --bg: rgba(30, 30, 40, 0.45);
    --surface: rgba(255, 255, 255, 0.03);
    --surface-2: rgba(255, 255, 255, 0.06);
    --text-primary: #e4e4e7;
    --text-secondary: #a1a1aa;
    --text-tertiary: #a1a1aa;
    --border: rgba(255, 255, 255, 0.08);
    --border-strong: #3f3f46;
    --accent: #fda4af;
    --on-accent: #3b1d26;
    --accent-glow: rgba(253, 164, 175, 0.12);
    --card-bg: #3b1d26;
    --card-hover: #4a2531;
    --card-border: #7c364c;
    --card-ink: #fff1f2;
    --card-muted: #e5bbc7;
    --card-quiet: #caa0ad;
    --fair-bg: #332914;
    --fair-hover: #44361b;
    --fair-border: #806222;
    --fair-ink: #fef3c7;
    --fair-muted: #e8cf92;
    --fair-quiet: #d4bb86;
    --fair-accent: #fcd34d;
    --fair-glow: rgba(252, 211, 77, 0.12);
  }
}
</style>
