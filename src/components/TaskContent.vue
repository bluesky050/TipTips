<script setup lang="ts">
import { computed, ref } from 'vue'
import { openUrl } from '@tauri-apps/plugin-opener'
import type { PlannerItem } from '../types'

const props = defineProps<{ item: PlannerItem }>()
const schedule = computed(() => props.item.schedule)
const shorten = (text: string, limit: number) => {
  const chars = Array.from(text)
  return chars.length > limit ? chars.slice(0, limit).join('') + '…' : text
}
const titlePreview = computed(() => shorten(props.item.text, 60))
const notesPreview = computed(() => shorten(schedule.value?.notes || '', 80))
const linkError = ref(false)
const safeLink = computed(() => {
  try {
    const url = new URL(schedule.value?.link || '')
    return ['https:', 'http:'].includes(url.protocol) ? url.href : ''
  } catch { return '' }
})
const timeLabel = computed(() => {
  const s = schedule.value
  if (!s?.date) return '时间待确认'
  const start = [s.date, s.time].filter(Boolean).join(' ')
  if (s.time_kind === 'window' && s.end_date) {
    const end = [s.end_date === s.date ? '' : s.end_date, s.end_time].filter(Boolean).join(' ') || s.end_date
    return `${start} 至 ${end} · ${props.item.source === 'seu' ? '宣讲时间' : '可参加时间段'}`
  }
  const kind = s.time_kind === 'start' ? '开始' : s.time_kind === 'deadline' ? '截止' : '时间含义待确认'
  return `${start} · ${kind}`
})
async function openLink() {
  linkError.value = false
  try { await openUrl(safeLink.value) }
  catch { linkError.value = true }
}
</script>

<template>
  <div class="task-content" :class="{ completed: item.done }">
    <div class="task-title">{{ titlePreview }}</div>
    <div v-if="item.source === 'seu'" class="task-meta official-source">东南大学就业网 · 官网日程</div>
    <details v-if="titlePreview !== item.text" class="task-source" @dblclick.stop @click.stop>
      <summary>完整文本</summary>
      <div class="evidence">{{ item.text }}</div>
    </details>
    <template v-if="schedule">
      <div class="task-meta">
        {{ timeLabel }}<span v-if="schedule.time && schedule.timezone"> · {{ schedule.timezone }}</span>
      </div>
      <div v-if="schedule.location" class="task-meta">地点：{{ shorten(schedule.location, 50) }}</div>
      <div v-else-if="item.source === 'seu'" class="task-meta">地点待公布</div>
      <div v-if="schedule.notes" class="task-meta">备注：{{ notesPreview }}</div>
      <details v-if="notesPreview !== schedule.notes || Array.from(schedule.location).length > 50" class="task-source" @dblclick.stop @click.stop>
        <summary>完整备注与地点</summary>
        <div v-if="schedule.location">地点：{{ schedule.location }}</div>
        <div class="evidence">{{ schedule.notes }}</div>
      </details>
      <div class="task-tools" @dblclick.stop>
        <span v-if="schedule.needs_confirmation" class="confirmation">待确认</span>
        <button v-if="safeLink" type="button" class="task-link" @click.stop="openLink" :title="safeLink">{{ item.source === 'seu' ? '查看官网详情' : '打开活动链接' }}</button>
        <span v-if="linkError" class="confirmation">链接打开失败</span>
      </div>
      <details class="task-source" @dblclick.stop @click.stop>
        <summary>{{ item.source === 'seu' ? '官网信息' : '邮件依据' }}</summary>
        <div>{{ schedule.source_subject }}</div>
        <div>{{ schedule.source_date }}</div>
        <div v-if="schedule.time_evidence" class="evidence">{{ schedule.time_evidence }}</div>
      </details>
    </template>
  </div>
</template>

<style scoped>
.task-content { flex: 1; min-width: 0; overflow-wrap: anywhere; }
.task-title { font-size: 14px; font-weight: 600; color: var(--card-ink); line-height: 1.65; }
.completed .task-title { text-decoration: line-through; text-decoration-thickness: 1px; color: var(--card-muted); }
.task-meta { font-size: 12px; color: var(--card-muted); line-height: 1.65; margin-top: 4px; }
.task-tools { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; margin-top: 3px; }
.confirmation { font-size: 11px; color: var(--accent); }
.task-link { border: 0; padding: 0; background: transparent; color: var(--accent); cursor: pointer; font-size: 12px; font-weight: 500; text-decoration: underline; text-underline-offset: 3px; }
.task-source { margin-top: 5px; font-size: 11px; line-height: 1.65; color: var(--card-quiet); }
.task-source summary { cursor: pointer; }
.evidence { white-space: pre-wrap; }
.official-source { color: var(--accent); }
</style>
