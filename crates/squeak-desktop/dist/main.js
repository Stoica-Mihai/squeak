const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const LOD = { 1: "1.0 mm", 2: "2.0 mm", 3: "0.7 mm" }; // device codes (0 is not settable)
const SCREENS = [
  ["overview", "▤", "Overview"],
  ["dpi", "⊙", "DPI"],
  ["polling", "⟳", "Polling"],
  ["sensor", "◎", "Sensor"],
  ["buttons", "⊞", "Buttons"],
  ["profiles", "❏", "Profiles"],
];

const KEYS = [
  ["↑↓", "section"],
  ["r", "refresh"],
  ["t", "theme"],
  ["u", "check fw"],
  ["?", "help"],
  ["q", "quit"],
];

// Palettes ported from the TUI (theme.rs). Cycled with `t`.
const THEMES = [
  { name: "Mocha", bg: "#1e1e2e", surface: "#11111b", card: "#24273a", cardhi: "#2a2d44", line: "#313244", text: "#cdd6f4", sub: "#a6adc8", dim: "#6c7086", accent: "#89b4fa", mauve: "#cba6f7", green: "#a6e3a1", red: "#f38ba8", peach: "#fab387" },
  { name: "Gruvbox", bg: "#282828", surface: "#1d2021", card: "#32302f", cardhi: "#3c3836", line: "#504945", text: "#ebdbb2", sub: "#bdae93", dim: "#928374", accent: "#fabd2f", mauve: "#d3869b", green: "#b8bb26", red: "#fb4934", peach: "#fe8019" },
  { name: "Nord", bg: "#2e3440", surface: "#272c36", card: "#3b4252", cardhi: "#434c5e", line: "#434c5e", text: "#d8dee9", sub: "#aeb8c9", dim: "#4c566a", accent: "#88c0d0", mauve: "#b48ead", green: "#a3be8c", red: "#bf616a", peach: "#d08770" },
  { name: "Dracula", bg: "#282a36", surface: "#21222c", card: "#343746", cardhi: "#44475a", line: "#44475a", text: "#f8f8f2", sub: "#c8c9d6", dim: "#6272a4", accent: "#bd93f9", mauve: "#ff79c6", green: "#50fa7b", red: "#ff5555", peach: "#ffb86c" },
];

const DPI_DOTS = ["#cdd6f4", "#a6e3a1", "#89b4fa", "#fab387", "#f38ba8"];

const state = {
  screen: "overview",
  theme: 0,
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

// ---- events ----------------------------------------------------------------

async function wireEvents() {
  // Await every subscription before any command runs, else the worker's first
  // connected/settings events fire before listeners attach and are lost.
  await Promise.all([
    listen("connected", (e) => {
      $("device").textContent = e.payload.name;
      $("meta").textContent = `· ${e.payload.transport} · fw ${e.payload.firmware}`;
    }),
    listen("settings", (e) => {
      state.settings = e.payload;
      paintStatus();
      render();
    }),
    listen("buttons", (e) => {
      state.buttons = e.payload;
      if (state.screen === "buttons") render();
    }),
    listen("written", (e) => toast(e.payload.msg, e.payload.ok ? "ok" : "err")),
    listen("firmware", (e) => {
      const v = e.payload.latest;
      toast(v ? `firmware: latest is ${v}` : "firmware check failed (offline?)", v ? "ok" : "err");
    }),
    listen("error", (e) => {
      $("device").textContent = "disconnected";
      toast(String(e.payload.message).split("\n")[0], "err"); // first line only
    }),
  ]);
}

// ---- keyboard + footer keybar ----------------------------------------------

function buildKeys() {
  const f = $("keys");
  f.innerHTML = "";
  for (const [k, l] of KEYS) {
    const g = el("span", "grp");
    g.append(el("span", "k", k), el("span", "kg", l));
    f.appendChild(g);
  }
}

function moveSection(d) {
  const i = SCREENS.findIndex((x) => x[0] === state.screen);
  const n = (i + d + SCREENS.length) % SCREENS.length;
  state.screen = SCREENS[n][0];
  buildRail();
  render();
}

function applyTheme() {
  const t = THEMES[state.theme];
  const root = document.documentElement.style;
  for (const [k, v] of Object.entries(t)) {
    if (k !== "name") root.setProperty(`--${k}`, v);
  }
}

// Theme picker: live-preview on hover/arrows, swatches, ↵ confirm / esc revert.
function openThemePicker() {
  if (document.querySelector(".scrim")) return;
  const prev = state.theme;
  let cur = state.theme;
  const scrim = el("div", "scrim");
  const modal = el("div", "modal");
  modal.appendChild(el("h3", null, "Theme"));

  const rows = [];
  const mark = () => rows.forEach((r, i) => r.classList.toggle("sel", i === cur));
  const preview = (i) => { cur = i; state.theme = i; applyTheme(); mark(); };
  THEMES.forEach((t, i) => {
    const row = el("div", "theme-opt");
    const sw = el("span", "swatches");
    for (const k of ["bg", "surface", "accent", "green", "peach", "red", "mauve"]) {
      const d = el("span", "sw");
      d.style.background = t[k];
      sw.appendChild(d);
    }
    row.append(el("span", "tname", t.name), sw);
    row.onmouseenter = () => preview(i);
    row.onclick = () => finish(true);
    rows.push(row);
    modal.appendChild(row);
  });
  modal.appendChild(el("p", "sub", "↑↓ preview · ↵ apply · esc cancel"));
  scrim.appendChild(modal);
  document.body.appendChild(scrim);
  mark();

  function finish(ok) {
    if (!ok) { state.theme = prev; applyTheme(); }
    document.removeEventListener("keydown", onPickKey, true);
    scrim.remove();
  }
  function onPickKey(e) {
    e.stopPropagation();
    if (e.key === "Escape") finish(false);
    else if (e.key === "Enter") finish(true);
    else if (e.key === "ArrowDown" || e.key === "j") { preview((cur + 1) % THEMES.length); e.preventDefault(); }
    else if (e.key === "ArrowUp" || e.key === "k") { preview((cur - 1 + THEMES.length) % THEMES.length); e.preventDefault(); }
  }
  document.addEventListener("keydown", onPickKey, true);
  scrim.onclick = (e) => { if (e.target === scrim) finish(false); };
}

function quit() {
  window.__TAURI__.window.getCurrentWindow().close();
}

function openHelp() {
  if (document.querySelector(".scrim")) return;
  const rows = [
    ["↑ ↓", "switch section"],
    ["r", "refresh from device"],
    ["t", "cycle theme"],
    ["u", "check firmware version (online)"],
    ["?", "this help"],
    ["q", "quit"],
    ["Esc", "close dialog"],
  ];
  const scrim = el("div", "scrim");
  const modal = el("div", "modal");
  modal.appendChild(el("h3", null, "Keyboard shortcuts"));
  for (const [k, d] of rows) {
    const r = el("div", "help-row");
    r.append(el("span", "hk", k), el("span", "hd", d));
    modal.appendChild(r);
  }
  scrim.appendChild(modal);
  scrim.onclick = (e) => { if (e.target === scrim) scrim.remove(); };
  document.body.appendChild(scrim);
}

function onKey(e) {
  if (e.target.tagName === "INPUT") return; // don't hijack typing in fields
  const scrim = document.querySelector(".scrim");
  if (e.key === "Escape") { if (scrim) scrim.remove(); return; }
  if (scrim) return; // modal owns the keyboard
  switch (e.key) {
    case "ArrowDown": case "j": moveSection(1); break;
    case "ArrowUp": case "k": moveSection(-1); break;
    case "r": invoke("read_all"); invoke("read_buttons"); break;
    case "t": openThemePicker(); break;
    case "u": invoke("check_update"); break;
    case "?": openHelp(); break;
    case "q": quit(); break;
    default: return;
  }
  e.preventDefault();
}

function paintStatus() {
  const b = state.settings?.battery;
  if (!b) return;
  const segs = 10;
  const filled = Math.round((b.percent / 100) * segs);
  $("batt-gauge").innerHTML =
    `<span class="batt-on">${"▰".repeat(filled)}</span>` +
    `<span class="batt-off">${"▰".repeat(segs - filled)}</span>`;
  $("batt").textContent = `${b.percent}%${b.charging ? " ⚡" : ""}`;
}

// ---- shell -----------------------------------------------------------------

function buildRail() {
  const rail = $("rail");
  rail.innerHTML = "";
  for (const [id, ic, name] of SCREENS) {
    const n = el("div", "nav" + (id === state.screen ? " active" : ""));
    n.append(el("span", "ic", ic), el("span", null, name));
    n.onclick = () => { state.screen = id; buildRail(); render(); };
    rail.appendChild(n);
  }
}

function render() {
  const s = state.settings;
  const m = $("screen");
  m.innerHTML = "";
  m.appendChild(el("h1", "screen-title", SCREENS.find((x) => x[0] === state.screen)[2]));
  if (!s && state.screen !== "buttons") {
    m.appendChild(el("p", "sub", "reading device…"));
    return;
  }
  ({ overview, dpi, polling, sensor, buttons, profiles })[state.screen](m);
}

// ---- screens ---------------------------------------------------------------

function card(title, ic) {
  const c = el("section", "card");
  const t = el("div", "t");
  t.append(el("span", "ic", ic), document.createTextNode(" " + title));
  c.appendChild(t);
  return c;
}

function goto(screen) {
  state.screen = screen;
  buildRail();
  render();
}

function overview(m) {
  const s = state.settings;
  const grid = el("div", "grid");

  const d = card("DPI", "⊙");
  const big = el("div", "big");
  big.append(el("span", null, String(s.dpi.presets[s.dpi.active] ?? "—")), el("span", "unit", " active"));
  d.appendChild(big);
  const pills = el("div", "pills");
  s.dpi.presets.forEach((v, i) => pills.appendChild(el("span", "pill" + (i === s.dpi.active ? " on" : ""), v)));
  d.appendChild(pills);

  const p = card("Polling rate", "⟳");
  const pb = el("div", "big");
  pb.append(el("span", null, String(s.pollingHz || "—")), el("span", "unit", " Hz"));
  p.append(pb, el("div", "sub", "125 · 500 · 1000 · 2000 · 4000 · 8000"));

  const se = card("Sensor", "◎");
  se.append(
    kvLine("LOD", LOD[s.sensor.lod] ?? "?"),
    kvLine("scroll", s.sensor.scrollInverted ? "inverted" : "normal"),
    kvLine("motion", s.sensor.motion ? "● on" : "○ off", s.sensor.motion ? "on" : "off"),
    kvLine("angle", s.sensor.angle === 0 ? "off" : `${s.sensor.angle}°`),
  );

  const t = card("Timing & power", "⌁");
  t.append(kvLine("debounce", `${s.debounce} ms`), kvLine("sleep", `${s.sleepMin} min`));

  // Overview cards are shortcuts to their editable screen.
  for (const [c, screen] of [[d, "dpi"], [p, "polling"], [se, "sensor"], [t, "sensor"]]) {
    c.classList.add("link");
    c.onclick = () => goto(screen);
  }

  grid.append(d, p, se, t);
  m.appendChild(grid);
}

function kvLine(k, v, cls) {
  const r = el("div", "kv");
  r.append(el("span", "k", k), el("span", "v" + (cls ? " " + cls : ""), v));
  return r;
}

function setDpiVal(i, v) {
  const max = state.settings.dpi.max || 26000;
  const val = Math.min(max, Math.max(50, Math.round(v / 50) * 50));
  invoke("set_dpi", { index: i, value: val });
}

function dpi(m) {
  const s = state.settings;
  const max = s.dpi.max || 26000;
  if (state.dpiSel == null || state.dpiSel >= s.dpi.presets.length) state.dpiSel = s.dpi.active;
  const sel = state.dpiSel;
  const val = s.dpi.presets[sel];

  const layout = el("div", "dpi-layout");

  // left: preset list (click = activate + select for editing)
  const list = el("div", "dpi-list");
  list.appendChild(el("div", "dpi-levels", `Levels: ${s.dpi.count}`));
  s.dpi.presets.forEach((v, i) => {
    const row = el("div", "dpi-row" + (i === sel ? " sel" : "") + (i === s.dpi.active ? " active" : ""));
    const dot = el("span", "dpi-dot");
    dot.style.background = DPI_DOTS[i % DPI_DOTS.length];
    row.append(dot, el("span", "dpi-val", v));
    if (i === s.dpi.active) row.append(el("span", "dpi-tag", "active"));
    row.onclick = () => { state.dpiSel = i; if (i !== s.dpi.active) invoke("set_active_dpi", { index: i }); render(); };
    list.appendChild(row);
  });

  // right: value stepper + gradient slider
  const ed = el("div", "dpi-editor");
  ed.append(
    el("h2", "ed-title", "DPI Settings"),
    el("p", "sub", "DPI sets cursor sensitivity — higher moves the cursor farther for the same hand movement."),
  );
  const stepper = el("div", "dpi-stepper");
  const minus = el("button", "step", "−");
  const num = el("input", "dpi-num");
  num.type = "text";
  num.inputMode = "numeric";
  num.value = String(val);
  num.title = "click to type a value";
  const plus = el("button", "step", "+");
  minus.onclick = () => setDpiVal(sel, val - 50);
  plus.onclick = () => setDpiVal(sel, val + 50);
  const commit = () => { const n = parseInt(num.value, 10); if (!Number.isNaN(n)) setDpiVal(sel, n); };
  num.onchange = commit;
  num.onkeydown = (e) => { if (e.key === "Enter") { e.preventDefault(); num.blur(); } };
  num.onfocus = () => num.select();
  stepper.append(minus, num, plus);

  const slider = el("input");
  slider.type = "range"; slider.min = 50; slider.max = max; slider.step = 50; slider.value = val;
  slider.className = "dpi-slider";
  slider.oninput = () => { num.value = slider.value; };
  slider.onchange = () => setDpiVal(sel, +slider.value);

  const ticks = el("div", "dpi-ticks");
  // Position each tick at the thumb's true travel (track inset by half the
  // 10px thumb), centered — so labels line up with the handle.
  const tickVals = [50, 6500, 13000, 19500, max];
  tickVals.forEach((t, i) => {
    const sp = el("span", null, String(t));
    const frac = (t - 50) / (max - 50);
    sp.style.left = `calc(5px + ${frac} * (100% - 10px))`;
    sp.style.transform = i === 0 ? "translateX(0)" : i === tickVals.length - 1 ? "translateX(-100%)" : "translateX(-50%)";
    ticks.appendChild(sp);
  });

  ed.append(stepper, slider, ticks);
  layout.append(list, ed);
  m.appendChild(layout);
}

function polling(m) {
  const s = state.settings;
  const rates = state.palettes.rates;
  const active = s.pollingHz;
  const n = rates.length;

  const chart = el("div", "poll-chart");
  rates.forEach((hz, i) => {
    const on = hz === active;
    const col = el("div", "poll-col" + (on ? " active" : ""));
    const bar = el("div", "poll-bar");
    bar.style.height = `${22 + (i / (n - 1)) * 78}%`;
    if (on) {
      bar.style.background = "linear-gradient(180deg, var(--accent), #74a0f0)";
    } else {
      // Office (blue) → Gaming (red) ramp via purple, muted for inactive bars.
      const hue = 220 + (i / (n - 1)) * 140; // 220→360 (blue→purple→red)
      bar.style.background = `linear-gradient(180deg, hsl(${hue} 40% 36%), hsl(${hue} 40% 26%))`;
    }
    col.append(bar, el("div", "poll-hz", `${hz}Hz${on ? " ✓" : ""}`));
    col.onclick = () => invoke("set_rate", { hz });
    chart.appendChild(col);
  });
  m.appendChild(chart);

  const ends = el("div", "poll-ends");
  ends.append(el("span", null, "Office"), el("span", null, "Gaming"));
  m.appendChild(ends);
}

function sensor(m) {
  const s = state.settings.sensor;
  m.appendChild(segRow("Lift-off distance", [["0.7 mm", 3], ["1.0 mm", 1], ["2.0 mm", 2]], s.lod,
    (v) => invoke("set_lod", { value: v })));
  m.appendChild(segRow("Scroll direction", [["normal", false], ["inverted", true]], s.scrollInverted,
    (v) => invoke("set_scroll", { inverted: v })));
  m.appendChild(segRow("Motion sync", [["off", false], ["on", true]], s.motion,
    (v) => invoke("set_motion", { on: v })));
  m.appendChild(segRow("Sampling mode", [["Standard", false], ["Competitive", true]], s.fps20k,
    (v) => invoke("set_fps20k", { on: v })));

  // angle: enable toggle + (when on) a degree input
  const enabled = s.angle !== 0;
  const deg = Math.abs(s.angle) || 5;
  const arow = el("div", "row");
  arow.appendChild(el("span", "label", "Angle snapping"));
  const seg = el("div", "seg");
  const off = el("button", enabled ? "" : "on", "off");
  const on = el("button", enabled ? "on" : "", "on");
  off.onclick = () => invoke("set_angle", { degrees: 0, enable: false });
  on.onclick = () => invoke("set_angle", { degrees: deg, enable: true });
  seg.append(off, on);
  arow.appendChild(seg);
  if (enabled) {
    const input = el("input");
    input.type = "number"; input.min = 1; input.max = 90; input.value = deg;
    input.style.width = "72px";
    const apply = el("button", "btn primary", "set °");
    const commit = () => {
      const d = Math.min(90, Math.max(1, Math.round(+input.value)));
      invoke("set_angle", { degrees: d, enable: true });
    };
    apply.onclick = commit;
    input.onkeydown = (e) => { if (e.key === "Enter") commit(); };
    arow.append(input, el("span", "v", "°"), apply);
  }
  m.appendChild(arow);

  m.appendChild(numRow("Debounce (ms)", state.settings.debounce, 0, 30,
    (v) => invoke("set_debounce", { ms: v })));
  m.appendChild(numRow("Sleep (min)", state.settings.sleepMin, 1, 240,
    (v) => invoke("set_sleep", { minutes: v })));
}

function segRow(label, opts, current, on) {
  const row = el("div", "row");
  row.appendChild(el("span", "label", label));
  const seg = el("div", "seg");
  for (const [name, val] of opts) {
    const b = el("button", val === current ? "on" : "", name);
    b.onclick = () => on(val);
    seg.appendChild(b);
  }
  row.appendChild(seg);
  return row;
}

function numRow(label, value, min, max, on) {
  const row = el("div", "row");
  row.appendChild(el("span", "label", label));
  const input = el("input");
  input.type = "number"; input.min = min; input.max = max; input.value = value;
  const apply = el("button", "btn primary", "set");
  apply.onclick = () => on(Math.round(+input.value));
  row.append(input, apply);
  return row;
}

function buttons(m) {
  m.appendChild(lockBar());
  if (!state.buttons.length) { m.appendChild(el("p", "sub", "loading buttons…")); return; }
  const table = el("table");
  const head = el("tr");
  ["id", "button", "type", "assignment", ""].forEach((h) => head.appendChild(el("th", null, h)));
  table.appendChild(head);
  for (const b of state.buttons) {
    const locked = b.id === 0 && state.leftLock; // left button protected
    const tr = el("tr", "btn-row" + (b.present ? "" : " empty") + (locked ? " locked" : ""));
    tr.append(
      el("td", null, String(b.id)),
      el("td", null, b.friendly || ""),
      tagCell(b.typeName, b.typeId),
      el("td", null, b.label),
      el("td", null, locked ? "🔒" : b.present ? "›" : ""),
    );
    if (b.present && !locked) tr.onclick = () => openPicker(b);
    table.appendChild(tr);
  }
  m.appendChild(table);
}

// UI-side guard (matches the Launcher): while on, the left button can't be remapped.
function lockBar() {
  const bar = el("div", "lock-bar");
  const txt = el("div");
  txt.append(el("div", "lock-title", "Left Click Lock"),
    el("div", "sub", "While on, the left button cannot be remapped."));
  const sw = el("label", "switch");
  const u = el("span", "sw-lbl" + (state.leftLock ? "" : " on"), "Unlock");
  const track = el("span", "track" + (state.leftLock ? " on" : ""));
  track.appendChild(el("span", "knob"));
  const l = el("span", "sw-lbl" + (state.leftLock ? " on" : ""), "Lock");
  sw.append(u, track, l);
  sw.onclick = () => { state.leftLock = !state.leftLock; render(); };
  bar.append(txt, sw);
  return bar;
}

function tagCell(name, typeId) {
  const td = el("td");
  td.appendChild(el("span", "tag" + (typeId === 3 ? " media" : ""), name));
  return td;
}

function profiles(m) {
  const s = state.settings;
  const list = el("div", "plist");
  for (let i = 0; i < s.profile.count; i++) {
    const active = i === s.profile.current;
    const p = el("div", "prof" + (active ? " active" : ""));
    p.append(el("span", "dot", active ? "●" : "○"), el("span", null, `Profile ${i + 1}`));
    if (active) p.appendChild(el("span", "sub", "  active"));
    p.onclick = () => invoke("set_profile", { index: i });
    list.appendChild(p);
  }
  m.appendChild(list);
  m.appendChild(el("p", "sub", "switching reloads the whole config from the new profile"));
}

// ---- button picker modal ---------------------------------------------------

const ACTION_LABELS = {
  leftDouble: "Double-click",
  upScroll: "Scroll ↑", downScroll: "Scroll ↓",
  leftScroll: "Scroll ←", rightScroll: "Scroll →",
};
function pretty(n) {
  return ACTION_LABELS[n] || n.charAt(0).toUpperCase() + n.slice(1);
}

function openPicker(b) {
  let kind = "mouse";
  let action = null;
  const scrim = el("div", "scrim");
  const modal = el("div", "modal picker");
  modal.appendChild(el("h3", null, `Assign button ${b.id}${b.friendly ? " · " + b.friendly : ""}`));

  modal.appendChild(el("div", "picker-label", "Type"));
  const kindSeg = el("div", "seg kind");
  const actionLabel = el("div", "picker-label", "Action");
  const opts = el("div", "opts");
  const renderOpts = () => {
    const list = kind === "mouse" || kind === "media";
    actionLabel.textContent = list ? "Action" : kind === "disable" ? "Disabled" : "Default";
    opts.className = list ? "opts" : "opts hint";
    opts.innerHTML = "";
    if (list) {
      for (const name of state.palettes[kind]) {
        const o = el("div", "opt" + (name === action ? " sel" : ""), pretty(name));
        o.onclick = () => { action = name; renderOpts(); };
        opts.appendChild(o);
      }
    } else {
      opts.appendChild(el("div", "pick-hint",
        kind === "disable" ? "This button will do nothing." : "Restore the button's hardware default function."));
    }
  };
  for (const k of ["mouse", "media", "disable", "default"]) {
    const kb = el("button", k === kind ? "on" : "", pretty(k));
    kb.onclick = () => { kind = k; action = null; [...kindSeg.children].forEach((c) => c.classList.toggle("on", c === kb)); renderOpts(); };
    kindSeg.appendChild(kb);
  }
  modal.append(kindSeg, actionLabel, opts);
  renderOpts();

  const actions = el("div", "actions");
  const cancel = el("button", "btn", "cancel");
  cancel.onclick = () => scrim.remove();
  const apply = el("button", "btn primary", "assign");
  apply.onclick = () => {
    if (kind === "mouse" && action) invoke("set_button_mouse", { id: b.id, action });
    else if (kind === "media" && action) invoke("set_button_media", { id: b.id, action });
    else if (kind === "disable") invoke("set_button_disable", { id: b.id });
    else if (kind === "default") invoke("set_button_default", { id: b.id });
    else return;
    scrim.remove();
  };
  actions.append(cancel, apply);
  modal.appendChild(actions);
  scrim.appendChild(modal);
  scrim.onclick = (e) => { if (e.target === scrim) scrim.remove(); };
  document.body.appendChild(scrim);
}

// ---- toast -----------------------------------------------------------------

function toast(msg, kind) {
  const t = el("div", "t " + (kind || ""), msg);
  $("toast").appendChild(t);
  setTimeout(() => t.remove(), 3200);
}

// ---- boot ------------------------------------------------------------------

async function boot() {
  await wireEvents();
  buildRail();
  buildKeys();
  render();
  document.addEventListener("keydown", onKey);
  state.palettes = await invoke("palettes");
  await invoke("read_all");
  await invoke("read_buttons");
}
boot();
