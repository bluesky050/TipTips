<script setup lang="ts">
const props = defineProps<{
  mode: 'list' | 'calendar'
}>()

const emit = defineEmits<{
  switch: [mode: 'list' | 'calendar']
}>()
</script>

<template>
  <div class="view-switcher">
    <button
      class="switch-btn"
      :class="{ active: mode === 'list' }"
      @click="emit('switch', 'list')"
      title="列表视图"
    >
      <svg width="16" height="16" viewBox="0 0 16 16">
        <path d="M3 4h10M3 8h10M3 12h10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
      </svg>
    </button>
    <button
      class="switch-btn"
      :class="{ active: mode === 'calendar' }"
      @click="emit('switch', 'calendar')"
      title="日历视图"
    >
      <svg width="16" height="16" viewBox="0 0 16 16">
        <rect x="2" y="3" width="12" height="11" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.3" />
        <path d="M2 6h12M5 2v3M11 2v3" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" />
      </svg>
    </button>
  </div>
</template>

<style scoped>
.view-switcher {
  display: flex;
  gap: 4px;
  padding: 0 14px 8px;
  flex-shrink: 0;
}

.switch-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 28px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--surface);
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.2s;
}

.switch-btn:hover {
  background: var(--surface-2);
  color: var(--text-primary);
}

.switch-btn.active {
  background: var(--accent);
  border-color: var(--accent);
  color: white;
}
</style>
