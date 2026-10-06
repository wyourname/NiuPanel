import assert from 'node:assert/strict'
import { randomUUID } from 'node:crypto'
import { writeFile, unlink } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright'

// Exercises real stores and components through Docker Vite, with no writes to user data.
const origin = process.env.FILE_UI_URL || 'http://127.0.0.1:7787'
const name = `reliability-${randomUUID()}.html`
const path = fileURLToPath(new URL(`../${name}`, import.meta.url))
const html = `<!doctype html><html><head><meta name="viewport" content="width=device-width,initial-scale=1"></head><body><div id="app"></div><script type="module">
import {createApp,h,ref,computed,proxyRefs,nextTick} from 'vue';
import {createPinia} from 'pinia';
import ElementPlus from 'element-plus';
import 'element-plus/dist/index.css';
import '/src/assets/styles/index.css';
import 'virtual:uno.css';
import {createLogConnection} from '/src/utils/logConnection.ts';
import {useEnvironmentJobsStore} from '/src/stores/environmentJobs.ts';
import {useFileTransfersStore} from '/src/stores/fileTransfers.ts';
import {useAppStore} from '/src/stores/app.ts';
import {useUserStore} from '/src/stores/user.ts';
import {useMobileBackCloseAction} from '/src/composables/useMobileBackCloseAction.ts';
import {useEnvPackageManager} from '/src/views/modules/environment/composables/useEnvPackageManager.ts';
import {useFileTransfers} from '/src/composables/file/useFileTransfers.ts';
import request from '/src/utils/request.ts';
import MobileSelectionBar from '/src/components/common/MobileSelectionBar.vue';
import FileBulkActions from '/src/views/modules/file/components/FileBulkActions.vue';
import TaskMobileListPane from '/src/components/tasks/TaskMobileListPane.vue';
import FileTransferPanel from '/src/views/modules/file/components/FileTransferPanel.vue';
const pinia=createPinia();
const mode=ref('none'),selected=ref(true),overlay=ref(false),blocked=ref(false);
createApp({setup(){
 const app=useAppStore();
 useMobileBackCloseAction({appStore:app,visible:selected,close:()=>{selected.value=false}});
 useMobileBackCloseAction({appStore:app,visible:overlay,close:()=>{if(blocked.value)return false;overlay.value=false}});
 const env=ref({env_type:'python',name:'3.12',path:'venv',version:'3.12'}),visible=ref(false);
 const manager=useEnvPackageManager({env,isVisible:visible,onShowLog:()=>{}});
 let refreshes=0;
 useFileTransfers({currentPath:ref('/target'),loading:ref(false),loadContents:async()=>{refreshes++;window.uploadRefreshes=refreshes}});
 window.manager=proxyRefs({...manager,env,visible});
 return ()=>h('main',{style:'height:100%;display:flex;flex-direction:column'},[
  h(FileTransferPanel),
  mode.value==='file'?h(FileBulkActions,{count:selected.value?2:0,isAllSelected:false,onCancel:()=>selected.value=false}):null,
  mode.value==='task'?h(TaskMobileListPane,{allTasks:[],tasks:[],selectedIds:selected.value?[1,2]:[],selectedTasks:[],selectionMode:selected.value,searchQuery:'',statusFilter:'all',loading:false,totalTasks:0,noMore:true,isAllSelected:false,refreshTasks:()=>{},onCancelSelection:()=>selected.value=false}):null,
  mode.value==='variable'?h(MobileSelectionBar,{count:selected.value?2:0,isAllSelected:false,actions:[{command:'enable',label:'启用',icon:'i-ep-check'},{command:'disable',label:'禁用',icon:'i-ep-close'},{command:'delete',label:'删除',icon:'i-ep-delete',danger:true}],onCancel:()=>selected.value=false}):null
 ]);
}}).use(pinia).use(ElementPlus).mount('#app');
Object.assign(window,{createLogConnection,request,app:useAppStore(),user:useUserStore(),jobs:useEnvironmentJobsStore(),transfers:useFileTransfersStore(),nextTick,ui:proxyRefs({mode,selected,overlay,blocked})});
</script><style>html,body,#app{margin:0;width:100%;height:100%;overflow:hidden}</style></body></html>`
let browser
try {
  await writeFile(path, html)
  browser = await chromium.launch({ headless: true, ...(process.env.CHROMIUM_PATH ? { executablePath: process.env.CHROMIUM_PATH } : {}) })
  const page = await browser.newPage({ viewport: { width: 375, height: 812 }, acceptDownloads: true })
  page.setDefaultTimeout(15000)
  const errors = []
  page.on('pageerror', error => errors.push(error.message))
  await page.route('**/api/v1/**', route => route.fulfill({ json: { code: 200, data: [], message: 'ok' } }))
  await page.goto(`${origin}/${name}`)
  await page.waitForFunction(() => !!window.transfers)

  const logs = await page.evaluate(async () => {
    const check=(ok,message)=>{if(!ok)throw Error(message)};
    const flush=()=>new Promise(resolve=>setTimeout(resolve,0));
    class Source extends EventTarget {
      closed=false;
      close(){this.closed=true}
      send(type,data='',id=''){this.dispatchEvent(new MessageEvent(type,{data,lastEventId:String(id)}))}
      fail(){this.onerror?.()}
    }
    const sources=[],states=[],writes=[];
    let output='', reads=0;
    const missing='断线日志🙂\n'.repeat(15000);
    const bytes=new TextEncoder().encode('初始\n'+missing);
    const initial=new TextEncoder().encode('初始\n').length;
    const connection=window.createLogConnection({
      open:()=>{const s=new Source();sources.push(s);return s},status:s=>states.push(s),
      write:(text,reset)=>{writes.push(reset);output=reset?text:output+text},
      readMissing:async(offset,limit)=>{reads++;let end=Math.min(bytes.length,offset+limit);while(end<bytes.length&&(bytes[end]&0xc0)===0x80)end--;return {offset,length:end-offset,content:new TextDecoder().decode(bytes.slice(offset,end))}}
    });
    sources[0].send('history','初始\n',initial);await flush();
    sources[0].fail();await flush();
    check(states.at(-1)==='reconnecting'&&output==='初始\n','disconnect retains logs');
    await new Promise(resolve=>setTimeout(resolve,1100));
    check(sources.length===2,'automatic reconnect');
    sources[1].send('history','only a tail',bytes.length);
    sources[1].send('log',JSON.stringify({content:'duplicate',offset:bytes.length}));
    sources[1].send('log',JSON.stringify({content:'末行\n',offset:bytes.length+7}));
    await flush();
    check(output==='初始\n'+missing+'末行\n'&&reads>1&&writes.filter(Boolean).length===1,'paged catch-up and duplicate suppression');
    sources[1].send('log',JSON.stringify({content:'末行\n追加\n',offset:bytes.length+14}));await flush();
    check(output==='初始\n'+missing+'末行\n追加\n','partially overlapping event only appends unseen bytes');
    sources[1].send('end');sources[1].fail();await flush();
    check(states.at(-1)==='ended','normal completion does not reconnect');
    await new Promise(resolve=>setTimeout(resolve,1100));check(sources.length===2,'ended stays closed');
    connection.close();
    let resolveRead;const late=[],lateStates=[];
    const c=window.createLogConnection({open:()=>{const s=new Source();late.push(s);return s},status:s=>lateStates.push(s),write:(s,r)=>{output=r?s:output+s},readMissing:()=>new Promise(r=>resolveRead=r)});
    late[0].send('history','old',3);await flush();c.retry();late[1].send('history','new tail',10);await flush();c.close();resolveRead({offset:3,length:7,content:'stale!!'});await flush();check(output==='old','closed stream discards delayed reads');
    const off=[],offStates=[];const c2=window.createLogConnection({open:()=>{const s=new Source();off.push(s);return s},status:s=>offStates.push(s),write:(s,r)=>{output=r?s:output+s}});
    off[0].send('history','old',3);await flush();
    Object.defineProperty(navigator,'onLine',{configurable:true,value:false});window.dispatchEvent(new Event('offline'));
    check(off[0].closed&&offStates.at(-1)==='offline','offline disconnect');
    Object.defineProperty(navigator,'onLine',{configurable:true,value:true});window.dispatchEvent(new Event('online'));
    off[1].send('history','x',1);await flush();check(output==='x','truncated file snapshot replaces old text');
    off[1].send('resync');await flush();check(offStates.at(-1)==='reconnecting','buffer overflow resync');c2.close();delete navigator.onLine;
    return ['自动重连、离线恢复、分页补齐中文日志、重复去除、正常结束、关闭后旧响应隔离、日志截断和缓冲溢出'];
  })
  console.log('PASS:', logs.join('; '))

  await page.evaluate(async () => {
    const check=(ok,message)=>{if(!ok)throw Error(message)};
    const job=(id,env_type,env_name,status)=>({id,name:'packages',status,metadata:{kind:'environment-packages',env_type,env_name}});
    window.jobData=[job(1,'python','3.12','Running'),job(2,'node','22','Pending'),job(3,'sh','System','Finished'),job(4,'python','3.12','Finished')];
    window.requests=[];window.jobFail=false;
    window.request.defaults.adapter=async config=>{
      window.requests.push(config.url);
      if(config.url==='/jobs'&&window.jobFail)throw Error('offline');
      return {data:{code:200,data:config.url==='/jobs'?structuredClone(window.jobData):[],message:'ok'},status:200,statusText:'OK',headers:{},config};
    };
    await window.jobs.restore();
    check(window.jobs.jobs['python:3.12'].id===1&&window.jobs.jobs['node:22'].id===2&&window.jobs.jobs['sh:System'].id===3,'environment association and active priority');
    window.manager.visible=true;await window.nextTick();await window.jobs.restore();
    check(window.manager.busy,'restored job blocks duplicate install');
    window.manager.env={env_type:'node',name:'24'};await window.nextTick();await window.jobs.restore();check(!window.manager.busy,'different runtime remains usable');
    window.manager.env={env_type:'python',name:'3.12'};await window.nextTick();await window.jobs.restore();
    let finished=0;window.addEventListener('niu:job-finished',()=>finished++);
    window.jobData=window.jobData.filter(j=>j.id!==4);window.jobData[0].status='Finished';await window.jobs.restore();
    check(!window.manager.busy&&finished===1,'finished job unlocks and refreshes packages');
    window.jobFail=true;await window.jobs.restore();check(!!window.jobs.error&&window.manager.busy,'unknown state blocks duplicate submissions');
    window.jobFail=false;await window.jobs.restore();check(!window.jobs.error,'restore retries');
    const adapter=window.request.defaults.adapter;
    let release;window.request.defaults.adapter=config=>new Promise(resolve=>release=()=>resolve({data:{data:[job(99,'python','old','Running')]},status:200,headers:{},config}));
    const stale=window.jobs.restore();await new Promise(r=>setTimeout(r,0));
    window.user.userInfo.username='other-account';await window.nextTick();release();await stale;
    check(Object.keys(window.jobs.jobs).length===0,'old account response cannot overwrite current jobs');
    window.request.defaults.adapter=adapter;window.manager.visible=false;
    window.jobData=[job(22,'python','3.12','Pending')];await window.jobs.restore();
  })
  console.log('PASS: 按环境恢复安装状态、阻止重复安装、完成后解锁刷新、失败重试、账号切换隔离')

  // Actual reload creates a fresh Pinia store and recovers from server metadata.
  await page.route('**/api/v1/jobs', route => route.fulfill({ json: { code: 200, data: [{ id: 22, name: '恢复安装', status: 'Pending', metadata: { kind: 'environment-packages', env_type: 'python', env_name: '3.12' } }] } }))
  await page.reload();await page.waitForFunction(() => !!window.jobs)
  await page.evaluate(() => window.jobs.restore())
  assert.equal(await page.evaluate(() => window.jobs.jobs['python:3.12']?.id), 22)
  console.log('PASS: 刷新页面后从服务器恢复未结束的安装作业')

  await page.evaluate(async () => {
    const check=(ok,message)=>{if(!ok)throw Error(message)};
    const originalFetch=window.fetch, originalPicker=window.showSaveFilePicker;
    let writes=0,bytes=0,closed=0,aborted=0,picked=0,slow=false,fail=false;
    window.showSaveFilePicker=async()=>{picked++;return {createWritable:async()=>new WritableStream({write(chunk){writes++;bytes+=chunk.length},close(){closed++},abort(){aborted++}})}};
    window.fetch=async()=>{if(fail)return new Response('',{status:503});let sent=0;return new Response(new ReadableStream({async pull(controller){if(slow)await new Promise(r=>setTimeout(r,15));if(sent++<20)controller.enqueue(new Uint8Array(65536));else controller.close()},cancel(){aborted++}}),{headers:{'Content-Length':String(20*65536)}})};
    await window.transfers.download('大文件.bin',['data.bin'],false);
    check(writes===20&&bytes===20*65536&&closed===1&&window.transfers.items[0].state==='success','streams directly to writable destination');
    slow=true;const pending=window.transfers.download('取消.bin',['cancel.bin'],false);await new Promise(r=>setTimeout(r,50));const id=window.transfers.items[0].id;window.transfers.cancel(id);await pending;
    check(window.transfers.items[0].state==='cancelled'&&aborted>0,'cancel aborts stream and writable');
    const pickCount=picked;slow=false;await window.transfers.retry(id);check(window.transfers.items[0].state==='success'&&picked===pickCount,'retry reuses chosen destination');
    fail=true;await window.transfers.download('重试.bin',['retry.bin'],false);check(window.transfers.items[0].state==='error','HTTP failure visible');fail=false;await window.transfers.retry(window.transfers.items[0].id);check(window.transfers.items[0].state==='success','HTTP failure retry');
    window.fetch=originalFetch;window.showSaveFilePicker=originalPicker;
    let uploadFailure=true,uploads=0;
    window.request.defaults.adapter=async config=>{if(config.url.includes('/upload')){uploads++;check(config.timeout===0,'uploads have no 30-second timeout');if(uploadFailure)throw Error('网络中断')};return {status:200,headers:{},config,data:{code:200,data:[]}}};
    const files=[new File(['content'],'上传.py')];await window.transfers.upload(files,'/target');check(window.transfers.items[0].state==='error','upload failure retained');uploadFailure=false;await window.transfers.retry(window.transfers.items[0].id);
    check(uploads===2&&window.uploadRefreshes===1,'upload retry refreshes current destination');
  })
  console.log('PASS: 大文件逐块写入、进度、取消与重试、复用保存位置、上传重试刷新目录')

  // Browser-managed fallback must initiate real downloads, including a native POST form.
  await page.evaluate(() => { delete window.showSaveFilePicker })
  await page.route('**/api/v1/files/download/**', route => route.fulfill({ body: 'native download', headers: { 'Content-Disposition': "attachment; filename*=UTF-8''test.py", 'Content-Type': 'application/octet-stream' } }))
  let event=page.waitForEvent('download');await page.evaluate(() => window.transfers.download('test.py',['test.py'],false));assert.equal((await event).suggestedFilename(),'test.py')
  let batchPayload
  await page.route('**/api/v1/files/download_batch/form', route => {batchPayload=route.request().postData();return route.fulfill({body:'tar test',headers:{'Content-Disposition':'attachment; filename="files.tar"','Content-Type':'application/x-tar'}})})
  event=page.waitForEvent('download');await page.evaluate(() => window.transfers.download('files.tar',['中文.py','space file'],true));await event
  assert.deepEqual(JSON.parse(new URLSearchParams(batchPayload).get('paths')),['中文.py','space file'])
  assert.equal(await page.evaluate(() => window.transfers.items[0].state),'browser')
  console.log('PASS: 不支持直接写入文件的浏览器使用原生下载，批量 TAR 通过表单下载')

  await page.evaluate(()=>{for(const item of [...window.transfers.items])window.transfers.dismiss(item.id)})
  for (const width of [320,375,768]) {
    await page.setViewportSize({width,height:900})
    for(const mode of ['file','task','variable']) {
      await page.evaluate(mode=>{window.ui.mode=mode;window.ui.selected=true},mode)
      await page.locator('.mobile-selection-bar').waitFor({state:'visible'})
      for(const button of await page.locator('.mobile-selection-bar button').all()) {
        const box=await button.boundingBox();assert.ok(box&&box.width>=44&&box.height>=44&&box.x>=0&&box.x+box.width<=width,`${mode} ${width}: ${JSON.stringify(box)}`)
      }
      assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth))
      await page.evaluate(async()=>{window.ui.overlay=true;await window.nextTick();window.ui.blocked=true;await window.app.handleBack();if(!window.ui.overlay)throw Error('busy overlay closed');window.ui.blocked=false;await window.app.handleBack();if(window.ui.overlay||!window.ui.selected)throw Error('top overlay must close first');await window.app.handleBack();if(window.ui.selected)throw Error('back must exit selection')})
      await page.locator('.mobile-selection-bar').waitFor({state:'hidden'})
    }
  }
  assert.deepEqual(errors,[])
  console.log('PASS: 320/375/768px 文件、任务、环境变量多选栏按钮 ≥44px；返回先关顶层，再退多选；无运行错误')
} finally {
  await browser?.close()
  await unlink(path).catch(()=>{})
}
