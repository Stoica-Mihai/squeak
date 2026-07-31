/* ===== Futurism Design System JS (vendored from futurism-design 0.6.9) ===== */
function fdOptValue(o){return o.dataset.value!==undefined?o.dataset.value:o.textContent}
function fdSelPosition(sel){
  var list=sel.querySelector('.sel-list'),v=sel.querySelector('.sel-val');
  if(!list||!v)return;
  var r=v.getBoundingClientRect();
  var h=Math.min(list.scrollHeight,240);
  var below=window.innerHeight-r.bottom,up=below<h+8&&r.top>below;
  list.style.left=r.left+'px';
  list.style.width=r.width+'px';
  list.style.top=(up?r.top-h-4:r.bottom+4)+'px';
  list.style.transformOrigin=up?'bottom':'top';
}
function fdSelOpen(sel,open){
  if(open)document.querySelectorAll('.sel.open').forEach(function(o){if(o!==sel)fdSelOpen(o,false)});
  sel.classList.toggle('open',open);
  var v=sel.querySelector('.sel-val');
  if(v)v.setAttribute('aria-expanded',open?'true':'false');
  if(!sel._fdReposition)sel._fdReposition=function(){fdSelPosition(sel)};
  if(open){
    fdSelPosition(sel);
    window.addEventListener('scroll',sel._fdReposition,true);
    window.addEventListener('resize',sel._fdReposition);
    var cur=sel.querySelector('.sel-opt.sel-on')||sel.querySelector('.sel-opt');if(cur)cur.focus();
  }else{
    window.removeEventListener('scroll',sel._fdReposition,true);
    window.removeEventListener('resize',sel._fdReposition);
  }
}
function fdSel(opt){
  var sel=opt.closest('.sel');
  sel.querySelectorAll('.sel-opt').forEach(function(o){o.classList.remove('sel-on');o.setAttribute('aria-selected','false')});
  opt.classList.add('sel-on');opt.setAttribute('aria-selected','true');
  sel.querySelector('.sel-cur').textContent=opt.textContent;
  sel.dataset.value=fdOptValue(opt);
  fdSelOpen(sel,false);
  var v=sel.querySelector('.sel-val');if(v)v.focus();
}
function fdSelVal(sel){return sel?(sel.dataset.value||''):''}
var fdUid=0;
function fdId(el,prefix){if(!el.id)el.id=prefix+(fdUid++);return el.id}
function fdInit(root){
  root=root||document;
  root.querySelectorAll('.sel').forEach(function(sel){
    var v=sel.querySelector('.sel-val'),list=sel.querySelector('.sel-list');
    if(list)list.setAttribute('role','listbox');
    if(v){v.setAttribute('role','button');v.setAttribute('aria-haspopup','listbox');v.setAttribute('aria-expanded','false');
      if(list)v.setAttribute('aria-controls',fdId(list,'fd-list-'));if(!v.hasAttribute('tabindex'))v.tabIndex=0}
    sel.querySelectorAll('.sel-opt').forEach(function(o){o.setAttribute('role','option');o.setAttribute('aria-selected',o.classList.contains('sel-on')?'true':'false');o.tabIndex=-1});
  });
  root.querySelectorAll('.toggle').forEach(function(t){
    if(t.tagName!=='BUTTON'){t.setAttribute('role','switch');if(!t.hasAttribute('tabindex'))t.tabIndex=0}
    t.setAttribute('aria-checked',t.classList.contains('on')?'true':'false');
  });
}
if(document.readyState!=='loading')fdInit();else document.addEventListener('DOMContentLoaded',function(){fdInit()});
function fdTheme(root){root=root||document.documentElement;root.setAttribute('data-theme',root.getAttribute('data-theme')==='dark'?'light':'dark')}
function fdToast(msg,opts){
  opts=opts||{};
  var wrap=document.querySelector('.toaster');
  if(!wrap){wrap=document.createElement('div');wrap.className='toaster';wrap.setAttribute('role','status');wrap.setAttribute('aria-live','polite');document.body.appendChild(wrap)}
  var t=document.createElement('div');
  t.className='toast'+(opts.type==='err'?' err':'');
  t.textContent=msg;
  wrap.appendChild(t);
  setTimeout(function(){t.classList.add('out');setTimeout(function(){if(t.parentNode)t.remove()},220)},opts.timeout||3200);
  return t;
}
function fdDrawer(panel,scrim){
  panel=typeof panel==='string'?document.getElementById(panel):panel;
  if(!panel)return;
  var open=panel.classList.toggle('drawer-open');
  scrim=typeof scrim==='string'?document.getElementById(scrim):scrim;
  if(scrim)scrim.style.display=open?'block':'none';
}
function fdAccent(pick,accents,onChange){
  pick=typeof pick==='string'?document.getElementById(pick):pick;
  if(!pick)return;
  var trig=pick.querySelector('.acctrig'),pop=pick.querySelector('.accpop'),current;
  var saved=localStorage.getItem('fd-accent')||accents[0].name;
  function dark(){return document.documentElement.getAttribute('data-theme')==='dark'}
  function apply(a){
    current=a;
    var col=dark()?a.dark:a.light,r=document.documentElement.style;
    r.setProperty('--accent',col);
    r.setProperty('--shadow',dark()?col:'#1a1714');
    localStorage.setItem('fd-accent',a.name);
    if(trig)trig.style.background=col;
    render(a);
    if(onChange)onChange(a);
  }
  function render(cur){
    if(!pop)return;pop.innerHTML='';
    accents.forEach(function(a){
      var s=document.createElement('button');
      var on=a.name===cur.name;
      s.className='acc'+(on?' on':'');
      s.style.background=dark()?a.dark:a.light;s.title=a.name;
      s.setAttribute('aria-label',a.name);s.setAttribute('aria-pressed',on?'true':'false');
      s.onclick=function(){apply(a);pick.classList.remove('open');if(trig)trig.setAttribute('aria-expanded','false')};
      pop.appendChild(s);
    });
  }
  if(trig){
    trig.setAttribute('aria-haspopup','true');trig.setAttribute('aria-expanded','false');
    trig.onclick=function(){var o=pick.classList.toggle('open');trig.setAttribute('aria-expanded',o?'true':'false')};
  }
  apply(accents.find(function(a){return a.name===saved})||accents[0]);
  if(pick._fdObs)pick._fdObs.disconnect();
  pick._fdObs=new MutationObserver(function(){if(current)apply(current)});
  pick._fdObs.observe(document.documentElement,{attributes:true,attributeFilter:['data-theme']});
}
document.addEventListener('click',function(e){
  var val=e.target.closest('.sel-val');
  if(val){var s=val.closest('.sel');fdSelOpen(s,!s.classList.contains('open'))}
  var op=e.target.closest('.sel-opt');
  if(op){fdSel(op)}
  document.querySelectorAll('.sel.open').forEach(function(s){if(!s.contains(e.target))fdSelOpen(s,false)});
  document.querySelectorAll('.accpick.open').forEach(function(p){if(!p.contains(e.target)){p.classList.remove('open');var t=p.querySelector('.acctrig');if(t)t.setAttribute('aria-expanded','false')}});
},false);
document.addEventListener('keydown',function(e){
  if(!e.target||!e.target.closest)return;
  var sel=e.target.closest('.sel');
  if(sel){
    var opts=Array.prototype.slice.call(sel.querySelectorAll('.sel-opt'));
    var open=sel.classList.contains('open'),i=opts.indexOf(e.target);
    if(e.key==='ArrowDown'){e.preventDefault();if(!open)fdSelOpen(sel,true);else if(i<opts.length-1)opts[i+1].focus()}
    else if(e.key==='ArrowUp'){e.preventDefault();if(i>0)opts[i-1].focus()}
    else if(e.key==='Enter'||e.key===' '){e.preventDefault();if(!open)fdSelOpen(sel,true);else if(i>-1)fdSel(opts[i])}
    else if(e.key==='Escape'){if(open){e.preventDefault();fdSelOpen(sel,false);var v=sel.querySelector('.sel-val');if(v)v.focus()}}
    else if(e.key==='Tab'){if(open)fdSelOpen(sel,false)}
    return;
  }
  if(e.key==='Escape'){
    document.querySelectorAll('.accpick.open').forEach(function(p){p.classList.remove('open');var t=p.querySelector('.acctrig');if(t){t.setAttribute('aria-expanded','false');t.focus()}});
    var drawers=document.querySelectorAll('.drawer.drawer-open');
    if(drawers.length){drawers.forEach(function(d){d.classList.remove('drawer-open')});document.querySelectorAll('.scrim-bg').forEach(function(s){s.style.display='none'})}
  }
},false);
