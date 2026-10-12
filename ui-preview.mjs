// Local visual QA: render real Vue components with synthetic data, without Tauri or email access.
import { createRequire } from 'node:module'
import { readFile, writeFile } from 'node:fs/promises'
import { pathToFileURL } from 'node:url'
import { createServer } from 'node:http'
const require = createRequire(import.meta.url)
const { parse, compileScript, compileStyle } = require('@vue/compiler-sfc')
const ts = require('typescript')
const { createSSRApp } = require('vue')
const { renderToString } = require('@vue/server-renderer')
let css = ''
const moduleUrl = js => `data:text/javascript;base64,${Buffer.from(js).toString('base64')}`
async function component(file, id, replacements = {}) {
  const { descriptor } = parse(await readFile(file, 'utf8'))
  for (const style of descriptor.styles) css += compileStyle({ source: style.content, id: `data-v-${id}`, scoped: style.scoped }).code + '\n'
  let js = ts.transpileModule(compileScript(descriptor, { id, inlineTemplate: true }).content,
    { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext } }).outputText
  const imports = { vue: pathToFileURL(require.resolve('vue')).href,
    '@tauri-apps/plugin-opener': moduleUrl('export async function openUrl() {}'), ...replacements }
  for (const [key, value] of Object.entries(imports)) js = js.replaceAll(`from "${key}"`, `from "${value}"`).replaceAll(`from '${key}'`, `from '${value}'`)
  const url = moduleUrl(js)
  const result = (await import(url)).default
  result.__scopeId = `data-v-${id}`
  return { component: result, url }
}
const content = await component('src/components/TaskContent.vue', 'qa-task')
const planner = await component('src/components/PlannerItem.vue', 'qa-item', { './TaskContent.vue': content.url })
const date = '2026-10-12'
const schedule = { event_type: '面试', date, time: '14:00', time_kind: 'start', timezone: 'UTC+08:00', location: '线上会议', notes: '提前10分钟进入，准备简历与项目介绍。', link: '', source_subject: '合成界面检查', source_date: date, needs_confirmation: false }
const items = [
  { id: 'task', text: '[面试] 示例公司 · 后端开发 · 二面', done: false, source: 'manual', schedule },
  { id: 'seu:one', text: '[宣讲会] 示例科技公司校园招聘宣讲会', done: false, source: 'seu', schedule: { ...schedule, event_type: '宣讲会', time: '16:00', end_date: date, end_time: '18:00', time_kind: 'window', location: '九龙湖校区 大学生活动中心', notes: '' } },
  { id: 'done', text: '已完成：准备项目介绍与简历', done: true, source: 'manual' },
  { id: 'long', text: '[测评] 示例公司 · ' + '软件工程与开发能力综合测评报名说明'.repeat(5), done: false, source: 'manual', schedule },
]
const cards = (await Promise.all(items.map(item => renderToString(createSSRApp(planner.component, { item }))))).join('')
const days = [...Array(3).fill({ date: '', day: 0 }), ...Array.from({ length: 31 }, (_, i) => ({ date: `2026-10-${String(i + 1).padStart(2, '0')}`, day: i + 1 })), { date: '', day: 0 }]
const calendarStub = moduleUrl(`import { ref } from '${pathToFileURL(require.resolve('vue')).href}'; export function useCalendar() { const noop=()=>{}; return { monthLabel:ref('2026年10月'), todayStr:ref('${date}'), calendarDays:ref(${JSON.stringify(days)}), statsByDate:ref({'${date}':{total:2,done:1,fairs:12}}), selectedDate:ref(null), selectedDateItems:ref([]), loadMonthData:noop, prevMonth:noop, nextMonth:noop, selectDate:noop, addItemToDate:noop, toggleItem:noop, deleteItem:noop, updateItem:noop }; }`)
const calendar = await component('src/components/CalendarView.vue', 'qa-calendar', { './TaskContent.vue': content.url, '../composables/useCalendar': calendarStub })
const grid = await renderToString(createSSRApp(calendar.component))
const app = parse(await readFile('src/App.vue', 'utf8')).descriptor
const globalCss = app.styles.map(style => style.content).join('\n')
const themeBlocks = [...globalCss.matchAll(/:root\s*\{([^}]+)\}/g)].map(match => match[1])
const html = `<!doctype html><html lang="zh-CN"><meta charset="utf-8"><title>TipTips · 合成组件预览</title><style>${globalCss}\n${css}
body{height:auto;overflow:auto;background:#e8eaf0;padding:20px}.qa-grid{display:flex;align-items:flex-start;gap:20px}.qa-panel{width:224px;flex-shrink:0}.qa-panel h2{font-size:14px;margin-bottom:10px;color:#321c26}.qa-panel .app{height:850px}.qa-panel.light{${themeBlocks[0]}}.qa-panel.dark{${themeBlocks[0]}${themeBlocks[1]}}.qa-calendar .app{height:500px}.qa-calendar .calendar-view{padding-top:12px}</style><div class="qa-grid">
<section class="qa-panel light"><h2>浅色 · 窄窗口 · 合成数据</h2><div class="app"><div class="item-list">${cards}</div></div></section>
<section class="qa-panel dark"><h2>深色 · 窄窗口 · 合成数据</h2><div class="app"><div class="item-list">${cards}</div></div></section>
<section class="qa-panel light qa-calendar"><h2>日历 · 合成宣讲数量12</h2><div class="app">${grid}</div></section></div></html>`
await writeFile('docs/ui-polish-preview.html', html)
if (process.argv.includes('--serve')) {
  const server = createServer((request, response) => { response.setHeader('Content-Type', 'text/html; charset=utf-8'); response.end(html) })
  server.listen(0, '127.0.0.1', () => console.log(`Preview: http://127.0.0.1:${server.address().port}/`))
}
