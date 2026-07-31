/* Event wiring for markup that previously carried inline on* handlers.
   Inline handlers need script-src 'unsafe-inline', which would defeat the CSP,
   so index.html tags elements with data-act and they are bound here instead. */

const ACTIONS = {
  navDrawer: () => fdDrawer("navDrawer", "navScrim"),
  toggleTheme: () => toggleTheme(),
  dpiDown: () => stepDpi(-50),
  dpiUp: () => stepDpi(50),
  toggleLock: () => toggleLock(),
  refresh: () => doRefresh(),
  checkFw: () => checkFw(),
  openHelp: () => openHelp(),
  quit: () => quit(),
  closeHelp: () => document.getElementById("helpModal").close(),
  pickMouse: () => pickType("mouse"),
  pickMedia: () => pickType("media"),
  pickDisable: () => pickType("disable"),
  pickDefault: () => pickType("default"),
  closeRemap: () => closeRemap(),
  saveRemap: () => saveRemap(),
};

document.addEventListener("click", (e) => {
  const el = e.target.closest("[data-act]");
  if (!el) return;
  const fn = ACTIONS[el.dataset.act];
  if (fn) fn();
});

const dpiNum = document.getElementById("dpiNum");
dpiNum.addEventListener("input", () => onDpiType(dpiNum.value));
dpiNum.addEventListener("change", () => commitDpi());
dpiNum.addEventListener("keydown", (e) => {
  if (e.key === "Enter") {
    e.preventDefault();
    commitDpi();
    dpiNum.blur();
  }
});

const dpiSlider = document.getElementById("dpiSlider");
dpiSlider.addEventListener("input", () => onSlider(dpiSlider.value));
dpiSlider.addEventListener("change", () => commitDpi());
