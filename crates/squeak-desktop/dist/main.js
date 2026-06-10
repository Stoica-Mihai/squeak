const { invoke } = window.__TAURI__.core;

const $ = (id) => document.getElementById(id);
const LOD_MM = { 0: "0.7 mm", 1: "1.0 mm", 2: "2.0 mm" };

async function load() {
  $("err").textContent = "";
  try {
    const o = await invoke("overview");
    render(o);
  } catch (e) {
    $("device").textContent = "disconnected";
    $("meta").textContent = "";
    $("err").textContent = String(e);
  }
}

function render(o) {
  $("device").textContent = o.name;
  $("meta").textContent = `· ${o.transport} · fw ${o.firmware}`;

  $("batt-fill").style.width = `${o.battery}%`;
  $("batt").textContent = `${o.battery}%${o.charging ? " ⚡" : ""}`;

  $("dpi-active").textContent = o.dpi[o.dpi_active] ?? "—";
  const pills = $("dpi-pills");
  pills.innerHTML = "";
  o.dpi.forEach((v, i) => {
    const el = document.createElement("span");
    el.className = "pill" + (i === o.dpi_active ? " on" : "");
    el.textContent = v;
    pills.appendChild(el);
  });

  $("polling").textContent = o.polling_hz || "—";
  $("lod").textContent = LOD_MM[o.lod] ?? "?";
  $("scroll").textContent = o.scroll_inverted ? "inverted" : "normal";

  const motion = $("motion");
  motion.textContent = o.motion ? "● on" : "○ off";
  motion.className = "v " + (o.motion ? "on" : "off");
  $("angle").textContent = o.angle === 0 ? "off" : `${o.angle}°`;

  $("debounce").textContent = `${o.debounce} ms`;
  $("sleep").textContent = `${o.sleep_min} min`;
}

$("refresh").addEventListener("click", load);
load();
