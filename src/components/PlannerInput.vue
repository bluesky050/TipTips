<script setup lang="ts">
import { ref } from 'vue'

const emit = defineEmits<{
  add: [text: string]
}>()

const inputText = ref('')

function handleAdd() {
  if (!inputText.value.trim()) return
  emit('add', inputText.value)
  inputText.value = ''
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter') {
    handleAdd()
  }
}
</script>

<template>
  <div class="input-container">
    <input
      v-model="inputText"
      type="text"
      class="planner-input"
      placeholder="添加今日规划..."
      @keydown="handleKeydown"
    />
    <button class="add-btn" @click="handleAdd" :disabled="!inputText.trim()">
      <svg width="18" height="18" viewBox="0 0 18 18">
        <path d="M9 3v12M3 9h12" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
      </svg>
    </button>
  </div>
</template>

<style scoped>
.input-container {
  display: flex;
  gap: 8px;
  padding: 0 14px 10px;
  flex-shrink: 0;
}

.planner-input {
  flex: 1;
  height: 38px;
  padding: 0 12px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface-2);
  color: var(--text-primary);
  font-size: 13px;
  outline: none;
  transition: border-color 0.2s, box-shadow 0.2s;
}

.planner-input::placeholder {
  color: var(--text-tertiary);
}

.planner-input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-glow);
}

.add-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 38px;
  height: 38px;
  border: none;
  border-radius: 10px;
  background: var(--accent);
  color: var(--on-accent);
  cursor: pointer;
  transition: opacity 0.2s, transform 0.15s;
  flex-shrink: 0;
}

.add-btn:hover:not(:disabled) {
  transform: scale(1.05);
}

.add-btn:active:not(:disabled) {
  transform: scale(0.95);
}

.add-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
</style>
