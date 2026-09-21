<script setup lang="ts">
import { ref, nextTick } from 'vue'
import type { PlannerItem } from '../types'

const props = defineProps<{
  item: PlannerItem
  canMoveUp?: boolean
  canMoveDown?: boolean
}>()

const emit = defineEmits<{
  toggle: [id: string]
  update: [id: string, text: string]
  delete: [id: string]
  moveUp: [id: string]
  moveDown: [id: string]
}>()

const isEditing = ref(false)
const editText = ref('')
const editInputRef = ref<HTMLInputElement | null>(null)

function startEdit() {
  isEditing.value = true
  editText.value = props.item.text
  nextTick(() => {
    editInputRef.value?.focus()
    editInputRef.value?.select()
  })
}

function confirmEdit() {
  isEditing.value = false
  if (editText.value.trim() !== props.item.text) {
    emit('update', props.item.id, editText.value)
  }
}

function cancelEdit() {
  isEditing.value = false
  editText.value = props.item.text
}

function handleEditKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter') {
    confirmEdit()
  } else if (e.key === 'Escape') {
    cancelEdit()
  }
}
</script>

<template>
  <div class="planner-item" :class="{ done: item.done, carried: item.carried_over }">
    <button class="checkbox" @click="emit('toggle', item.id)" :class="{ checked: item.done }">
      <svg v-if="item.done" width="14" height="14" viewBox="0 0 14 14">
        <path d="M2 7L5.5 10.5L12 3.5" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </button>

    <div class="item-content" v-if="!isEditing" @dblclick="startEdit">
      <span class="item-text">{{ item.text }}</span>
      <span v-if="item.carried_over" class="carried-badge">延续</span>
    </div>

    <input
      v-else
      ref="editInputRef"
      v-model="editText"
      class="edit-input"
      type="text"
      @blur="confirmEdit"
      @keydown="handleEditKeydown"
    />

    <div class="item-actions" v-if="!isEditing">
      <button
        v-if="canMoveUp"
        class="action-btn move"
        @click="emit('moveUp', item.id)"
        title="上移"
      >
        <svg width="12" height="12" viewBox="0 0 12 12">
          <path d="M6 2L2 7H10L6 2Z" fill="currentColor" />
        </svg>
      </button>
      <button
        v-if="canMoveDown"
        class="action-btn move"
        @click="emit('moveDown', item.id)"
        title="下移"
      >
        <svg width="12" height="12" viewBox="0 0 12 12">
          <path d="M6 10L2 5H10L6 10Z" fill="currentColor" />
        </svg>
      </button>
      <button class="action-btn edit" @click="startEdit" title="编辑">
        <svg width="13" height="13" viewBox="0 0 13 13">
          <path d="M8.5 1.5L11.5 4.5L4 12L1 12L1 9L8.5 1.5Z" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" />
        </svg>
      </button>
      <button class="action-btn delete" @click="emit('delete', item.id)" title="删除">
        <svg width="13" height="13" viewBox="0 0 13 13">
          <path d="M2.5 3.5H10.5M4.5 3.5V2H8.5V3.5M3.5 3.5L4 11H9L9.5 3.5" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
.planner-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 14px;
  border-radius: 10px;
  transition: background 0.2s;
  width: 100%;
}

.planner-item:hover {
  background: var(--surface-2);
}

.planner-item:not(.done) {
  background: rgba(239, 68, 68, 0.08);
}

.planner-item:not(.done):hover {
  background: rgba(239, 68, 68, 0.14);
}

.planner-item.done {
  background: rgba(34, 197, 94, 0.1);
}

.planner-item.done:hover {
  background: rgba(34, 197, 94, 0.16);
}

.checkbox {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border: 2px solid var(--border-strong);
  border-radius: 6px;
  background: transparent;
  cursor: pointer;
  flex-shrink: 0;
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.checkbox.checked {
  background: var(--accent);
  border-color: var(--accent);
}

.item-content {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.item-text {
  font-size: 13px;
  color: var(--text-primary);
  line-height: 1.4;
  word-break: break-word;
  transition: color 0.3s, text-decoration-color 0.3s;
}

.done .item-text {
  text-decoration: line-through;
  text-decoration-color: var(--text-tertiary);
  color: var(--text-tertiary);
}

.carried-badge {
  font-size: 10px;
  color: var(--accent);
  background: var(--accent-glow);
  padding: 1px 6px;
  border-radius: 4px;
  flex-shrink: 0;
}

.edit-input {
  flex: 1;
  height: 28px;
  padding: 0 8px;
  border: 1px solid var(--accent);
  border-radius: 6px;
  background: var(--surface);
  color: var(--text-primary);
  font-size: 13px;
  outline: none;
}

.item-actions {
  display: flex;
  gap: 2px;
  opacity: 0;
  transition: opacity 0.2s;
  flex-shrink: 0;
}

.planner-item:hover .item-actions {
  opacity: 1;
}

.action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border: none;
  border-radius: 5px;
  background: transparent;
  cursor: pointer;
  transition: background 0.2s, color 0.2s;
}

.action-btn.move {
  color: var(--text-tertiary);
}

.action-btn.move:hover {
  background: var(--accent-glow);
  color: var(--accent);
}

.action-btn.edit {
  color: var(--text-secondary);
}

.action-btn.edit:hover {
  background: var(--accent-glow);
  color: var(--accent);
}

.action-btn.delete {
  color: var(--text-secondary);
}

.action-btn.delete:hover {
  background: var(--danger-bg);
  color: var(--danger);
}
</style>
