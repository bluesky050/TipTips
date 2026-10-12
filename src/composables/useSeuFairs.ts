import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useDataSync } from './useDataSync'

interface FairStatus { updated_at: string; count: number }
const status = ref<FairStatus>({ updated_at: '', count: 0 })
const refreshing = ref(false)
const error = ref('')
export function beijingDate(date = new Date()) {
  const parts = new Intl.DateTimeFormat('en', { timeZone: 'Asia/Shanghai', year: 'numeric', month: '2-digit', day: '2-digit' }).formatToParts(date)
  return ['year', 'month', 'day'].map(type => parts.find(part => part.type === type)!.value).join('-')
}
const officialDay = ref(beijingDate())

export function useSeuFairs() {
  const { notifyDataChange } = useDataSync()
  async function loadStatus() {
    try { status.value = await invoke<FairStatus>('get_seu_fair_status') }
    catch { error.value = '官网缓存异常，请刷新重试' }
  }
  async function refresh() {
    if (refreshing.value) return
    refreshing.value = true
    error.value = ''
    try {
      status.value = await invoke<FairStatus>('refresh_seu_fairs')
      notifyDataChange()
    } catch {
      error.value = status.value.updated_at ? '同步失败，保留上次结果' : '同步失败，暂无官网日程'
    } finally { refreshing.value = false }
  }
  function checkDayBoundary(date = new Date()) {
    const day = beijingDate(date)
    if (day === officialDay.value) return false
    officialDay.value = day
    notifyDataChange()
    void loadStatus()
    return true
  }
  return { status, refreshing, error, officialDay, loadStatus, refresh, checkDayBoundary }
}
