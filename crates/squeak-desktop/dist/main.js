const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const LOD = ["0.7 mm", "1.0 mm", "2.0 mm"];
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

const state = {
  screen: "overview",
  theme: 0,
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
      toast(e.payload.message, "err");
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

function cycleTheme() {
  state.theme = (state.theme + 1) % THEMES.length;
  applyTheme();
  toast(`theme: ${THEMES[state.theme].name}`, "ok");
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
    case "t": cycleTheme(); break;
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

function dpi(m) {
  const s = state.settings;
  m.appendChild(el("p", "sub", "Activate a preset to use it now; edit a value + Set to change it."));
  s.dpi.presets.forEach((v, i) => {
    const active = i === s.dpi.active;
    const row = el("div", "row");

    const use = el("button", "use" + (active ? " on" : ""), active ? "● active" : "activate");
    use.title = "make this the active DPI stage";
    if (!active) use.onclick = () => invoke("set_active_dpi", { index: i });

    const input = el("input");
    input.type = "number"; input.min = 50; input.max = s.dpi.max || 26000; input.step = 50; input.value = v;
    const apply = el("button", "btn primary", "set");
    apply.onclick = () => invoke("set_dpi", { index: i, value: Math.round(+input.value) });

    row.append(use, el("span", "label", `Preset ${i + 1}`), input, apply);
    m.appendChild(row);
  });
}

function polling(m) {
  const s = state.settings;
  const row = el("div", "row");
  row.appendChild(el("span", "label", "Polling rate"));
  const seg = el("div", "seg");
  state.palettes.rates.forEach((hz) => {
    const b = el("button", hz === s.pollingHz ? "on" : "", String(hz));
    b.onclick = () => invoke("set_rate", { hz });
    seg.appendChild(b);
  });
  row.appendChild(seg);
  m.appendChild(row);
}

function sensor(m) {
  const s = state.settings.sensor;
  m.appendChild(segRow("Lift-off distance", [["0.7 mm", 0], ["1.0 mm", 1], ["2.0 mm", 2]], s.lod,
    (v) => invoke("set_lod", { value: v })));
  m.appendChild(segRow("Scroll direction", [["normal", false], ["inverted", true]], s.scrollInverted,
    (v) => invoke("set_scroll", { inverted: v })));
  m.appendChild(segRow("Motion sync", [["off", false], ["on", true]], s.motion,
    (v) => invoke("set_motion", { on: v })));
  m.appendChild(segRow("Sampling mode", [["Standard", false], ["Competitive", true]], s.fps20k,
    (v) => invoke("set_fps20k", { on: v })));

  // angle: enable toggle + degree slider
  const arow = el("div", "row");
  arow.appendChild(el("span", "label", "Angle snapping"));
  const seg = el("div", "seg");
  const off = el("button", s.angle === 0 ? "on" : "", "off");
  const on = el("button", s.angle !== 0 ? "on" : "", "on");
  off.onclick = () => invoke("set_angle", { degrees: 0, enable: false });
  on.onclick = () => invoke("set_angle", { degrees: Math.abs(s.angle) || 5, enable: true });
  seg.append(off, on);
  arow.appendChild(seg);
  if (s.angle !== 0) arow.appendChild(el("span", "v", `${s.angle}°`));
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
  if (!state.buttons.length) { m.appendChild(el("p", "sub", "loading buttons…")); return; }
  const table = el("table");
  const head = el("tr");
  ["id", "button", "type", "assignment", ""].forEach((h) => head.appendChild(el("th", null, h)));
  table.appendChild(head);
  for (const b of state.buttons) {
    const tr = el("tr", "btn-row" + (b.present ? "" : " empty"));
    tr.append(
      el("td", null, String(b.id)),
      el("td", null, b.friendly || ""),
      tagCell(b.typeName, b.typeId),
      el("td", null, b.label),
      el("td", null, b.present ? "›" : ""),
    );
    if (b.present) tr.onclick = () => openPicker(b);
    table.appendChild(tr);
  }
  m.appendChild(table);
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

function openPicker(b) {
  let kind = "mouse";
  let action = null;
  const scrim = el("div", "scrim");
  const modal = el("div", "modal");
  modal.appendChild(el("h3", null, `Assign button ${b.id}${b.friendly ? " · " + b.friendly : ""}`));

  const kindSeg = el("div", "seg");
  const opts = el("div", "opts");
  const renderOpts = () => {
    opts.innerHTML = "";
    if (kind === "mouse" || kind === "media") {
      for (const name of state.palettes[kind]) {
        const o = el("div", "opt" + (name === action ? " sel" : ""), name);
        o.onclick = () => { action = name; renderOpts(); };
        opts.appendChild(o);
      }
    } else {
      opts.appendChild(el("p", "sub", kind === "disable" ? "Button does nothing." : "Restore hardware default."));
    }
  };
  for (const k of ["mouse", "media", "disable", "default"]) {
    const kb = el("button", k === kind ? "on" : "", k);
    kb.onclick = () => { kind = k; action = null; [...kindSeg.children].forEach((c) => c.classList.toggle("on", c.textContent === k)); renderOpts(); };
    kindSeg.appendChild(kb);
  }
  modal.appendChild(kindSeg);
  modal.appendChild(opts);
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
