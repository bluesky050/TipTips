import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { PlannerItem } from '../types'
import { useDataSync } from './useDataSync'

const items = ref<PlannerItem[]>([])
const loading = ref(false)
const { dataVersion, notifyDataChange } = useDataSync()

export function usePlanner() {
  // Auto-reload when another view changes data
  watch(dataVersion, () => {
    loadItems()
  })

  async function loadItems() {
    loading.value = true
    try {
      items.value = await invoke<PlannerItem[]>('get_items')
    } catch (e) {
      console.error('Failed to load items:', e)
    } finally {
      loading.value = false
    }
  }

  async function addItem(text: string, date?: string) {
    if (!text.trim()) return
    try {
      const item = await invoke<PlannerItem>('add_item', { text, date: date || null })
      items.value.push(item)
      notifyDataChange()
    } catch (e) {
      console.error('Failed to add item:', e)
    }
  }

  async function updateItem(id: string, text: string) {
    try {
      await invoke('update_item', { id, text })
      if (text.trim()) {
        const item = items.value.find((i) => i.id === id)
        if (item) item.text = text.trim()
      } else {
        items.value = items.value.filter((i) => i.id !== id)
      }
      notifyDataChange()
    } catch (e) {
      console.error('Failed to update item:', e)
    }
  }

  async function deleteItem(id: string) {
    try {
      await invoke('delete_item', { id })
      items.value = items.value.filter((i) => i.id !== id)
      notifyDataChange()
    } catch (e) {
      console.error('Failed to delete item:', e)
    }
  }

  async function toggleItem(id: string) {
    try {
      const updated = await invoke<PlannerItem>('toggle_item', { id })
      const item = items.value.find((i) => i.id === id)
      if (item) {
        item.done = updated.done
      }
      notifyDataChange()
    } catch (e) {
      console.error('Failed to toggle item:', e)
    }
  }

  async function checkRollover() {
    try {
      const changed = await invoke<boolean>('check_day_rollover')
      if (changed) {
        await loadItems()
      }
    } catch (e) {
      console.error('Failed to check rollover:', e)
    }
  }

  async function reorderItems(orderedIds: string[]) {
    try {
      await invoke('reorder_items', { orderedIds })
    } catch (e) {
      console.error('Failed to reorder items:', e)
    }
  }

  return {
    items,
    loading,
    loadItems,
    addItem,
    updateItem,
    deleteItem,
    toggleItem,
    checkRollover,
    reorderItems,
  }
}
