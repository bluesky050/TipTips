import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { EmailConfig, ScanResult } from '../types'
import { useDataSync } from './useDataSync'

export type ScanStatus = 'idle' | 'scanning' | 'success' | 'error'

const { notifyDataChange } = useDataSync()

export function useEmailScan() {
  const config = ref<EmailConfig>({
    enabled: false,
    email_address: '',
    has_auth_code: false,
    has_gemini_api_key: false,
    scan_interval_minutes: 60,
    last_scan_time: '',
    processed_count: 0,
    gemini_model: 'gemini-3.8-flash'
  })

  const scanStatus = ref<ScanStatus>('idle')
  const lastMessage = ref('')
  const scanResult = ref<ScanResult | null>(null)

  const loadConfig = async () => {
    try {
      config.value = await invoke<EmailConfig>('get_email_config')
    } catch (e) {
      console.error('Failed to load email config:', e)
    }
  }

  const saveConfig = async (
    emailAddress: string,
    authCode: string | undefined,
    geminiApiKey: string | undefined,
    scanInterval: number,
    enabled: boolean,
    geminiModel: string
  ) => {
    try {
      await invoke('save_email_config', {
        emailAddress,
        authCode,
        geminiApiKey,
        scanIntervalMinutes: scanInterval,
        enabled,
        geminiModel
      })
      await loadConfig()
      return { success: true }
    } catch (e: any) {
      console.error('Failed to save email config:', e)
      return { success: false, error: e.toString() }
    }
  }

  const testConnection = async (emailAddress: string, authCode?: string) => {
    try {
      const result = await invoke<string>('test_email_connection', {
        emailAddress,
        authCode: authCode ? authCode : null
      })
      return { success: true, message: result }
    } catch (e: any) {
      return { success: false, error: e.toString() }
    }
  }

  const scanNow = async () => {
    if (scanStatus.value === 'scanning') return

    scanStatus.value = 'scanning'
    lastMessage.value = '正在扫描...'

    try {
      const result = await invoke<ScanResult>('scan_emails_now')
      scanResult.value = result
      if (result.success) {
        scanStatus.value = 'success'
        lastMessage.value = `扫描完成。扫描了 ${result.emails_scanned} 封邮件，创建了 ${result.items_created} 个待办事项。`

        if (result.items_created > 0) {
          notifyDataChange()
        }

        // Reset success status after a delay
        setTimeout(() => {
          if (scanStatus.value === 'success') {
            scanStatus.value = 'idle'
          }
        }, 5000)
      } else {
        scanStatus.value = 'error'
        lastMessage.value = `扫描失败: ${result.errors.join(', ')}`
      }

      await loadConfig() // Update stats
      return result
    } catch (e: any) {
      scanStatus.value = 'error'
      lastMessage.value = `扫描发生错误: ${e.toString()}`
      return null
    }
  }

  const setEnabled = async (enabled: boolean) => {
    try {
      await invoke('set_email_scan_enabled', { enabled })
      await loadConfig()
      return true
    } catch (e) {
      console.error('Failed to set enabled state:', e)
      return false
    }
  }

  const resetHistory = async () => {
    try {
      await invoke('reset_email_scan_history')
      await loadConfig()
      return true
    } catch (e) {
      console.error('Failed to reset history:', e)
      return false
    }
  }

  // Setup event listeners
  let unlistenComplete: (() => void) | null = null
  let unlistenStatus: (() => void) | null = null

  const setupListeners = async (onNewItems: () => void) => {
    unlistenComplete = await listen<ScanResult>('email-scan-complete', (event) => {
      scanResult.value = event.payload
      if (event.payload.success) {
        scanStatus.value = 'success'
        lastMessage.value = `后台扫描完成。创建了 ${event.payload.items_created} 个待办事项。`

        if (event.payload.items_created > 0) {
          notifyDataChange()
          onNewItems()
        }

        loadConfig() // Update stats

        setTimeout(() => {
          if (scanStatus.value === 'success') {
            scanStatus.value = 'idle'
          }
        }, 5000)
      } else {
        scanStatus.value = 'error'
        lastMessage.value = `后台扫描失败: ${event.payload.errors.join(', ')}`
      }
    })

    unlistenStatus = await listen<{ status: 'success' | 'error', message: string, time?: string }>('email-scan-status', (event) => {
      const { status, message } = event.payload
      scanStatus.value = status
      lastMessage.value = message

      if (status === 'success') {
        setTimeout(() => {
          if (scanStatus.value === 'success') {
            scanStatus.value = 'idle'
          }
        }, 5000)
      }
    })
  }

  const cleanupListeners = () => {
    if (unlistenComplete) unlistenComplete()
    if (unlistenStatus) unlistenStatus()
  }

  return {
    config,
    scanStatus,
    lastMessage,
    scanResult,
    loadConfig,
    saveConfig,
    testConnection,
    scanNow,
    setEnabled,
    resetHistory,
    setupListeners,
    cleanupListeners
  }
}
