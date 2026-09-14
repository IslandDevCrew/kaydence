const assert = require('node:assert/strict');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const url = process.env.APP_URL || 'http://127.0.0.1:1432';
async function focusInfo(page) {
  return page.evaluate(() => {
    const scope=document.querySelector('.cleanup-board'),el=document.activeElement;
    if(!scope?.contains(el))return null;
    const r=el.getBoundingClientRect(),s=getComputedStyle(el),ring=Math.max(0,parseFloat(s.outlineWidth)+parseFloat(s.outlineOffset));
    let top=0,bottom=innerHeight,left=0,right=innerWidth;
    for(let a=el.parentElement;a;a=a.parentElement){const v=getComputedStyle(a),b=a.getBoundingClientRect();if(/hidden|auto|scroll|clip/.test(v.overflowY)){top=Math.max(top,b.top);bottom=Math.min(bottom,b.bottom);}if(/hidden|auto|scroll|clip/.test(v.overflowX)){left=Math.max(left,b.left);right=Math.min(right,b.right);}}
    return{id:[...scope.querySelectorAll('*')].indexOf(el),text:el.getAttribute('aria-label')||el.textContent.trim().slice(0,80),class:el.className,visible:r.top-ring>=top&&r.bottom+ring<=bottom&&r.left-ring>=left&&r.right+ring<=right,outline:s.outlineStyle,ring,rect:{top:r.top,bottom:r.bottom,left:r.left,right:r.right},clip:{top,bottom,left,right}};
  });
}
async function naturalWalk(page,failures,tag){
  const seen=new Set(),controls=[];
  for(let n=0;n<85;n++){
    await page.keyboard.press('Tab');const info=await focusInfo(page);
    if(!info){if(seen.size)break;continue;}if(seen.has(info.id))break;seen.add(info.id);controls.push(info.text);
    if(!info.visible||info.outline==='none')failures.push({tag,kind:'focus',...info});
  }
  assert(controls.some(t=>t==='Check permissions')&&controls.some(t=>t==='Save Changes'),'natural action traversal');
  return controls;
}
async function wheelRead(page,selector,failures,tag,skip=''){
  const area=page.locator(selector),box=await area.boundingBox(),vp=page.viewportSize();
  await page.mouse.move(Math.max(5,Math.min(vp.width-5,box.x+box.width/2)),Math.max(5,Math.min(vp.height-5,box.y+box.height/2)));
  await page.waitForTimeout(250);await page.mouse.wheel(0,-15000);await page.waitForTimeout(250);
  const seen=new Set();let fragments=[];
  for(let n=0;n<85;n++){
    fragments=await area.evaluate((el,skip)=>{
      const walker=document.createTreeWalker(el,NodeFilter.SHOW_TEXT),out=[];let node,id=0;
      while(node=walker.nextNode()){
        if(!node.textContent.trim()||(skip&&node.parentElement.closest(skip)))continue;
        const range=document.createRange();range.selectNodeContents(node);let line=0;
        for(const r of range.getClientRects()){
          if(!r.width||!r.height)continue;let top=0,bottom=innerHeight,left=0,right=innerWidth;
          for(let a=node.parentElement;a;a=a.parentElement){const s=getComputedStyle(a),b=a.getBoundingClientRect();if(/hidden|auto|scroll|clip/.test(s.overflowY)){top=Math.max(top,b.top);bottom=Math.min(bottom,b.bottom);}if(/hidden|auto|scroll|clip/.test(s.overflowX)){left=Math.max(left,b.left);right=Math.min(right,b.right);}}
          out.push({id:`${id}:${line++}`,text:node.textContent.trim().slice(0,80),visible:r.top>=top-1&&r.bottom<=bottom+1&&r.left>=left-1&&r.right<=right+1});
        }id++;
      }return out;
    },skip);
    for(const f of fragments)if(f.visible)seen.add(f.id);
    if(seen.size===fragments.length)return{selector,skip,fragments:seen.size};
    await page.mouse.wheel(0,Math.max(4,Math.min(vp.height,box.height)/4));await page.waitForTimeout(40);
  }
  failures.push({tag,kind:'wheel',selector,seen:seen.size,total:fragments.length,missing:fragments.filter(f=>!seen.has(f.id))});
  return{selector,fragments:seen.size,total:fragments.length};
}
(async()=>{
  const browser=await chromium.launch(),failures=[],metrics=[];
  try{for(const theme of ['light','dark'])for(const width of [900,450,500,501])for(const lane of ['macOS','Windows','Linux']){
    const height=width===900?600:300,tag=`${theme}-${width}x${height}-${lane}`,page=await browser.newPage({viewport:{width,height},colorScheme:theme,reducedMotion:'reduce'});
    page.on('pageerror',e=>failures.push({tag,kind:'pageerror',text:e.message}));
    await page.goto(url);await page.locator('.nav-list').getByRole('button',{name:'Cleanup',exact:true}).click();
    await page.locator('.cleanup-lane-switcher').getByRole('button',{name:lane,exact:true}).click();
    for(let n=0;n<20;n++){await page.keyboard.press('Shift+Tab');if(!await focusInfo(page))break;}
    const controls=await naturalWalk(page,failures,tag),wheel=[];
    if(width===900){wheel.push(await wheelRead(page,'.cleanup-board',failures,tag,'.cleanup-main-grid'));for(const s of ['.cleanup-rules-panel','.cleanup-injection-panel'])wheel.push(await wheelRead(page,s,failures,tag));}
    else wheel.push(await wheelRead(page,'.cleanup-board',failures,tag));
    if(process.env.CAPTURE_DIR&&lane==='macOS')await page.screenshot({path:`${process.env.CAPTURE_DIR}/cleanup-independent-${theme}-${width}.png`,animations:'disabled'});
    metrics.push({tag,controls,wheel});console.log(`CHECK ${tag}`);await page.close();
  }console.log(JSON.stringify({metrics,failures},null,2));assert.deepEqual(failures,[]);console.log(`PASS ${metrics.length} independent Cleanup keyboard/wheel cases`);}
  finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
