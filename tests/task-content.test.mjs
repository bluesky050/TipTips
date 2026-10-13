import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { test } from 'node:test'
import { parse, compileScript } from '@vue/compiler-sfc'
import ts from 'typescript'
import { createSSRApp } from 'vue'
import { renderToString } from '@vue/server-renderer'

// Compile the actual Vue SFC in memory, including its template. No duplicate renderer.
async function componentModule(file, replacements = {}) {
  const source = await readFile(new URL(file, import.meta.url), 'utf8')
  const { descriptor } = parse(source)
  const compiled = compileScript(descriptor, { id: 'component-test', inlineTemplate: true })
  let moduleSource = ts.transpileModule(compiled.content, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
  }).outputText
  const imports = { vue: import.meta.resolve('vue'), '@tauri-apps/plugin-opener': import.meta.resolve('@tauri-apps/plugin-opener'), ...replacements }
  for (const [dependency, url] of Object.entries(imports)) {
    moduleSource = moduleSource.replaceAll(`from "${dependency}"`, `from "${url}"`)
      .replaceAll(`from '${dependency}'`, `from '${url}'`)
  }
  return `data:text/javascript;base64,${Buffer.from(moduleSource).toString('base64')}`
}
async function typescriptModule(file, replacements) {
  const source = await readFile(new URL(file, import.meta.url), 'utf8')
  let moduleSource = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext } }).outputText
  for (const [dependency, url] of Object.entries({ vue: import.meta.resolve('vue'), ...replacements })) {
    moduleSource = moduleSource.replaceAll(`from "${dependency}"`, `from "${url}"`).replaceAll(`from '${dependency}'`, `from '${url}'`)
  }
  return `data:text/javascript;base64,${Buffer.from(moduleSource).toString('base64')}`
}
const taskContentModule = await componentModule('../src/components/TaskContent.vue')
const { default: TaskContent } = await import(taskContentModule)
const { default: PlannerItem } = await import(await componentModule('../src/components/PlannerItem.vue', { './TaskContent.vue': taskContentModule }))
const schedule = {
  event_type: '面试', company: '示例公司', company_raw: '示例公司', role: '后端开发', round: '二面',
  date: '2026-10-15', time: '14:00', end_date: '', end_time: '', time_kind: 'start', timezone: 'UTC+08:00',
  location: '腾讯会议 123456', link: 'https://meeting.example.com/join', notes: '提前10分钟进入',
  source_subject: '示例公司面试邀请', source_date: 'Sat, 10 Oct 2026 10:00:00 +0800',
  time_evidence: '面试时间：2026-10-15 14:00', needs_confirmation: false,
}
const item = { id: 'test', text: '[面试] 示例公司 · 后端开发 · 二面', done: false,
  created_date: '2026-10-15', carried_over: false, sort_order: 0, source: 'email', schedule }
const render = (value) => renderToString(createSSRApp(TaskContent, { item: value }))

test('partial scan failure still refreshes created tasks for manual and background scans', async () => {
  const result = { success: false, emails_scanned: 2, items_created: 1, errors: ['合成解析失败'], created_items: [], scan_time: '2026-10-13 10:00:00' }
  const invokeStub = `data:text/javascript;base64,${Buffer.from(`export async function invoke(command) { return command === 'scan_emails_now' ? ${JSON.stringify(result)} : {} }`).toString('base64')}`
  const eventStub = `data:text/javascript;base64,${Buffer.from('export const handlers = {}; export async function listen(name, fn) { handlers[name] = fn; return () => {} }').toString('base64')}`
  const syncModule = await typescriptModule('../src/composables/useDataSync.ts', {})
  const { useDataSync } = await import(syncModule)
  const { dataVersion } = useDataSync()
  const emailScanModule = await typescriptModule('../src/composables/useEmailScan.ts', { '@tauri-apps/api/core': invokeStub, '@tauri-apps/api/event': eventStub, './useDataSync': syncModule })
  const { useEmailScan } = await import(emailScanModule)
  const scan = useEmailScan()
  const settings = useEmailScan()
  const before = dataVersion.value
  await scan.scanNow()
  assert.equal(dataVersion.value, before + 1)
  assert.equal(scan.scanStatus.value, 'error')
  const { default: EmailSettings } = await import(await componentModule('../src/components/EmailSettings.vue', { '../composables/useEmailScan': emailScanModule }))
  const settingsHtml = await renderToString(createSSRApp(EmailSettings))
  assert.ok(settingsHtml.includes('合成解析失败'))
  assert.ok(settingsHtml.includes('本次提取到的事项') === false)
  assert.equal(settings.scanStatus.value, 'error')
  assert.deepEqual(settings.scanResult.value, result)
  assert.equal(settings.lastMessage.value, scan.lastMessage.value)
  let refreshed = 0
  await scan.setupListeners(() => refreshed++)
  const { handlers } = await import(eventStub)
  handlers['email-scan-complete']({ payload: result })
  assert.equal(refreshed, 1)
  assert.equal(dataVersion.value, before + 2)
  assert.equal(scan.scanStatus.value, 'error')
  scan.cleanupListeners()
})

test('long notes and legacy titles are collapsed with complete text available', async () => {
  const notes = '很长的准备说明'.repeat(60)
  const html = await render({ ...item, schedule: { ...schedule, notes } })
  assert.match(html, /<details[^>]*>[\s\S]*完整备注[\s\S]*很长的准备说明/)
  const legacy = await render({ ...item, schedule: undefined, text: '旧邮件说明'.repeat(40) })
  assert.ok(legacy.includes('完整文本'))
})

test('official schedules identify the website and never label it as email evidence', async () => {
  const html = await render({ ...item, source: 'seu', schedule: { ...schedule, event_type: '宣讲会', time_kind: 'window', end_date: '2026-10-15', end_time: '16:00' } })
  assert.ok(html.includes('东南大学就业网'))
  assert.ok(html.includes('官网信息'))
  assert.ok(html.includes('宣讲时间'))
  assert.ok(!html.includes('邮件依据'))
})

test('official list card is read-only while personal tasks retain actions', async () => {
  const official = await renderToString(createSSRApp(PlannerItem, { item: { ...item, source: 'seu' }, canMoveUp: true }))
  assert.ok(official.includes('官网日程'))
  assert.ok(!official.includes('class="checkbox'))
  assert.ok(!official.includes('title="编辑"'))
  assert.ok(!official.includes('title="删除"'))
  assert.ok(!official.includes('title="上移"'))
  const personal = await renderToString(createSSRApp(PlannerItem, { item }))
  assert.ok(personal.includes('class="checkbox'))
  assert.ok(personal.includes('title="编辑"'))
})

test('calendar counts official talks separately from personal completion', async () => {
  const fixtures = [
    { ...item, id: 'seu:one', source: 'seu' },
    { ...item, id: 'personal', source: 'manual', done: true },
    { ...item, id: 'seu:only', source: 'seu', created_date: '2026-10-16' },
  ]
  const invokeStub = `data:text/javascript;base64,${Buffer.from(`export async function invoke() { return ${JSON.stringify(fixtures)} }`).toString('base64')}`
  const syncModule = await typescriptModule('../src/composables/useDataSync.ts', {})
  const calendarModule = await typescriptModule('../src/composables/useCalendar.ts', { '@tauri-apps/api/core': invokeStub, './useDataSync': syncModule })
  const { useCalendar } = await import(calendarModule)
  const calendar = useCalendar()
  calendar.currentYear.value = 2026
  calendar.currentMonth.value = 9
  await calendar.loadMonthData()
  assert.deepEqual(calendar.statsByDate.value['2026-10-15'], { total: 1, done: 1, fairs: 1 })
  assert.deepEqual(calendar.statsByDate.value['2026-10-16'], { total: 0, done: 0, fairs: 1 })
  const { default: CalendarView } = await import(await componentModule('../src/components/CalendarView.vue', { '../composables/useCalendar': calendarModule, './TaskContent.vue': taskContentModule }))
  const html = await renderToString(createSSRApp(CalendarView))
  assert.ok(html.includes('宣讲 1'))
  assert.ok(html.includes('1/1'))
  assert.ok(!html.includes('0/0'))
  const { useDataSync } = await import(syncModule)
  const { notifyDataChange } = useDataSync()
  const originalDate = globalThis.Date
  let simulated = '2026-10-11T12:00:00+08:00'
  globalThis.Date = class extends originalDate {
    constructor(...args) { super(...(args.length ? args : [simulated])) }
  }
  try {
    notifyDataChange()
    assert.equal(calendar.todayStr.value, '2026-10-11')
    simulated = '2026-10-12T12:00:00+08:00'
    notifyDataChange()
    assert.equal(calendar.todayStr.value, '2026-10-12')
  } finally { globalThis.Date = originalDate }
})

test('Beijing midnight invalidates both views without another website request', async () => {
  const syncModule = await typescriptModule('../src/composables/useDataSync.ts', {})
  const invokeStub = `data:text/javascript;base64,${Buffer.from('export async function invoke(command) { if(command !== "get_seu_fair_status") throw new Error("Unexpected network refresh"); return {updated_at:"",count:0} }').toString('base64')}`
  const fairModule = await typescriptModule('../src/composables/useSeuFairs.ts', { './useDataSync': syncModule, '@tauri-apps/api/core': invokeStub })
  const { beijingDate, useSeuFairs } = await import(fairModule)
  assert.equal(beijingDate(new Date('2026-10-10T15:59:59Z')), '2026-10-10')
  assert.equal(beijingDate(new Date('2026-10-10T16:00:00Z')), '2026-10-11')
  const { useDataSync } = await import(syncModule)
  const { dataVersion } = useDataSync()
  const fairs = useSeuFairs()
  fairs.checkDayBoundary(new Date('2099-12-31T12:00:00+08:00'))
  const version = dataVersion.value
  assert.equal(fairs.checkDayBoundary(new Date('2099-12-31T20:00:00+08:00')), false)
  assert.equal(dataVersion.value, version)
  assert.equal(fairs.checkDayBoundary(new Date('2100-01-01T00:00:00+08:00')), true)
  assert.equal(dataVersion.value, version + 1)
  await Promise.resolve()
})

test('structured item displays title, start time, location, notes and source separately', async () => {
  const html = await render(item)
  for (const text of [item.text, '2026-10-15 14:00 · 开始', '地点：腾讯会议 123456', '备注：提前10分钟进入', '打开活动链接', '邮件依据', schedule.source_subject]) {
    assert.ok(html.includes(text), text)
  }
  assert.ok(!html.includes('>待确认<'))
})

test('deadline and window are labeled with different meanings', async () => {
  const deadline = await render({ ...item, schedule: { ...schedule, time_kind: 'deadline' } })
  assert.ok(deadline.includes('2026-10-15 14:00 · 截止'))
  const window = await render({ ...item, schedule: { ...schedule, time_kind: 'window', end_date: '2026-10-16', end_time: '18:00' } })
  assert.ok(window.includes('2026-10-15 14:00 至 2026-10-16 18:00 · 可参加时间段'))
})

test('undated item is visibly awaiting confirmation', async () => {
  const html = await render({ ...item, schedule: { ...schedule, date: '', time: '', time_kind: 'unknown', needs_confirmation: true } })
  assert.ok(html.includes('时间待确认'))
  assert.ok(html.includes('>待确认<'))
})

test('legacy and manual text are preserved without fabricated schedule details', async () => {
  const html = await render({ ...item, schedule: undefined, text: '我自己输入的待办', source: 'manual' })
  assert.ok(html.includes('我自己输入的待办'))
  assert.ok(!html.includes('邮件依据'))
  assert.ok(!html.includes('时间待确认'))
})

test('manual title edits override generated title while retaining details', async () => {
  const html = await render({ ...item, text: '我调整后的标题', done: true })
  assert.ok(html.includes('我调整后的标题'))
  assert.ok(!html.includes('[面试] 示例公司'))
  assert.ok(html.includes('2026-10-15 14:00 · 开始'))
  assert.ok(html.includes('completed'))
})

test('email text is escaped and unsafe links have no open action', async () => {
  const html = await render({ ...item, text: '<script>bad()</script>', schedule: { ...schedule, link: 'javascript:bad()' } })
  assert.ok(html.includes('&lt;script&gt;bad()&lt;/script&gt;'))
  assert.ok(!html.includes('打开活动链接'))
})
