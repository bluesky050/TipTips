export interface PlannerItem {
  id: string
  text: string
  done: boolean
  created_date: string
  carried_over: boolean
  sort_order: number
  source: string      // 'manual' | 'email' | 'seu' (read-only official schedule)
  email_uid?: number
  schedule?: Schedule | null
}

export interface Schedule {
  event_type: string
  company: string
  company_raw: string
  role: string
  round: string
  date: string
  time: string
  end_date: string
  end_time: string
  time_kind: 'start' | 'deadline' | 'window' | 'unknown'
  timezone: string
  location: string
  link: string
  notes: string
  source_subject: string
  source_date: string
  time_evidence: string
  needs_confirmation: boolean
}

export interface EmailConfig {
  enabled: boolean
  email_address: string
  has_auth_code: boolean
  has_gemini_api_key: boolean
  scan_interval_minutes: number
  last_scan_time: string
  processed_count: number
  gemini_model: string
}

export interface ScanResult {
  success: boolean
  emails_scanned: number
  items_created: number
  errors: string[]
  created_items: { email_subject: string; item_text: string; item_date: string }[]
  scan_time: string
}
