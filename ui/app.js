const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const $ = (id) => document.getElementById(id);
const panel = $("panel");
let guide, state;

const esc = (s) => s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
const md = (s) => esc(s).replace(/\*\*(.+?)\*\*/g, "<b>$1</b>");
const t = (v) => (v ? v[state.lang] || v.de : "");

// XP penalty window: 3 levels + 1 per 16 character levels.
function levelWarn(char, area) {
  return char && area && Math.abs(area - char) > 3 + Math.floor(char / 16);
}

// Same matching as src/guide.rs: "A -> B" and "A / B" mean either, a leading "The" is ignored.
const norm = (z) => z.trim().toLowerCase().replace(/^the /, "");
const zonesOf = (z) => z.split("->").flatMap((p) => p.split("/")).map(norm).filter(Boolean);

// The step's zone if the player is somewhere else (town, hideout, wrong zone), else "".
function goTo(s) {
  if (!state.zone || zonesOf(s.zone).includes(norm(state.zone))) return "";
  return s.zone.split("->")[0].split("/")[0].trim();
}

function render() {
  const i = state.step, s = guide.steps[i];
  const act = guide.acts[s.act];
  document.documentElement.lang = state.lang;
  panel.classList.toggle("mini", state.mini);
  panel.style.left = state.x + "px";
  panel.style.top = state.y + "px";

  $("act").textContent = `${t(act.short)} · ${i + 1}/${guide.steps.length}`;
  const lv = $("levels");
  lv.textContent = state.char_level ? `Lvl ${state.char_level}` + (state.area_level ? ` · ${state.lang === "de" ? "Gebiet" : "Area"} ${state.area_level}` : "") : "";
  lv.classList.toggle("warn", levelWarn(state.char_level, state.area_level));

  const cur = $("current");
  cur.className = s.tags.join(" ");
  const go = goTo(s);
  const goLabel = state.lang === "de" ? "Gehe nach" : "Go to";
  if (state.mini) {
    // One line. Elsewhere: where to go and what to do there. In the zone: what to do and where next.
    const to = guide.steps.slice(i + 1).find((n) => n.zone !== s.zone);
    cur.innerHTML = go
      ? `<span class="go">${goLabel} <b>${esc(go)}</b>:</span> <span class="goal">${md(t(s.text))}</span>`
      : `<span class="zone">${esc(s.zone)}</span> <span class="goal">${md(t(s.text))}</span>` +
        (to ? ` <span class="to">-&gt; ${esc(to.zone)}</span>` : "");
    $("upcoming").innerHTML = "";
    $("nolog").hidden = true;
    reportHitRect();
    return;
  }
  const opt = s.tags.includes("must") ? "" : `<span class="optional">optional</span>`;
  cur.innerHTML =
    (go ? `<div class="go">${goLabel} <b>${esc(go)}</b><span class="here">${esc(state.zone)}</span></div>` : "") +
    `<div class="zone">${esc(s.zone)}${s.level ? `<span class="lvl">Lvl ${esc(s.level)}</span>` : ""}${opt}</div>` +
    `<div class="goal">${md(t(s.text))}</div>` +
    (s.hint ? `<div class="hint">${md(t(s.hint))}</div>` : "") +
    (s.notes.length ? `<ul class="notes">${s.notes.map((n) => `<li>${md(t(n))}</li>`).join("")}</ul>` : "");

  $("upcoming").innerHTML = guide.steps
    .slice(i + 1, i + 3)
    .map((n) => `<li><span class="z">${esc(n.zone)}</span>${md(t(n.text))}</li>`)
    .join("");
  $("nolog").hidden = state.log_found;
  panel.hidden = false;
  reportHitRect();
}

// The Rust side makes the overlay clickable only inside this rectangle.
function reportHitRect(full) {
  const r = full ? { x: 0, y: 0, width: innerWidth, height: innerHeight } : panel.getBoundingClientRect();
  invoke("set_hit_rect", { x: r.x, y: r.y, w: r.width, h: r.height });
}
new ResizeObserver(() => reportHitRect()).observe(panel);

$("prev").onclick = () => invoke("step", { delta: -1 });
$("next").onclick = () => invoke("step", { delta: 1 });
$("mini").onclick = () => invoke("toggle_mini");

// Drag by the header (mini mode: by the whole line, a plain click opens the full panel).
// While dragging the whole window stays clickable, otherwise a fast mouse
// would leave the panel and drop the drag.
panel.addEventListener("mousedown", (e) => {
  if (e.target.closest("button") || (!state.mini && !e.target.closest("#bar"))) return;
  let moved = false;
  const dx = e.clientX - panel.offsetLeft, dy = e.clientY - panel.offsetTop;
  reportHitRect(true);
  const move = (m) => {
    moved = true;
    panel.style.left = Math.max(0, Math.min(innerWidth - panel.offsetWidth, m.clientX - dx)) + "px";
    panel.style.top = Math.max(0, Math.min(innerHeight - panel.offsetHeight, m.clientY - dy)) + "px";
  };
  const up = () => {
    removeEventListener("mousemove", move);
    removeEventListener("mouseup", up);
    if (moved) invoke("save_pos", { x: panel.offsetLeft, y: panel.offsetTop });
    else if (state.mini) invoke("toggle_mini");
    else reportHitRect();
  };
  addEventListener("mousemove", move);
  addEventListener("mouseup", up);
});

(async () => {
  guide = await (await fetch("guide.json")).json();
  state = await invoke("snapshot");
  render();
  listen("state", (e) => {
    state = e.payload;
    render();
  });
})();
