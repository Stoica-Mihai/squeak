const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const LOD = { 1: "1.0 mm", 2: "2.0 mm", 3: "0.7 mm" }; // device codes (0 is not settable)

// Sidebar sections — inline stroke SVG icons (currentColor, square/miter).
const NAV = [
  ["overview", "Overview", '<rect x="3" y="4" width="18" height="16"/><path d="M3 9h18"/>'],
  ["dpi", "DPI", '<circle cx="12" cy="12" r="9"/><circle cx="12" cy="12" r="2"/>'],
  ["polling", "Polling", '<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>'],
  ["sensor", "Sensor", '<circle cx="12" cy="12" r="9"/><circle cx="12" cy="12" r="4"/>'],
  ["buttons", "Buttons", '<rect x="4" y="4" width="7" height="7"/><rect x="13" y="4" width="7" height="7"/><rect x="4" y="13" width="7" height="7"/><rect x="13" y="13" width="7" height="7"/>'],
  ["profiles", "Profiles", '<rect x="4" y="4" width="16" height="16"/><path d="M4 9h16"/>'],
];

const LOCK_SVG = '<svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="miter"><rect x="5" y="11" width="14" height="9"/><path d="M8 11V7a4 4 0 0 1 8 0v4"/></svg>';
const CHECK_SVG = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="square"><path d="M4 12l5 5L20 6"/></svg>';
const BOLT_SVG = '<svg viewBox="0 0 24 24" width="11" height="11" fill="currentColor" style="margin-right:3px;vertical-align:-1px"><path d="M13 2 4 14h6l-1 8 9-12h-6z"/></svg>';

const ACCENTS = [
  { name: "red", light: "#d22f1a", dark: "#ff4d33" },
  { name: "orange", light: "#d2691a", dark: "#ff8c33" },
  { name: "amber", light: "#bd8b00", dark: "#ffc233" },
  { name: "green", light: "#2f8a3e", dark: "#54d168" },
  { name: "teal", light: "#0d8a85", dark: "#33d6cf" },
  { name: "blue", light: "#1a5fd2", dark: "#4d8cff" },
  { name: "violet", light: "#7a3ad2", dark: "#a366ff" },
  { name: "pink", light: "#c81a7a", dark: "#ff4da6" },
];

const ACTION_LABELS = {
  leftDouble: "Double-click",
  upScroll: "Scroll ↑", downScroll: "Scroll ↓",
  leftScroll: "Scroll ←", rightScroll: "Scroll →",
};
function pretty(n) {
  return ACTION_LABELS[n] || n.charAt(0).toUpperCase() + n.slice(1);
}

const state = {
  screen: "overview",
  leftLock: true,
  dpiSel: null,
  settings: null,
  buttons: [],
  palettes: { mouse: [], media: [], rates: [125, 500, 1000, 2000, 4000, 8000] },
};

const $ = (id) => document.getElementById(id);
const el = (tag, cls, txt) => {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (txt != null) e.textContent = txt;
  return e;
};

// make a non-native element keyboard-operable like a button
function kbd(node, fn, checked) {
  node.setAttribute("role", "button");
  node.tabIndex = 0;
  if (checked !== undefined) node.setAttribute("aria-pressed", checked ? "true" : "false");
  node.onclick = fn;
  node.onkeydown = (e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); fn(); } };
}

// ---- events ----------------------------------------------------------------

async function wireEvents() {
  // Await every subscription before any command runs, else the worker's first
  // connected/settings events fire before listeners attach and are lost.
  await Promise.all([
    listen("connected", (e) => {
      $("devName").textContent = e.payload.name;
      $("devMeta").textContent = `· ${e.payload.transport} · fw ${e.payload.firmware}`;
    }),
    listen("settings", (e) => {
      state.settings = e.payload;
      paintBattery();
      renderAll();
    }),
    listen("buttons", (e) => {
      state.buttons = e.payload;
      renderButtons();
    }),
    listen("written", (e) => toast(e.payload.ok ? e.payload.msg : friendlyError(e.payload.msg), e.payload.ok ? "ok" : "err")),
    listen("firmware", (e) => {
      const v = e.payload.latest;
      toast(v ? `firmware: latest is ${v}` : "firmware check failed (offline?)", v ? "ok" : "err");
    }),
    listen("error", (e) => {
      $("devName").textContent = "Disconnected";
      $("devMeta").textContent = "";
      $("battCells").innerHTML = "";
      $("battPct").textContent = "";
      toast(friendlyError(e.payload.message), "err");
    }),
  ]);
}

function paintBattery() {
  const b = state.settings?.battery;
  if (!b) { $("battCells").innerHTML = ""; $("battPct").textContent = ""; return; }
  const segs = 10;
  const filled = Math.round((b.percent / 100) * segs);
  const cells = $("battCells");
  cells.innerHTML = "";
  for (let i = 0; i < segs; i++) cells.appendChild(el("span", "cell" + (i < filled ? "" : " off")));
  $("battPct").innerHTML = (b.charging ? BOLT_SVG : "") + `${b.percent}%`;
  $("batt").title = `Battery ${b.percent}%${b.charging ? " (charging)" : ""}`;
}

// ---- shell -----------------------------------------------------------------

function buildNav(host) {
  host.innerHTML = "";
  for (const [id, name, icon] of NAV) {
    const b = document.createElement("button");
    if (id === state.screen) b.setAttribute("aria-current", "page");
    b.innerHTML = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="miter">${icon}</svg>${name}`;
    b.onclick = () => goto(id);
    host.appendChild(b);
  }
}

function goto(id) {
  state.screen = id;
  document.querySelectorAll(".section").forEach((s) => s.classList.toggle("on", s.dataset.sec === id));
  buildNav($("sideNav"));
  buildNav($("sideNavMobile"));
  const nd = $("navDrawer");
  if (nd.classList.contains("drawer-open")) fdDrawer("navDrawer", "navScrim");
  $("main").scrollTop = 0;
}

function renderAll() {
  renderOverview();
  renderDpi();
  renderPoll();
  renderSensor();
  renderButtons();
  renderProfiles();
}

// ---- overview --------------------------------------------------------------

function renderOverview() {
  const s = state.settings;
  if (!s) return;
  $("ovDpi").textContent = s.dpi.presets[s.dpi.active] ?? "—";
  $("ovPoll").textContent = s.pollingHz || "—";
  $("ovLod").textContent = LOD[s.sensor.lod] ?? "?";
  $("ovScroll").textContent = s.sensor.scrollInverted ? "inverted" : "normal";
  $("ovMotion").innerHTML = s.sensor.motion ? '<span class="dot"></span> on' : "off";
  $("ovAngle").textContent = s.sensor.angle === 0 ? "off" : `${s.sensor.angle}°`;
  $("ovDeb").textContent = `${s.debounce} ms`;
  $("ovSleep").textContent = `${s.sleepMin} min`;

  const c = $("ovDpiChips");
  c.innerHTML = "";
  s.dpi.presets.forEach((v, i) => {
    const b = el("button", i === s.dpi.active ? "on" : "", String(v));
    b.onclick = (ev) => { ev.stopPropagation(); if (i !== s.dpi.active) invoke("set_active_dpi", { index: i }); };
    c.appendChild(b);
  });
}

// ---- dpi --------------------------------------------------------------------

function dpiMax() { return state.settings?.dpi.max || 26000; }

function setDpiVal(i, v) {
  const max = dpiMax();
  const val = Math.min(max, Math.max(50, Math.round(v / 50) * 50));
  invoke("set_dpi", { index: i, value: val });
}

function renderDpi() {
  const s = state.settings;
  const list = $("presetList");
  if (!s) { list.innerHTML = ""; list.appendChild(el("p", "note", "reading device…")); return; }
  if (state.dpiSel == null || state.dpiSel >= s.dpi.presets.length) state.dpiSel = s.dpi.active;
  const sel = state.dpiSel;

  $("dpiCount").textContent = s.dpi.count;
  list.innerHTML = "";
  const n = s.dpi.presets.length;
  s.dpi.presets.forEach((v, i) => {
    const pct = n > 1 ? Math.round(25 + i * (75 / (n - 1))) : 75;
    const row = el("div", "preset" + (i === sel ? " active" : ""));
    const sw = el("span", "swatch");
    sw.style.background = `color-mix(in srgb, var(--accent) ${pct}%, var(--muted))`;
    row.append(sw, el("span", null, String(v)));
    if (i === s.dpi.active) row.appendChild(el("span", "active-tag", "active"));
    row.setAttribute("aria-label", `${v} DPI${i === s.dpi.active ? " (active)" : ""}`);
    kbd(row, () => { state.dpiSel = i; if (i !== s.dpi.active) invoke("set_active_dpi", { index: i }); else renderDpi(); }, i === s.dpi.active);
    list.appendChild(row);
  });

  const max = dpiMax();
  const val = s.dpi.presets[sel];
  $("dpiNum").value = val;
  const slider = $("dpiSlider");
  slider.max = max;
  slider.value = val;

  const ticks = $("dpiTicks");
  ticks.innerHTML = "";
  const q = Math.round(max / 4);
  [50, q, q * 2, q * 3, max].forEach((t) => ticks.appendChild(el("span", null, String(t))));
}

function stepDpi(d) {
  const s = state.settings;
  if (!s) return;
  setDpiVal(state.dpiSel, s.dpi.presets[state.dpiSel] + d);
}
function onDpiType(v) {
  const n = parseInt(String(v).replace(/[^0-9]/g, ""), 10) || 0;
  $("dpiSlider").value = Math.min(dpiMax(), Math.max(50, n));
  $("ovDpi").textContent = n;
}
function onSlider(v) {
  $("dpiNum").value = v;
  $("ovDpi").textContent = v;
}
function commitDpi() {
  if (state.settings == null) return;
  const n = parseInt($("dpiNum").value.replace(/[^0-9]/g, ""), 10);
  if (!Number.isNaN(n)) setDpiVal(state.dpiSel, n);
}

// ---- polling ----------------------------------------------------------------

function renderPoll() {
  const s = state.settings;
  const host = $("pollBars");
  host.innerHTML = "";
  if (!s) { host.appendChild(el("p", "note", "reading device…")); return; }
  const rates = state.palettes.rates;
  const active = s.pollingHz;
  const n = rates.length;
  rates.forEach((hz, i) => {
    const on = hz === active;
    const col = el("div", "barcol" + (on ? " on" : ""));
    const bar = el("div", "bar");
    bar.style.height = `${30 + (i / (n - 1)) * 70}%`;
    const label = el("div", "barlabel");
    label.innerHTML = `${hz}Hz${on ? " " + CHECK_SVG : ""}`;
    col.append(bar, label);
    col.setAttribute("aria-label", `${hz}Hz${on ? " (active)" : ""}`);
    kbd(col, () => invoke("set_rate", { hz }), on);
    host.appendChild(col);
  });
}

// ---- sensor -----------------------------------------------------------------

function renderSensor() {
  const host = $("sensorRows");
  host.innerHTML = "";
  if (!state.settings) { host.appendChild(el("p", "note", "reading device…")); return; }
  const s = state.settings.sensor;

  host.appendChild(segRow("Lift-off distance", [["0.7 mm", 3], ["1.0 mm", 1], ["2.0 mm", 2]], s.lod,
    (v) => invoke("set_lod", { value: v })));
  host.appendChild(segRow("Scroll direction", [["normal", false], ["inverted", true]], s.scrollInverted,
    (v) => invoke("set_scroll", { inverted: v })));
  host.appendChild(segRow("Motion sync", [["off", false], ["on", true]], s.motion,
    (v) => invoke("set_motion", { on: v })));
  host.appendChild(segRow("Sampling mode", [["Standard", false], ["Competitive", true]], s.fps20k,
    (v) => invoke("set_fps20k", { on: v })));

  // angle: off/on segment + (when on) a degree input
  const enabled = s.angle !== 0;
  const deg = Math.abs(s.angle) || 5;
  const row = el("div", "srow");
  row.appendChild(el("div", "slabel", "Angle snapping"));
  const right = el("div", "inline-set");
  const seg = el("div", "seg");
  const off = el("button", enabled ? "" : "on", "off");
  const on = el("button", enabled ? "on" : "", "on");
  off.onclick = () => invoke("set_angle", { degrees: 0, enable: false });
  on.onclick = () => invoke("set_angle", { degrees: deg, enable: true });
  seg.append(off, on);
  right.appendChild(seg);
  if (enabled) {
    const input = el("input");
    input.type = "number"; input.min = 1; input.max = 90; input.value = deg;
    input.setAttribute("aria-label", "Angle degrees");
    input.style.maxWidth = "90px";
    const apply = btnSquare("set °", () => {
      const d = Math.min(90, Math.max(1, Math.round(+input.value)));
      invoke("set_angle", { degrees: d, enable: true });
    });
    input.onkeydown = (e) => { if (e.key === "Enter") apply.click(); };
    right.append(input, el("span", null, "°"), apply);
  }
  row.appendChild(right);
  host.appendChild(row);

  host.appendChild(numRow("Debounce (ms)", state.settings.debounce, (v) => invoke("set_debounce", { ms: v })));
  host.appendChild(numRow("Sleep (min)", state.settings.sleepMin, (v) => invoke("set_sleep", { minutes: v })));
}

function segRow(label, opts, current, on) {
  const row = el("div", "srow");
  row.appendChild(el("div", "slabel", label));
  const wrap = el("div");
  const seg = el("div", "seg");
  for (const [name, val] of opts) {
    const b = el("button", val === current ? "on" : "", name);
    b.onclick = () => on(val);
    seg.appendChild(b);
  }
  wrap.appendChild(seg);
  row.appendChild(wrap);
  return row;
}

function numRow(label, value, on) {
  const row = el("div", "srow");
  row.appendChild(el("div", "slabel", label));
  const set = el("div", "inline-set");
  const input = el("input");
  input.type = "number"; input.value = value;
  input.setAttribute("aria-label", label);
  const apply = btnSquare("set", () => on(Math.round(+input.value)));
  input.onkeydown = (e) => { if (e.key === "Enter") apply.click(); };
  set.append(input, apply);
  row.appendChild(set);
  return row;
}

function btnSquare(label, fn) {
  const b = el("button", "btn btn-primary btn-square");
  b.appendChild(el("span", null, label));
  b.onclick = fn;
  return b;
}

// ---- buttons ----------------------------------------------------------------

function renderButtons() {
  const lock = $("lockToggle");
  lock.classList.toggle("on", state.leftLock);
  lock.setAttribute("aria-checked", state.leftLock ? "true" : "false");

  const host = $("btnRows");
  host.innerHTML = "";
  if (!state.buttons.length) {
    const tr = el("tr");
    const td = el("td", "note", "loading buttons…");
    td.colSpan = 5;
    tr.appendChild(td);
    host.appendChild(tr);
    return;
  }
  for (const b of state.buttons) {
    const locked = b.id === 0 && state.leftLock; // left button protected
    const tr = el("tr", (b.present ? "" : "empty") + (locked ? " locked" : ""));
    const badge = el("span", "badge " + (b.typeId === 3 ? "red" : "out"), b.typeName);
    const badgeTd = el("td"); badgeTd.appendChild(badge);
    const go = el("td"); go.style.textAlign = "center";
    if (locked) { const s = el("span", "lk"); s.innerHTML = LOCK_SVG; go.appendChild(s); }
    else if (b.present) go.appendChild(el("span", "go", "›"));
    tr.append(
      el("td", null, String(b.id)),
      el("td", null, b.friendly || ""),
      badgeTd,
      el("td", null, b.label),
      go,
    );
    if (b.present && !locked) {
      tr.setAttribute("aria-label", `Remap ${b.friendly || "button " + b.id}`);
      kbd(tr, () => openRemap(b));
    } else if (locked) {
      tr.title = "left click lock is on";
    }
    host.appendChild(tr);
  }
}

function toggleLock() {
  state.leftLock = !state.leftLock;
  renderButtons();
}

// ---- profiles ---------------------------------------------------------------

function renderProfiles() {
  const s = state.settings;
  const host = $("profileList");
  host.innerHTML = "";
  if (!s) { host.appendChild(el("p", "note", "reading device…")); return; }
  for (let i = 0; i < s.profile.count; i++) {
    const on = i === s.profile.current;
    const row = el("div", "list-row link card" + (on ? " sel" : ""));
    row.setAttribute("role", "option");
    row.setAttribute("aria-selected", on ? "true" : "false");
    row.tabIndex = 0;
    if (!on) row.style.boxShadow = "4px 4px 0 var(--shadow)";
    row.appendChild(el("span", on ? "dot" : "dot dead"));
    row.appendChild(el("span", null, `Profile ${i + 1}`));
    if (on) {
      const tag = el("span", null, "active");
      tag.style.cssText = "margin-left:8px;font-size:11px;letter-spacing:1px;text-transform:uppercase;color:var(--accent);font-weight:900";
      row.appendChild(tag);
    }
    const pick = () => invoke("set_profile", { index: i });
    row.onclick = pick;
    row.onkeydown = (e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); pick(); } };
    host.appendChild(row);
  }
}

// ---- button remap modal -----------------------------------------------------

let remapBtn = null;
let remapKind = "mouse";
let remapAction = null;

function openRemap(b) {
  remapBtn = b;
  remapKind = "mouse";
  remapAction = null;
  $("remapTitle").textContent = `Assign button ${b.id}${b.friendly ? " · " + b.friendly : ""}`;
  pickType("mouse");
  $("remapModal").showModal();
}

function pickType(kind) {
  remapKind = kind;
  remapAction = null;
  document.querySelectorAll("#remapType button").forEach((b) => b.classList.toggle("on", b.dataset.t === kind));
  const isList = kind === "mouse" || kind === "media";
  $("remapAssignLabel").textContent = isList ? "Action" : kind === "disable" ? "Disabled" : "Default";
  const opts = $("remapOpts");
  opts.className = isList ? "opts" : "";
  opts.innerHTML = "";
  if (isList) {
    for (const name of state.palettes[kind]) {
      const o = el("div", "opt", pretty(name));
      kbd(o, () => { remapAction = name; opts.querySelectorAll(".opt").forEach((x) => x.classList.toggle("sel", x === o)); });
      opts.appendChild(o);
    }
  } else {
    opts.appendChild(el("div", "pick-hint",
      kind === "disable" ? "This button will do nothing." : "Restore the button's hardware default function."));
  }
}

function closeRemap() { $("remapModal").close(); }

function saveRemap() {
  const b = remapBtn;
  if (!b) return;
  if (remapKind === "mouse" && remapAction) invoke("set_button_mouse", { id: b.id, action: remapAction });
  else if (remapKind === "media" && remapAction) invoke("set_button_media", { id: b.id, action: remapAction });
  else if (remapKind === "disable") invoke("set_button_disable", { id: b.id });
  else if (remapKind === "default") invoke("set_button_default", { id: b.id });
  else return; // mouse/media chosen but no action picked yet
  closeRemap();
}

// ---- theme + shortcuts ------------------------------------------------------

function applySavedTheme() {
  const t = localStorage.getItem("sq-theme") || "dark";
  document.documentElement.setAttribute("data-theme", t);
  syncThemeSwitch();
}
function syncThemeSwitch() {
  const dark = document.documentElement.getAttribute("data-theme") === "dark";
  $("thDark").classList.toggle("act", dark);
  $("thLight").classList.toggle("act", !dark);
}
function toggleTheme() {
  fdTheme();
  localStorage.setItem("sq-theme", document.documentElement.getAttribute("data-theme"));
  syncThemeSwitch();
}

function doRefresh() { invoke("read_all"); invoke("read_buttons"); }
function checkFw() { invoke("check_update"); }
function openHelp() { $("helpModal").showModal(); }
function quit() { window.__TAURI__.window.getCurrentWindow().close(); }

function onKey(e) {
  if (e.target.tagName === "INPUT" || e.target.tagName === "TEXTAREA") return;
  if (document.querySelector("dialog[open]")) return; // dialog owns the keyboard (Esc closes natively)
  const i = NAV.findIndex((x) => x[0] === state.screen);
  switch (e.key) {
    case "ArrowDown": case "j": goto(NAV[(i + 1) % NAV.length][0]); break;
    case "ArrowUp": case "k": goto(NAV[(i - 1 + NAV.length) % NAV.length][0]); break;
    case "r": doRefresh(); break;
    case "u": checkFw(); break;
    case "?": openHelp(); break;
    case "q": quit(); break;
    default: return;
  }
  e.preventDefault();
}

// ---- toast ------------------------------------------------------------------

// Map a raw backend error to a plain, actionable reason.
function friendlyError(raw) {
  const m = String(raw).toLowerCase();
  if (m.includes("no such device") || m.includes("os error 19")) return "Mouse disconnected.";
  if (m.includes("not found") || m.includes("no responding"))
    return "No mouse found — connect the cable or dongle (and unplug the unused one).";
  if (m.includes("permission") || m.includes("eacces") || m.includes("os error 13"))
    return "Permission denied opening the device — check the udev rule (see README).";
  if (m.includes("timeout")) return "Mouse not responding — reconnect it, then refresh.";
  if (m.includes("unconfirmed")) return "The device didn't confirm the change — try again.";
  if (m.includes("rejected")) return "The device rejected that value.";
  return String(raw).split("\n")[0]; // unknown: first line
}

function toast(msg, kind) {
  fdToast(msg, kind === "err" ? { type: "err" } : {});
}

// ---- boot -------------------------------------------------------------------

async function boot() {
  applySavedTheme();
  fdAccent("accpick", ACCENTS);
  await wireEvents();
  buildNav($("sideNav"));
  buildNav($("sideNavMobile"));
  document.querySelectorAll("[data-goto]").forEach((c) => { c.onclick = () => goto(c.dataset.goto); });
  document.addEventListener("keydown", onKey);
  renderAll();
  state.palettes = await invoke("palettes");
  await invoke("read_all");
  await invoke("read_buttons");
}
boot();
