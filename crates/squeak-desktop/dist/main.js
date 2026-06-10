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

const state = {
  screen: "overview",
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

function wireEvents() {
  listen("connected", (e) => {
    $("device").textContent = e.payload.name;
    $("meta").textContent = `· ${e.payload.transport} · fw ${e.payload.firmware}`;
  });
  listen("settings", (e) => {
    state.settings = e.payload;
    paintStatus();
    render();
  });
  listen("buttons", (e) => {
    state.buttons = e.payload;
    if (state.screen === "buttons") render();
  });
  listen("written", (e) => toast(e.payload.msg, e.payload.ok ? "ok" : "err"));
  listen("error", (e) => {
    $("device").textContent = "disconnected";
    toast(e.payload.message, "err");
  });
}

function paintStatus() {
  const b = state.settings?.battery;
  if (!b) return;
  $("batt-fill").style.width = `${b.percent}%`;
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
  s.dpi.presets.forEach((v, i) => {
    const row = el("div", "row");
    row.appendChild(el("span", "label", `Preset ${i + 1}${i === s.dpi.active ? "  • active" : ""}`));
    const input = el("input");
    input.type = "number"; input.min = 50; input.max = s.dpi.max || 26000; input.step = 50; input.value = v;
    const apply = el("button", "btn primary", "set");
    apply.onclick = () => invoke("set_dpi", { index: i, value: Math.round(+input.value) });
    row.append(input, apply);
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
  wireEvents();
  buildRail();
  render();
  state.palettes = await invoke("palettes");
  await invoke("read_all");
  await invoke("read_buttons");
}
boot();
