import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { PlannerItem } from '../types'
import { useDataSync } from './useDataSync'

const currentYear = ref(new Date().getFullYear())
const currentMonth = ref(new Date().getMonth()) // 0-11
const monthItems = ref<PlannerItem[]>([])
const selectedDate = ref<string | null>(null)
const loading = ref(false)
const { dataVersion, notifyDataChange } = useDataSync()

export function useCalendar() {
  watch(dataVersion, () => {
    loadMonthData()
  })
  const monthLabel = computed(() => {
    return `${currentYear.value}年${currentMonth.value + 1}月`
  })

  const todayStr = computed(() => {
    // Recompute the local today marker when a date boundary refreshes both views.
    void dataVersion.value
    const d = new Date()
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
  })

  // Build the calendar grid: array of { date: string | null, day: number }
  const calendarDays = computed(() => {
    const year = currentYear.value
    const month = currentMonth.value
    const firstDay = new Date(year, month, 1)
    const lastDay = new Date(year, month + 1, 0)
    const daysInMonth = lastDay.getDate()
    // Monday = 0, Sunday = 6
    let startWeekday = firstDay.getDay() - 1
    if (startWeekday < 0) startWeekday = 6

    const days: { date: string | null; day: number | null }[] = []
    // Leading blanks
    for (let i = 0; i < startWeekday; i++) {
      days.push({ date: null, day: null })
    }
    // Actual days
    for (let d = 1; d <= daysInMonth; d++) {
      const dateStr = `${year}-${String(month + 1).padStart(2, '0')}-${String(d).padStart(2, '0')}`
      days.push({ date: dateStr, day: d })
    }
    return days
  })

  // Group items by date for quick lookup
  const itemsByDate = computed(() => {
    const map: Record<string, PlannerItem[]> = {}
    for (const item of monthItems.value) {
      if (!map[item.created_date]) map[item.created_date] = []
      map[item.created_date].push(item)
    }
    return map
  })

  // Stats per date: { total, done }
  const statsByDate = computed(() => {
    const map: Record<string, { total: number; done: number; fairs: number }> = {}
    for (const [date, items] of Object.entries(itemsByDate.value)) {
      map[date] = {
        total: items.filter(i => i.source !== 'seu').length,
        done: items.filter((i) => i.done).length,
        fairs: items.filter(i => i.source === 'seu').length,
      }
    }
    return map
  })

  const selectedDateItems = computed(() => {
    if (!selectedDate.value) return []
    return itemsByDate.value[selectedDate.value] || []
  })

  async function loadMonthData() {
    loading.value = true
    const year = currentYear.value
    const month = currentMonth.value
    const startDate = `${year}-${String(month + 1).padStart(2, '0')}-01`
    const lastDay = new Date(year, month + 1, 0).getDate()
    const endDate = `${year}-${String(month + 1).padStart(2, '0')}-${String(lastDay).padStart(2, '0')}`
    try {
      monthItems.value = await invoke<PlannerItem[]>('get_items_by_date_range', {
        startDate,
        endDate,
      })
    } catch (e) {
      console.error('Failed to load month data:', e)
      monthItems.value = []
    } finally {
      loading.value = false
    }
  }

  function prevMonth() {
    if (currentMonth.value === 0) {
      currentMonth.value = 11
      currentYear.value--
    } else {
      currentMonth.value--
    }
    selectedDate.value = null
    loadMonthData()
  }

  function nextMonth() {
    if (currentMonth.value === 11) {
      currentMonth.value = 0
      currentYear.value++
    } else {
      currentMonth.value++
    }
    selectedDate.value = null
    loadMonthData()
  }

  function selectDate(date: string) {
    selectedDate.value = selectedDate.value === date ? null : date
  }

  async function addItemToDate(text: string, date: string) {
    if (!text.trim()) return
    try {
      const item = await invoke<PlannerItem>('add_item', { text, date })
      monthItems.value.push(item)
      notifyDataChange()
    } catch (e) {
      console.error('Failed to add item to date:', e)
    }
  }

  async function toggleItem(id: string) {
    try {
      const updated = await invoke<PlannerItem>('toggle_item', { id })
      const item = monthItems.value.find((i) => i.id === id)
      if (item) item.done = updated.done
      notifyDataChange()
    } catch (e) {
      console.error('Failed to toggle item:', e)
    }
  }

  async function deleteItem(id: string) {
    try {
      await invoke('delete_item', { id })
      monthItems.value = monthItems.value.filter((i) => i.id !== id)
      notifyDataChange()
    } catch (e) {
      console.error('Failed to delete item:', e)
    }
  }

  async function updateItem(id: string, text: string) {
    try {
      await invoke('update_item', { id, text })
      if (text.trim()) {
        const item = monthItems.value.find((i) => i.id === id)
        if (item) item.text = text.trim()
      } else {
        monthItems.value = monthItems.value.filter((i) => i.id !== id)
      }
      notifyDataChange()
      await loadMonthData()
    } catch (e) {
      console.error('Failed to update item:', e)
    }
  }

  return {
    currentYear,
    currentMonth,
    monthLabel,
    todayStr,
    calendarDays,
    statsByDate,
    selectedDate,
    selectedDateItems,
    loading,
    loadMonthData,
    prevMonth,
    nextMonth,
    selectDate,
    addItemToDate,
    toggleItem,
    deleteItem,
    updateItem,
  }
}
