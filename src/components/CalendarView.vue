<script setup lang="ts">
import { onMounted, ref, nextTick } from 'vue'
import { useCalendar } from '../composables/useCalendar'

const {
  monthLabel,
  todayStr,
  calendarDays,
  statsByDate,
  selectedDate,
  selectedDateItems,
  loadMonthData,
  prevMonth,
  nextMonth,
  selectDate,
  addItemToDate,
  toggleItem,
  deleteItem,
  updateItem,
} = useCalendar()

onMounted(() => {
  loadMonthData()
})

const weekdays = ['一', '二', '三', '四', '五', '六', '日']

// Two modes: 'grid' = calendar overview, 'detail' = full-screen date detail
const viewMode = ref<'grid' | 'detail'>('grid')

// Input for adding items
const newItemText = ref('')
const editingId = ref<string | null>(null)
const editText = ref('')
const editInputRef = ref<HTMLInputElement | null>(null)

function openDateDetail(date: string) {
  selectDate(date)
  viewMode.value = 'detail'
}

function backToGrid() {
  viewMode.value = 'grid'
  selectedDate.value = null
}

function handleAdd() {
  if (!newItemText.value.trim() || !selectedDate.value) return
  addItemToDate(newItemText.value, selectedDate.value)
  newItemText.value = ''
}

function handleAddKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter') handleAdd()
}

function startEdit(id: string, text: string) {
  editingId.value = id
  editText.value = text
  nextTick(() => {
    editInputRef.value?.focus()
    editInputRef.value?.select()
  })
}

function confirmEdit() {
  if (editingId.value) {
    updateItem(editingId.value, editText.value)
  }
  editingId.value = null
}

function cancelEdit() {
  editingId.value = null
}

function handleEditKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter') confirmEdit()
  else if (e.key === 'Escape') cancelEdit()
}

function cellStatus(date: string): 'all-done' | 'has-undone' | 'empty' {
  const stats = statsByDate.value[date]
  if (!stats || stats.total === 0) return 'empty'
  if (stats.done === stats.total) return 'all-done'
  return 'has-undone'
}

// Format date for display: "9月18日 周四"
function formatDateLabel(dateStr: string): string {
  const d = new Date(dateStr + 'T00:00:00')
  const weekdaysFull = ['周日', '周一', '周二', '周三', '周四', '周五', '周六']
  return `${d.getMonth() + 1}月${d.getDate()}日 ${weekdaysFull[d.getDay()]}`
}
</script>

<template>
  <div class="calendar-view">
    <Transition name="view-flip" mode="out-in">
      <!-- ===== GRID MODE: Month calendar overview ===== -->
      <div v-if="viewMode === 'grid'" key="grid" class="grid-mode">
        <!-- Month navigation -->
        <div class="month-nav">
          <button class="nav-btn" @click="prevMonth" title="上一月">
            <svg width="14" height="14" viewBox="0 0 14 14">
              <path d="M9 3L5 7L9 11" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
          </button>
          <span class="month-label">{{ monthLabel }}</span>
          <button class="nav-btn" @click="nextMonth" title="下一月">
            <svg width="14" height="14" viewBox="0 0 14 14">
              <path d="M5 3L9 7L5 11" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
          </button>
        </div>

        <!-- Weekday headers -->
        <div class="weekday-row">
          <div v-for="w in weekdays" :key="w" class="weekday-cell">{{ w }}</div>
        </div>

        <!-- Calendar grid -->
        <Transition name="calendar-fade" mode="out-in">
          <div class="calendar-grid" :key="monthLabel">
            <div
              v-for="(day, i) in calendarDays"
              :key="i"
              class="day-cell"
              :class="{
                empty: !day.date,
                today: day.date === todayStr,
                'all-done': day.date && cellStatus(day.date) === 'all-done',
                'has-undone': day.date && cellStatus(day.date) === 'has-undone',
              }"
              @click="day.date && openDateDetail(day.date)"
            >
              <template v-if="day.date">
                <span class="day-number">{{ day.day }}</span>
                <template v-if="statsByDate[day.date]">
                  <span class="day-stats">{{ statsByDate[day.date].done }}/{{ statsByDate[day.date].total }}</span>
                  <div class="day-progress">
                    <div
                      class="day-progress-bar"
                      :class="{ full: statsByDate[day.date].done === statsByDate[day.date].total }"
                      :style="{ width: (statsByDate[day.date].done / statsByDate[day.date].total) * 100 + '%' }"
                    />
                  </div>
                </template>
              </template>
            </div>
          </div>
        </Transition>
      </div>

      <!-- ===== DETAIL MODE: Full-screen date detail ===== -->
      <div v-else key="detail" class="detail-mode">
        <!-- Back header -->
        <div class="detail-top-bar">
          <button class="back-btn" @click="backToGrid" title="返回日历">
            <svg width="16" height="16" viewBox="0 0 16 16">
              <path d="M10 3L5 8L10 13" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
          </button>
          <span class="detail-title">{{ selectedDate ? formatDateLabel(selectedDate) : '' }}</span>
          <span v-if="selectedDate === todayStr" class="today-badge">今天</span>
        </div>

        <!-- Add input -->
        <div class="detail-add">
          <input
            v-model="newItemText"
            type="text"
            class="detail-input"
            :placeholder="`添加规划...`"
            @keydown="handleAddKeydown"
          />
          <button class="detail-add-btn" @click="handleAdd" :disabled="!newItemText.trim()">
            <svg width="18" height="18" viewBox="0 0 18 18">
              <path d="M9 3v12M3 9h12" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
            </svg>
          </button>
        </div>

        <!-- Items list (editable, same style as list view) -->
        <TransitionGroup v-if="selectedDateItems.length" name="item" tag="div" class="detail-items">
          <div
            v-for="item in selectedDateItems"
            :key="item.id"
            class="detail-item"
            :class="{ done: item.done }"
          >
            <button class="item-check" @click="toggleItem(item.id)">
              <svg v-if="item.done" width="14" height="14" viewBox="0 0 14 14">
                <path d="M2 7L5.5 10.5L12 3.5" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
              </svg>
            </button>
            <input
              v-if="editingId === item.id"
              ref="editInputRef"
              v-model="editText"
              class="item-edit-input"
              type="text"
              @blur="confirmEdit"
              @keydown="handleEditKeydown"
            />
            <span v-else class="item-text" @dblclick="startEdit(item.id, item.text)">{{ item.text }}</span>
            <span v-if="item.carried_over" class="item-carried">延续</span>
            <div class="item-actions">
              <button class="action-btn" @click="startEdit(item.id, item.text)" title="编辑">
                <svg width="13" height="13" viewBox="0 0 13 13">
                  <path d="M8.5 1.5L11.5 4.5L4 12L1 12L1 9L8.5 1.5Z" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" />
                </svg>
              </button>
              <button class="action-btn delete" @click="deleteItem(item.id)" title="删除">
                <svg width="13" height="13" viewBox="0 0 13 13">
                  <path d="M2.5 3.5H10.5M4.5 3.5V2H8.5V3.5M3.5 3.5L4 11H9L9.5 3.5" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" />
                </svg>
              </button>
            </div>
          </div>
        </TransitionGroup>

        <div v-else class="detail-empty">
          <svg width="40" height="40" viewBox="0 0 40 40" class="empty-icon">
            <rect x="6" y="8" width="28" height="26" rx="3" fill="none" stroke="currentColor" stroke-width="1.5" />
            <path d="M12 16h16M12 22h16M12 28h10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
          </svg>
          <p>还没有规划</p>
          <p class="empty-hint">在上方添加你的安排</p>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.calendar-view {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
  padding: 0 14px 10px;
}

/* === GRID MODE === */
.grid-mode {
  display: flex;
  flex-direction: column;
  flex: 1;
}

.month-nav {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 0 8px;
  flex-shrink: 0;
}

.nav-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--surface);
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.2s;
}

.nav-btn:hover {
  background: var(--surface-2);
  color: var(--text-primary);
}

.month-label {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.weekday-row {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 2px;
  margin-bottom: 4px;
  flex-shrink: 0;
}

.weekday-cell {
  text-align: center;
  font-size: 11px;
  font-weight: 500;
  color: var(--text-tertiary);
  padding: 4px 0;
}

.calendar-grid {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 2px;
  flex-shrink: 0;
}

.day-cell {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: flex-start;
  padding: 4px 2px;
  min-height: 44px;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.2s, border-color 0.2s;
  border: 1.5px solid transparent;
}

.day-cell:hover:not(.empty) {
  border-color: var(--border-strong);
}

.day-cell.empty {
  cursor: default;
}

.day-cell.today {
  border-color: var(--accent);
}

.day-cell.all-done {
  background: rgba(34, 197, 94, 0.12);
}

.day-cell.has-undone {
  background: rgba(239, 68, 68, 0.1);
}

.day-number {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-primary);
  line-height: 1.2;
}

.day-stats {
  font-size: 9px;
  color: var(--text-tertiary);
  margin-top: 1px;
}

.day-progress {
  width: 80%;
  height: 3px;
  border-radius: 2px;
  background: var(--border);
  margin-top: 2px;
  overflow: hidden;
}

.day-progress-bar {
  height: 100%;
  border-radius: 2px;
  background: var(--danger);
  transition: width 0.3s;
}

.day-progress-bar.full {
  background: #22c55e;
}

/* === DETAIL MODE === */
.detail-mode {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
}

.detail-top-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 0 10px;
  flex-shrink: 0;
}

.back-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--surface);
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.2s;
}

.back-btn:hover {
  background: var(--surface-2);
  color: var(--text-primary);
}

.detail-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
}

.today-badge {
  font-size: 11px;
  color: var(--accent);
  background: var(--accent-glow);
  padding: 2px 8px;
  border-radius: 6px;
}

.detail-add {
  display: flex;
  gap: 8px;
  padding-bottom: 10px;
  flex-shrink: 0;
}

.detail-input {
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

.detail-input::placeholder {
  color: var(--text-tertiary);
}

.detail-input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-glow);
}

.detail-add-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 38px;
  height: 38px;
  border: none;
  border-radius: 10px;
  background: var(--accent);
  color: white;
  cursor: pointer;
  transition: opacity 0.2s, transform 0.15s;
  flex-shrink: 0;
}

.detail-add-btn:hover:not(:disabled) {
  transform: scale(1.05);
}

.detail-add-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

/* Items list (mirrors PlannerItem style) */
.detail-items {
  flex: 1;
  overflow-y: auto;
}

.detail-items::-webkit-scrollbar {
  width: 4px;
}

.detail-items::-webkit-scrollbar-thumb {
  background: var(--border-strong);
  border-radius: 2px;
}

.detail-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-radius: 10px;
  transition: background 0.2s;
}

/* 未完成 — 浅红背景 */
.detail-item:not(.done) {
  background: rgba(239, 68, 68, 0.08);
}

.detail-item:not(.done):hover {
  background: rgba(239, 68, 68, 0.14);
}

/* 已完成 — 浅绿背景 */
.detail-item.done {
  background: rgba(34, 197, 94, 0.1);
}

.detail-item.done:hover {
  background: rgba(34, 197, 94, 0.16);
}

.item-check {
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

.detail-item.done .item-check {
  background: #22c55e;
  border-color: #22c55e;
}

.item-edit-input {
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

.item-text {
  font-size: 13px;
  color: var(--text-primary);
  flex: 1;
  cursor: pointer;
  word-break: break-word;
}

.detail-item.done .item-text {
  text-decoration: line-through;
  color: var(--text-tertiary);
}

.item-carried {
  font-size: 10px;
  color: var(--accent);
  background: var(--accent-glow);
  padding: 1px 6px;
  border-radius: 4px;
  flex-shrink: 0;
}

.item-actions {
  display: flex;
  gap: 4px;
  opacity: 0;
  transition: opacity 0.2s;
  flex-shrink: 0;
}

.detail-item:hover .item-actions {
  opacity: 1;
}

.action-btn {
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

.action-btn:hover {
  background: var(--accent-glow);
  color: var(--accent);
}

.action-btn.delete:hover {
  background: var(--danger-bg);
  color: var(--danger);
}

.detail-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  color: var(--text-tertiary);
  font-size: 13px;
}

.empty-icon {
  opacity: 0.4;
}

.empty-hint {
  font-size: 12px;
  color: var(--text-tertiary);
}

/* === ANIMATIONS === */
.view-flip-enter-active,
.view-flip-leave-active {
  transition: opacity 0.2s, transform 0.2s;
}

.view-flip-enter-from {
  opacity: 0;
  transform: translateX(16px);
}

.view-flip-leave-to {
  opacity: 0;
  transform: translateX(-16px);
}

.calendar-fade-enter-active,
.calendar-fade-leave-active {
  transition: opacity 0.25s, transform 0.25s;
}

.calendar-fade-enter-from {
  opacity: 0;
  transform: translateX(10px);
}

.calendar-fade-leave-to {
  opacity: 0;
  transform: translateX(-10px);
}

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
</style>
