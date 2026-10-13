<template>
  <div class="email-settings-overlay">
    <div class="settings-header">
      <button class="back-btn" @click="$emit('close')" title="返回">
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="icon"><path d="M19 12H5"/><path d="M12 19l-7-7 7-7"/></svg>
      </button>
      <h2>邮件扫描设置</h2>
    </div>

    <div class="settings-content">
      <div class="setting-group">
        <div class="setting-row">
          <label class="setting-label">启用邮件扫描</label>
          <label class="switch">
            <input type="checkbox" v-model="form.enabled" @change="onEnabledChange" />
            <span class="slider"></span>
          </label>
        </div>
      </div>

      <div class="setting-group" :class="{ disabled: !form.enabled }">
        <div class="setting-row vertical">
          <label class="setting-label">QQ邮箱地址</label>
          <input type="email" v-model="form.email_address" class="setting-input" placeholder="example@qq.com" :disabled="!form.enabled" />
        </div>

        <div class="setting-row vertical">
          <label class="setting-label">授权码</label>
          <div class="password-input">
            <input :type="showAuthCode ? 'text' : 'password'" v-model="form.auth_code" class="setting-input" :placeholder="config.has_auth_code ? '已保存 (输入新密码以覆盖)' : '输入QQ邮箱授权码'" :disabled="!form.enabled" />
            <button class="toggle-visibility" @click="showAuthCode = !showAuthCode" :disabled="!form.enabled">
              <svg v-if="showAuthCode" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="icon"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"/><line x1="1" y1="1" x2="23" y2="23"/></svg>
              <svg v-else xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="icon"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
            </button>
          </div>
          <span class="hint-text">在QQ邮箱设置中开启IMAP服务后获取</span>
        </div>

        <div class="setting-row vertical">
          <label class="setting-label">Gemini API Key</label>
          <div class="password-input">
            <input :type="showApiKey ? 'text' : 'password'" v-model="form.gemini_api_key" class="setting-input" :placeholder="config.has_gemini_api_key ? '已保存 (输入新Key以覆盖)' : '输入Gemini API Key'" :disabled="!form.enabled" />
            <button class="toggle-visibility" @click="showApiKey = !showApiKey" :disabled="!form.enabled">
              <svg v-if="showApiKey" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="icon"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"/><line x1="1" y1="1" x2="23" y2="23"/></svg>
              <svg v-else xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="icon"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
            </button>
          </div>
        </div>

        <div class="setting-row">
          <label class="setting-label">Gemini 模型</label>
          <select v-model="form.gemini_model" class="setting-select" :disabled="!form.enabled">
            <option value="gemini-3.8-flash">gemini-3.8-flash (推荐)</option>
            <option value="gemini-2.5-flash">gemini-2.5-flash</option>
            <option value="gemini-2.5-pro">gemini-2.5-pro</option>
            <option value="gemini-1.5-flash">gemini-1.5-flash</option>
            <option value="gemini-1.5-pro">gemini-1.5-pro</option>
          </select>
        </div>

        <div class="setting-row">
          <label class="setting-label">扫描间隔</label>
          <select v-model="form.scan_interval_minutes" class="setting-select" :disabled="!form.enabled">
            <option :value="15">15分钟</option>
            <option :value="30">30分钟</option>
            <option :value="60">1小时</option>
            <option :value="120">2小时</option>
            <option :value="360">6小时</option>
          </select>
        </div>
      </div>

      <div class="action-buttons" v-if="form.enabled">
        <button class="btn secondary" @click="handleTestConnection" :disabled="testing || !canTest">
          <span v-if="testing" class="spinner"></span>
          测试连接
        </button>
        <button class="btn primary" @click="handleScanNow" :disabled="scanStatus === 'scanning' || !config.has_auth_code">
          <span v-if="scanStatus === 'scanning'" class="spinner"></span>
          增量扫描
        </button>
        <button class="btn warning" @click="handleResetAndFullScan" :disabled="scanStatus === 'scanning' || !config.has_auth_code" title="清除历史扫描记录，全量重新扫描近30天所有邮件">
          <span v-if="scanStatus === 'scanning'" class="spinner"></span>
          全量重新扫描
        </button>
      </div>

      <div class="engine-tip-card">
        <div class="tip-badge">双引擎保障</div>
        <div class="tip-text">
          <b>本地离线规则引擎</b>：自动解析携程、长鑫、中兴、百度、牛客等笔试/面试/测评（无需外网，毫秒响应）。<br>
          <b>Gemini 大模型</b>：针对复杂格式邮件智能提炼兜底。<br>
          <span style="opacity: 0.85;">提示：若之前扫描遗漏了邮件，请点击“全量重新扫描”。</span>
        </div>
      </div>

      <div v-if="statusMessage" class="status-message" :class="statusType">
        {{ statusMessage }}
      </div>

      <!-- Extracted Items Preview -->
      <div v-if="scanResult && scanResult.created_items && scanResult.created_items.length > 0" class="extracted-items-box">
        <div class="extracted-title">
          <span>本次提取到的事项 ({{ scanResult.created_items.length }})</span>
        </div>
        <div class="extracted-list">
          <div v-for="(item, idx) in scanResult.created_items" :key="idx" class="extracted-item">
            <div class="extracted-top">
              <span class="extracted-date">{{ item.item_date }}</span>
              <span class="extracted-subject" :title="item.email_subject">{{ item.email_subject }}</span>
            </div>
            <div class="extracted-body">{{ item.item_text }}</div>
          </div>
        </div>
      </div>

      <div v-if="config.last_scan_time" class="stats-info">
        上次扫描: {{ formatDate(config.last_scan_time) }}<br>
        已处理邮件数: {{ config.processed_count }}
      </div>

    </div>

    <div class="settings-footer">
      <button class="btn primary full-width" @click="handleSave" :disabled="saving">
        <span v-if="saving" class="spinner"></span>
        保存设置
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue'
import { useEmailScan } from '../composables/useEmailScan'

const emit = defineEmits<{
  (e: 'close'): void
}>()

const { config, loadConfig, saveConfig, testConnection, scanNow, scanStatus, lastMessage, setEnabled, resetHistory, scanResult } = useEmailScan()

const form = ref({
  enabled: false,
  email_address: '',
  auth_code: '',
  gemini_api_key: '',
  gemini_model: 'gemini-3.8-flash',
  scan_interval_minutes: 60
})

const showAuthCode = ref(false)
const showApiKey = ref(false)
const testing = ref(false)
const saving = ref(false)
const statusMessage = ref('')
const statusType = ref<'success'|'error'|'info'>('info')

const canTest = computed(() => {
  return form.value.email_address && (form.value.auth_code || config.value.has_auth_code)
})

onMounted(async () => {
  await loadConfig()

  // Initialize form with current config
  form.value = {
    enabled: config.value.enabled,
    email_address: config.value.email_address,
    auth_code: '', // Don't pre-fill password fields
    gemini_api_key: '',
    gemini_model: config.value.gemini_model || 'gemini-2.5-flash',
    scan_interval_minutes: config.value.scan_interval_minutes || 60
  }
})

// Watch global scan status to update local messages if a background scan finishes
watch([scanStatus, lastMessage], ([newStatus, message]) => {
  if (newStatus !== 'idle' && message) {
    statusType.value = newStatus === 'scanning' ? 'info' : newStatus
    statusMessage.value = message
  }
}, { immediate: true })

const onEnabledChange = async () => {
  if (form.value.enabled !== config.value.enabled) {
    // We update the backend immediately for the toggle
    await setEnabled(form.value.enabled)
    if (form.value.enabled) {
      statusType.value = 'success'
      statusMessage.value = '邮件扫描已启用'
    } else {
      statusType.value = 'info'
      statusMessage.value = '邮件扫描已禁用'
    }

    setTimeout(() => {
      statusMessage.value = ''
    }, 3000)
  }
}

const handleTestConnection = async () => {
  testing.value = true
  statusMessage.value = '正在测试连接...'
  statusType.value = 'info'

  const authCodeToUse = form.value.auth_code && form.value.auth_code.trim() ? form.value.auth_code.trim() : undefined

  // If no auth code provided in input and none saved in config, error out
  if (!authCodeToUse && !config.value.has_auth_code) {
    statusMessage.value = '请提供授权码'
    statusType.value = 'error'
    testing.value = false
    return
  }

  const result = await testConnection(form.value.email_address, authCodeToUse)

  if (result.success) {
    statusType.value = 'success'
    statusMessage.value = '连接成功: ' + result.message
  } else {
    statusType.value = 'error'
    statusMessage.value = '连接失败: ' + result.error
  }

  testing.value = false
}

const handleScanNow = async () => {
  statusMessage.value = '正在启动增量扫描...'
  statusType.value = 'info'

  const result = await scanNow()

  if (result) {
    if (result.success) {
      statusType.value = 'success'
      statusMessage.value = `扫描完成。处理了 ${result.emails_scanned} 封邮件，创建了 ${result.items_created} 个待办事项。`
    } else {
      statusType.value = 'error'
      statusMessage.value = `扫描失败: ${result.errors.join(', ')}`
    }
  } else {
    statusType.value = 'error'
    statusMessage.value = '扫描发生错误'
  }
}

const handleResetAndFullScan = async () => {
  statusMessage.value = '正在重置历史记录并启动全量重新扫描...'
  statusType.value = 'info'

  await resetHistory()
  const result = await scanNow()

  if (result) {
    if (result.success) {
      statusType.value = 'success'
      statusMessage.value = `全量扫描完成！分析了 ${result.emails_scanned} 封邮件，提取并添加了 ${result.items_created} 个待办事项。`
    } else {
      statusType.value = 'error'
      statusMessage.value = `全量扫描完成，但部分失败: ${result.errors.join(', ')}`
    }
  } else {
    statusType.value = 'error'
    statusMessage.value = '全量扫描发生错误'
  }
}

const handleSave = async () => {
  saving.value = true
  statusMessage.value = '正在保存设置...'
  statusType.value = 'info'

  const result = await saveConfig(
    form.value.email_address,
    form.value.auth_code ? form.value.auth_code : undefined,
    form.value.gemini_api_key ? form.value.gemini_api_key : undefined,
    form.value.scan_interval_minutes,
    form.value.enabled,
    form.value.gemini_model
  )

  if (result.success) {
    statusType.value = 'success'
    statusMessage.value = '设置已保存'
    form.value.auth_code = '' // Clear on success
    form.value.gemini_api_key = '' // Clear on success

    setTimeout(() => {
      statusMessage.value = ''
      emit('close')
    }, 1500)
  } else {
    statusType.value = 'error'
    statusMessage.value = '保存失败: ' + result.error
  }

  saving.value = false
}

const formatDate = (dateStr: string) => {
  if (!dateStr) return '从未扫描'
  try {
    const date = new Date(dateStr)
    return date.toLocaleString('zh-CN', {
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit'
    })
  } catch (e) {
    return dateStr
  }
}
</script>

<style scoped>
.email-settings-overlay {
  position: absolute;
  top: 36px; /* Below TitleBar */
  left: 0;
  right: 0;
  bottom: 0;
  background-color: var(--bg);
  z-index: 100;
  display: flex;
  flex-direction: column;
}

.settings-header {
  display: flex;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
  background-color: var(--surface);
}

.back-btn {
  background: none;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  padding: 4px;
  margin-right: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius);
  transition: all 0.2s;
}

.back-btn:hover {
  background-color: var(--surface-2);
  color: var(--text-primary);
}

.icon {
  width: 18px;
  height: 18px;
}

h2 {
  font-size: 16px;
  font-weight: 500;
  margin: 0;
  color: var(--text-primary);
}

.settings-content {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}

.setting-group {
  background-color: var(--surface);
  border-radius: var(--radius);
  border: 1px solid var(--border);
  margin-bottom: 16px;
  padding: 12px;
}

.setting-group.disabled {
  opacity: 0.6;
  pointer-events: none;
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.setting-row:last-child {
  margin-bottom: 0;
}

.setting-row.vertical {
  flex-direction: column;
  align-items: flex-start;
}

.setting-row.vertical .setting-label {
  margin-bottom: 6px;
}

.setting-label {
  font-size: 14px;
  color: var(--text-primary);
  font-weight: 500;
}

.setting-input, .setting-select {
  width: 100%;
  padding: 8px 10px;
  background-color: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text-primary);
  font-size: 14px;
  outline: none;
  transition: border-color 0.2s;
}

.setting-input:focus, .setting-select:focus {
  border-color: var(--accent);
}

.setting-select {
  width: auto;
  min-width: 120px;
}

.password-input {
  position: relative;
  width: 100%;
  display: flex;
  align-items: center;
}

.password-input .setting-input {
  padding-right: 36px;
}

.toggle-visibility {
  position: absolute;
  right: 8px;
  background: none;
  border: none;
  color: var(--text-tertiary);
  cursor: pointer;
  padding: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.toggle-visibility:hover {
  color: var(--text-secondary);
}

.hint-text {
  font-size: 12px;
  color: var(--text-tertiary);
  margin-top: 4px;
}

/* Switch Styles */
.switch {
  position: relative;
  display: inline-block;
  width: 40px;
  height: 20px;
}

.switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

.slider {
  position: absolute;
  cursor: pointer;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: var(--surface-2);
  transition: .4s;
  border-radius: 20px;
  border: 1px solid var(--border);
}

.slider:before {
  position: absolute;
  content: "";
  height: 14px;
  width: 14px;
  left: 2px;
  bottom: 2px;
  background-color: var(--text-secondary);
  transition: .4s;
  border-radius: 50%;
}

input:checked + .slider {
  background-color: var(--accent);
  border-color: var(--accent);
}

input:checked + .slider:before {
  background-color: white;
  transform: translateX(20px);
}

.action-buttons {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 14px;
}

.btn {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 7px 12px;
  border-radius: var(--radius);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
  border: 1px solid transparent;
}

.btn.primary {
  background-color: var(--accent);
  color: var(--on-accent);
}

.btn.primary:hover:not(:disabled) {
  filter: brightness(1.1);
}

.btn.secondary {
  background-color: var(--surface);
  color: var(--text-primary);
  border-color: var(--border);
}

.btn.secondary:hover:not(:disabled) {
  background-color: var(--surface-2);
}

.btn.warning {
  background-color: rgba(245, 158, 11, 0.1);
  color: #f59e0b;
  border-color: rgba(245, 158, 11, 0.3);
}

.btn.warning:hover:not(:disabled) {
  background-color: rgba(245, 158, 11, 0.2);
  border-color: #f59e0b;
}

.btn.warning .spinner {
  border-color: rgba(245, 158, 11, 0.3);
  border-top-color: #f59e0b;
}

.engine-tip-card {
  background-color: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 10px 12px;
  margin-bottom: 14px;
  font-size: 12px;
  line-height: 1.5;
}

.tip-badge {
  display: inline-block;
  font-size: 11px;
  font-weight: 600;
  color: var(--accent);
  background-color: var(--accent-glow);
  padding: 2px 6px;
  border-radius: 4px;
  margin-bottom: 6px;
}

.tip-text {
  color: var(--text-secondary);
}

.tip-text b {
  color: var(--text-primary);
}

.extracted-items-box {
  background-color: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 12px;
  margin-bottom: 14px;
}

.extracted-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 8px;
}

.extracted-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 180px;
  overflow-y: auto;
}

.extracted-item {
  background-color: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px 10px;
}

.extracted-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
  gap: 8px;
}

.extracted-date {
  font-size: 12px;
  font-weight: 600;
  color: var(--accent);
  flex-shrink: 0;
}

.extracted-subject {
  font-size: 11px;
  color: var(--text-tertiary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.extracted-body {
  font-size: 12px;
  color: var(--text-primary);
  word-break: break-all;
  line-height: 1.4;
}

.btn.full-width {
  width: 100%;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.status-message {
  padding: 10px;
  border-radius: var(--radius);
  font-size: 13px;
  margin-bottom: 16px;
  word-break: break-word;
}

.status-message.success {
  background-color: rgba(34, 197, 94, 0.1);
  color: #22c55e;
  border: 1px solid rgba(34, 197, 94, 0.2);
}

.status-message.error {
  background-color: var(--danger-bg);
  color: var(--danger);
  border: 1px solid rgba(239, 68, 68, 0.2);
}

.status-message.info {
  background-color: var(--surface-2);
  color: var(--text-secondary);
  border: 1px solid var(--border);
}

.stats-info {
  font-size: 12px;
  color: var(--text-tertiary);
  line-height: 1.5;
  margin-top: auto;
}

.settings-footer {
  padding: 16px;
  border-top: 1px solid var(--border);
  background-color: var(--surface);
}

.spinner {
  display: inline-block;
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-radius: 50%;
  border-top-color: currentColor;
  animation: spin 1s ease-in-out infinite;
  margin-right: 8px;
}

.btn.secondary .spinner {
  border-color: rgba(0, 0, 0, 0.1);
  border-top-color: currentColor;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
