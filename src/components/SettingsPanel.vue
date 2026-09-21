<script setup lang="ts">
const props = defineProps<{
  opacity: number
}>()

const emit = defineEmits<{
  'update:opacity': [value: number]
  close: []
}>()

function onInput(e: Event) {
  const target = e.target as HTMLInputElement
  emit('update:opacity', parseFloat(target.value))
}
</script>

<template>
  <Transition name="settings-fade">
    <div class="settings-panel" @click.stop>
      <div class="settings-header">
        <span class="settings-title">设置</span>
        <button class="settings-close" @click="emit('close')">
          <svg width="12" height="12" viewBox="0 0 12 12">
            <path d="M3 3L9 9M9 3L3 9" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
          </svg>
        </button>
      </div>

      <div class="setting-row">
        <span class="setting-label">窗口透明度</span>
        <span class="setting-value">{{ Math.round(opacity * 100) }}%</span>
      </div>
      <input
        type="range"
        min="0.1"
        max="0.95"
        step="0.05"
        :value="opacity"
        class="opacity-slider"
        @input="onInput"
      />
      <div class="slider-hints">
        <span>透明</span>
        <span>不透明</span>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.settings-panel {
  position: absolute;
  top: 40px;
  right: 10px;
  width: 200px;
  padding: 12px 14px;
  border-radius: 12px;
  background: var(--surface-2);
  backdrop-filter: blur(30px) saturate(180%);
  -webkit-backdrop-filter: blur(30px) saturate(180%);
  border: 1px solid var(--border);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.15);
  z-index: 100;
}

.settings-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.settings-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}

.settings-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border: none;
  border-radius: 5px;
  background: transparent;
  color: var(--text-tertiary);
  cursor: pointer;
  transition: all 0.2s;
}

.settings-close:hover {
  background: var(--border);
  color: var(--text-primary);
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 6px;
}

.setting-label {
  font-size: 12px;
  color: var(--text-secondary);
}

.setting-value {
  font-size: 12px;
  font-weight: 600;
  color: var(--accent);
}

.opacity-slider {
  width: 100%;
  height: 6px;
  -webkit-appearance: none;
  appearance: none;
  border-radius: 3px;
  background: var(--border-strong);
  outline: none;
  cursor: pointer;
}

.opacity-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--accent);
  cursor: pointer;
  border: 2px solid white;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.2);
}

.slider-hints {
  display: flex;
  justify-content: space-between;
  margin-top: 4px;
}

.slider-hints span {
  font-size: 10px;
  color: var(--text-tertiary);
}

.settings-fade-enter-active,
.settings-fade-leave-active {
  transition: opacity 0.2s, transform 0.2s;
}

.settings-fade-enter-from,
.settings-fade-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}
</style>
