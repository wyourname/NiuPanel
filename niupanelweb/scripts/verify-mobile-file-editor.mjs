import assert from 'node:assert/strict'
import { randomUUID } from 'node:crypto'
import { mkdir, rm, writeFile } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright'

// Run against the Docker Vite server. All API calls are intercepted; no user files are touched.
const origin = process.env.FILE_UI_URL || 'http://127.0.0.1:7787'
const name = `file-ui-check-${randomUUID()}.html`
const harness = fileURLToPath(new URL(`../${name}`, import.meta.url))
const artifacts = process.env.FILE_UI_ARTIFACTS || '/tmp/niupanel-file-ui'
const html = `<!doctype html><html><head><meta name="viewport" content="width=device-width,initial-scale=1"></head>
<body><div id="app"></div><script type="module">
import {createApp, h, ref, proxyRefs} from 'vue';
import {createPinia} from 'pinia';
import {createRouter, createMemoryHistory, RouterView} from 'vue-router';
import ElementPlus from 'element-plus';
import 'element-plus/dist/index.css';
import 'element-plus/theme-chalk/dark/css-vars.css';
import '/src/assets/styles/index.css';
import 'virtual:uno.css';
import i18n from '/src/locales/index.ts';
import FileView from '/src/views/modules/File.vue';
import {useAppStore} from '/src/stores/app.ts';
import {useFileEditorSession} from '/src/composables/file/useFileEditorSession.ts';
import {useFileListState} from '/src/composables/file/useFileListState.ts';
import {useFileClipboard} from '/src/composables/file/useFileClipboard.ts';
const Logic = {setup() {
  const list = useFileListState(ref(null));
  const session = useFileEditorSession();
  const clipboard = useFileClipboard({currentPath: list.currentPath, loadContents: list.loadContents, clearSelection: list.clearSelection});
  window.logic = proxyRefs({...list, ...session, ...clipboard});
  return () => h('p', 'Logic checks');
}};
const router = createRouter({history:createMemoryHistory(), routes:[
  {path:'/',component:FileView}, {path:'/logic',component:Logic}, {path:'/away',component:{render:()=>h('p','Away')}}
]});
const pinia=createPinia();
createApp({render:()=>h(RouterView)}).use(pinia).use(router).use(i18n).use(ElementPlus).mount('#app');
window.appStore=useAppStore(); window.router=router;
</script><style>html,body,#app{margin:0;height:100%;width:100%;overflow:hidden}</style></body></html>`

const item = (path, is_dir = false) => ({ name: path.split('/').at(-1), path, is_dir, size: 123, mtime: 1780000000 })
const unusualName = '配置 #100%?.js'
const content = 'const 中文 = 1;\r\nconsole.log(中文);\r\n// 最后一行\r\n'
const documents = new Map([[unusualName, content], ['broken.py', 'print("重试成功")\n']])
const rootItems = [item('scripts', true), item('target', true), item(unusualName), item('broken.py')]
const requests = [], writes = [], copies = []
const deletedPaths = new Set()
let failDelete = ''
let failRead = true, failSave = false, saveDelay = 0, searchDelay = 0, copyDelay = 0, failCopy = ''
let browser, page
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms))
const visible = locator => locator.waitFor({ state: 'visible' })
const absent = locator => locator.waitFor({ state: 'hidden' })
const pass = message => console.log(`PASS: ${message}`)

try {
  await writeFile(harness, html)
  await mkdir(artifacts, { recursive: true })
  browser = await chromium.launch({ headless: true, ...(process.env.CHROMIUM_PATH ? { executablePath: process.env.CHROMIUM_PATH } : {}) })
  const context = await browser.newContext({ viewport: { width: 375, height: 812 }, isMobile: true, hasTouch: true })
  page = await context.newPage()
  page.setDefaultTimeout(15000)
  const errors = []
  page.on('pageerror', error => errors.push(error.message))
  await page.route('**/api/v1/**', async route => {
    const request = route.request(), url = new URL(request.url()), path = decodeURIComponent(url.pathname)
    requests.push({ url: request.url(), method: request.method() })
    const respond = (data, status = 200) => route.fulfill({ status, json: { code: status, data, message: status === 200 ? 'ok' : '模拟请求失败' } })
    if (request.method() === 'GET' && path.includes('/files/scripts')) {
      const directory = path.split('/files/scripts')[1].replace(/^\//, '')
      if (url.searchParams.has('q')) {
        await sleep(searchDelay)
        return respond([item(`${directory}/old-search.js`)])
      }
      return respond((directory ? [item(`${directory}/nested.js`)] : rootItems).filter(file => !deletedPaths.has(file.path)))
    }
    if (request.method() === 'GET' && path.includes('/files/file/')) {
      const file = path.split('/files/file/')[1]
      if (file === 'slow.js') await sleep(350)
      return respond(documents.get(file) || file, file === 'broken.py' && failRead ? 500 : 200)
    }
    if (request.method() === 'PUT' && path.endsWith('/files/file')) {
      const payload = request.postDataJSON(); writes.push(payload)
      await sleep(saveDelay)
      if (!failSave) documents.set(payload.path, payload.content)
      return respond(null, failSave ? 500 : 200)
    }
    if (request.method() === 'DELETE' && path.includes('/files/scripts/')) {
      const target = path.split('/files/scripts/')[1]
      if (target === failDelete) return respond(null, 500)
      deletedPaths.add(target)
      return respond(null)
    }
    if (path.endsWith('/files/copy') || path.endsWith('/files/rename')) {
      const payload = request.postDataJSON(); copies.push(payload)
      await sleep(copyDelay)
      return respond(null, payload.from_path === failCopy ? 500 : 200)
    }
    errors.push(`Unexpected API call: ${request.method()} ${url}`)
    return route.abort()
  })
  await page.goto(`${origin}/${name}`)
  await visible(page.getByRole('button', { name: `打开文件 ${unusualName}`, exact: true }))
  await page.screenshot({ path: `${artifacts}/mobile-files.png` })

  // Reading errors cannot turn into accidental empty-file saves.
  await page.getByRole('button', { name: '打开文件 broken.py', exact: true }).click()
  await visible(page.getByText('文件读取失败，请重试。', { exact: true }))
  assert.equal(await page.getByRole('button', { name: '保存文件', exact: true }).isDisabled(), true)
  assert.equal(writes.length, 0)
  failRead = false
  await page.getByRole('button', { name: '重新读取', exact: true }).click()
  await visible(page.getByRole('textbox', { name: '文件内容', exact: true }))
  assert.match(await page.locator('.cm-content').innerText(), /重试成功/)
  await page.getByRole('button', { name: '关闭文件编辑器', exact: true }).click()
  await absent(page.locator('.file-editor-overlay'))
  pass('读取失败禁止保存，重试可恢复编辑')

  await page.getByRole('button', { name: `打开文件 ${unusualName}`, exact: true }).click()
  const editor = page.getByRole('textbox', { name: '文件内容', exact: true })
  await visible(editor)
  assert.ok(requests.some(({ url }) => url.endsWith(encodeURIComponent(unusualName))))
  assert.match(await page.locator('.mobile-editor-status').innerText(), /CRLF/)
  await editor.click()
  await page.keyboard.press('Control+End')
  await page.keyboard.insertText('// 中文输入')
  await page.getByRole('button', { name: '输入 {', exact: true }).click()
  assert.match(await editor.innerText(), /中文输入\{/)
  await page.getByRole('button', { name: '撤销', exact: true }).click()
  assert.ok(!(await editor.innerText()).endsWith('{'))
  await page.getByRole('button', { name: '重做', exact: true }).click()
  assert.match(await editor.innerText(), /中文输入\{/)
  await page.getByRole('button', { name: '光标左移', exact: true }).click()
  await page.getByRole('button', { name: '输入 }', exact: true }).click()
  assert.match(await editor.innerText(), /中文输入\}\{/)
  const cdp = await context.newCDPSession(page)
  await cdp.send('Input.imeSetComposition', { text: '输', selectionStart: 1, selectionEnd: 1 })
  await cdp.send('Input.imeSetComposition', { text: '输入法', selectionStart: 3, selectionEnd: 3 })
  await cdp.send('Input.insertText', { text: '输入法' })
  assert.equal((await editor.innerText()).match(/输入法/g)?.length, 1)
  await cdp.detach()
  pass('特殊文件名、中文输入、符号栏、光标移动和撤销重做')

  await page.getByRole('button', { name: '查找和替换', exact: true }).click()
  await page.getByRole('textbox', { name: '查找内容', exact: true }).fill('中文')
  await visible(page.getByText('3 处匹配', { exact: true }))
  await page.getByRole('textbox', { name: '替换为', exact: true }).fill('变量')
  await page.getByRole('button', { name: '全部', exact: true }).click()
  await visible(page.getByText('没有匹配结果', { exact: true }))
  assert.ok(!(await editor.innerText()).includes('中文'))
  await page.getByRole('button', { name: '关闭查找', exact: true }).click()
  await page.getByRole('button', { name: '跳转行号', exact: true }).click()
  await page.getByRole('spinbutton', { name: '目标行号' }).fill('2')
  await page.getByRole('button', { name: '跳转', exact: true }).click()
  assert.match(await page.locator('.cursor-position').innerText(), /^2:1/)
  await page.getByRole('button', { name: '自动换行', exact: true }).click()
  assert.equal(await page.getByRole('button', { name: '自动换行', exact: true }).getAttribute('aria-pressed'), 'false')
  await page.getByRole('button', { name: '自动换行', exact: true }).click()
  pass('查找替换、行号跳转和自动换行')

  failSave = true
  await page.getByRole('button', { name: '保存文件', exact: true }).click()
  await visible(page.getByText('模拟请求失败', { exact: true }).last())
  await page.waitForFunction(() => !document.querySelector('.editor-save').disabled)
  assert.match(await page.locator('.editor-file-subtitle').innerText(), /未保存/)
  assert.ok(!(await editor.innerText()).includes('中文'))
  failSave = false; saveDelay = 400
  await page.getByRole('button', { name: '保存文件', exact: true }).click()
  await visible(page.getByRole('button', { name: '保存文件', exact: true }).filter({ hasText: '保存中' }))
  await editor.click(); await page.keyboard.press('Control+End'); await page.keyboard.insertText('继续编辑')
  await page.waitForFunction(() => !document.querySelector('.editor-save').disabled)
  assert.match(await editor.innerText(), /继续编辑/)
  assert.match(await page.locator('.editor-file-subtitle').innerText(), /未保存/)
  assert.ok(!writes.at(-1).content.includes('继续编辑'))
  assert.ok(writes.at(-1).content.includes('\r\n'))
  assert.ok(!writes.at(-1).content.replaceAll('\r\n', '').includes('\n'))
  saveDelay = 0
  pass('保存失败保留草稿；保存中输入不丢失；保留 CRLF')

  // Close confirmation: X keeps editing, discard closes, save closes after successful write.
  await page.getByRole('button', { name: '关闭文件编辑器', exact: true }).click()
  await visible(page.locator('.file-editor-unsaved-message'))
  await page.locator('.file-editor-unsaved-message .el-message-box__headerbtn').click()
  await absent(page.locator('.file-editor-unsaved-message'))
  await visible(editor)
  await page.getByRole('button', { name: '关闭文件编辑器', exact: true }).click()
  await page.getByRole('button', { name: '放弃修改', exact: true }).click()
  await absent(page.locator('.file-editor-overlay'))
  await page.getByRole('button', { name: `打开文件 ${unusualName}`, exact: true }).click()
  await visible(editor)
  assert.ok(!(await editor.innerText()).includes('继续编辑'))
  await editor.click(); await page.keyboard.press('Control+End'); await page.keyboard.insertText('最终修改')
  await page.getByRole('button', { name: '关闭文件编辑器', exact: true }).click()
  await page.getByRole('button', { name: '保存并关闭', exact: true }).click()
  await absent(page.locator('.file-editor-overlay'))
  assert.match(documents.get(unusualName), /最终修改/)
  pass('返回时继续编辑、放弃修改、保存并关闭三个分支')

  await page.getByRole('button', { name: `打开文件 ${unusualName}`, exact: true }).click()
  await visible(editor)
  await page.getByRole('button', { name: '查找和替换', exact: true }).click()
  await page.evaluate(() => window.appStore.handleBack())
  await absent(page.getByRole('textbox', { name: '查找内容', exact: true }))
  await visible(editor)
  await absent(page.locator('.el-message').last())
  await page.screenshot({ path: `${artifacts}/mobile-editor.png` })
  await page.evaluate(() => { window.appStore.isDark = true })
  await page.screenshot({ path: `${artifacts}/mobile-editor-dark.png` })
  await page.evaluate(() => {
    Object.defineProperty(window.visualViewport, 'height', { configurable: true, value: 440 })
    window.visualViewport.dispatchEvent(new Event('resize'))
  })
  await page.waitForFunction(() => document.querySelector('.file-editor-overlay').getBoundingClientRect().height <= 441)
  const symbolsBox = await page.locator('.mobile-editor-symbols').boundingBox()
  assert.ok(symbolsBox.y + symbolsBox.height <= 441)
  assert.ok((await page.locator('.mobile-editor-document').boundingBox()).height > 100)
  await page.screenshot({ path: `${artifacts}/mobile-editor-keyboard.png` })
  await page.evaluate(() => {
    delete window.visualViewport.height
    window.visualViewport.dispatchEvent(new Event('resize'))
    window.appStore.isDark = false
  })
  await page.setViewportSize({ width: 768, height: 1024 })
  await visible(editor)
  assert.equal(await page.locator('.file-editor-overlay.el-drawer').count(), 1)
  assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth))
  await page.setViewportSize({ width: 1440, height: 1000 })
  await visible(editor)
  assert.match(await editor.innerText(), /最终修改/)
  await page.getByRole('button', { name: '关闭文件编辑器', exact: true }).click()
  await absent(page.locator('.file-editor-overlay'))
  await page.screenshot({ path: `${artifacts}/desktop-files.png` })
  assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth))
  await page.setViewportSize({ width: 375, height: 812 })
  pass('明暗主题、768px 边界、窗口缩放保留内容、软键盘可见区域')

  // A delayed search result must not overwrite the directory reached with Back.
  await page.getByRole('button', { name: '打开目录 scripts', exact: true }).click()
  await visible(page.getByRole('button', { name: '打开文件 nested.js', exact: true }))
  searchDelay = 450
  await page.getByRole('textbox', { name: '搜索文件名', exact: true }).fill('old')
  await page.waitForRequest(request => request.url().includes('q=old'))
  await page.getByRole('button', { name: '返回上级目录', exact: true }).click()
  await visible(page.getByRole('button', { name: '打开目录 scripts', exact: true }))
  await sleep(550)
  assert.equal(await page.getByRole('textbox', { name: '搜索文件名', exact: true }).inputValue(), '')
  assert.equal(await page.getByRole('button', { name: '打开文件 old-search.js', exact: true }).count(), 0)
  pass('慢搜索不会覆盖导航后的目录')

  const selectedFile = page.getByRole('button', { name: `打开文件 ${unusualName}`, exact: true })
  await selectedFile.dispatchEvent('touchstart')
  await sleep(650)
  await selectedFile.dispatchEvent('touchend')
  await selectedFile.click()
  assert.equal(await page.getByRole('button', { name: `取消选择 ${unusualName}`, exact: true }).count(), 1)
  await absent(page.locator('.file-editor-overlay'))
  await page.screenshot({ path: `${artifacts}/mobile-selection.png` })
  assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth))
  await page.setViewportSize({ width: 320, height: 740 })
  for (const action of ['复制', '剪切', '移动', '打包下载', '删除']) {
    const box = await page.getByRole('button', { name: action, exact: true }).boundingBox()
    assert.ok(box.x >= 0 && box.x + box.width <= 320 && box.width >= 44 && box.height >= 44)
  }
  await page.setViewportSize({ width: 375, height: 812 })
  await page.getByRole('button', { name: '复制', exact: true }).click()
  await visible(page.getByText('待复制 1 项', { exact: true }))
  await page.getByRole('button', { name: '打开目录 target', exact: true }).click()
  await visible(page.getByText('目标：target', { exact: true }))
  await page.screenshot({ path: `${artifacts}/mobile-clipboard.png` })
  await page.getByRole('button', { name: '粘贴到此处', exact: true }).click()
  await absent(page.locator('.file-clipboard-bar'))
  assert.deepEqual(copies.at(-1), { from_path: unusualName, to_path: `target/${unusualName}` })
  copies.length = 0
  pass('长按选择不会误打开文件，复制后可直接导航并粘贴')

  await page.getByRole('button', { name: '返回上级目录', exact: true }).click()
  await page.getByRole('button', { name: `选择 ${unusualName}`, exact: true }).click()
  await page.getByRole('button', { name: '删除', exact: true }).click()
  await page.locator('.el-message-box').getByRole('button', { name: '取消', exact: true }).click()
  await absent(page.locator('.el-message-box'))
  assert.equal(requests.filter(request => request.method === 'DELETE').length, 0)
  await page.getByRole('button', { name: '选择 broken.py', exact: true }).click()
  failDelete = 'broken.py'
  await page.getByRole('button', { name: '删除', exact: true }).click()
  await page.locator('.el-message-box').getByRole('button', { name: '删除', exact: true }).click()
  await visible(page.getByText('1 项删除失败，可重试', { exact: true }))
  await absent(page.locator('.el-message-box'))
  await page.locator('.file-selection-bar').getByRole('button', { name: '删除', exact: true }).waitFor({ state: 'visible' })
  await page.waitForFunction(() => !document.querySelector('.file-selection-bar .selection-danger').disabled)
  assert.equal(await page.getByRole('button', { name: `打开文件 ${unusualName}`, exact: true }).count(), 0)
  assert.equal(await page.getByRole('button', { name: '取消选择 broken.py', exact: true }).count(), 1)
  failDelete = ''
  await page.getByRole('button', { name: '删除', exact: true }).click()
  await page.locator('.el-message-box').getByRole('button', { name: '删除', exact: true }).click()
  await absent(page.locator('.file-selection-bar'))
  pass('320px 多选按钮完整可见；取消删除不报错；部分失败保留选择并可重试')

  await page.evaluate(() => window.router.push('/logic'))
  await page.waitForFunction(() => !!window.logic)
  await page.evaluate(async () => {
    const slow = window.logic.showEditFileDialog({ name: 'slow.js', path: 'slow.js' })
    await window.logic.showEditFileDialog({ name: 'fast.js', path: 'fast.js' })
    await slow
  })
  assert.equal(await page.evaluate(() => window.logic.fileContent), 'fast.js')
  pass('旧读取响应不能覆盖新打开的文件')

  copyDelay = 200; failCopy = 'source/2.js'
  await page.evaluate(() => {
    window.logic.currentPath = 'target'
    window.logic.copyToClipboard(Array.from({ length: 7 }, (_, i) => ({ name: i + '.js', path: 'source/' + i + '.js', is_dir: false, size: 10 })))
    window.pastePromise = window.logic.pasteFromClipboard()
  })
  await page.waitForRequest(request => request.url().endsWith('/files/copy'))
  await page.evaluate(() => { window.logic.currentPath = 'elsewhere' })
  await page.evaluate(() => window.pastePromise)
  assert.equal(copies.length, 7)
  assert.ok(copies.every(payload => payload.to_path.startsWith('target/')))
  assert.deepEqual(await page.evaluate(() => window.logic.clipboard.files.map(file => file.path)), ['source/2.js'])
  failCopy = ''; copyDelay = 0
  await page.evaluate(async () => { window.logic.currentPath = 'target'; await window.logic.pasteFromClipboard() })
  assert.equal(await page.evaluate(() => window.logic.clipboard.files.length), 0)
  assert.deepEqual(errors, [])
  pass('多批粘贴固定目标目录，部分失败保留重试项；无浏览器运行错误')
  console.log(`Screenshots: ${artifacts}`)
} catch (error) {
  if (page) {
    await page.screenshot({ path: `${artifacts}/failure.png` }).catch(() => {})
    console.error((await page.locator('body').innerText().catch(() => '')).slice(0, 2500))
  }
  throw error
} finally {
  await browser?.close()
  await rm(harness, { force: true })
}
