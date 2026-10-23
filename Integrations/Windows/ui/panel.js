const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
let state = null;

function esc(s) {
  return String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);
}

function countdown(ts) {
  if (!ts) return "";
  let s = Math.max(0, Math.round(ts - Date.now() / 1000));
  if (s === 0) return "resetting now";
  const d = Math.floor(s / 86400); s -= d * 86400;
  const h = Math.floor(s / 3600); s -= h * 3600;
  const m = Math.floor(s / 60);
  if (d > 0) return `resets in ${d}d ${h}h`;
  if (h > 0) return `resets in ${h}h ${m}m`;
  return `resets in ${Math.max(1, m)}m`;
}

function ago(ts) {
  if (!ts) return "never";
  const s = Math.round(Date.now() / 1000 - ts);
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  return `${Math.floor(s / 3600)}h ago`;
}

function render() {
  if (!state) return;
  const list = document.getElementById("list");
  if (!state.providers.length) {
    list.innerHTML = `<div class="hint">${state.refreshing ? "Refreshing…" : "No providers enabled. Open Settings to enable some."}</div>`;
  } else {
    list.innerHTML = state.providers.map((p) => {
      const wins = p.windows.map((w) => {
        const pct = Math.max(0, Math.min(100, w.usedPercent));
        const cls = w.usedPercent >= state.criticalThreshold ? "crit" : w.usedPercent >= state.warnThreshold ? "warn" : "";
        return `<div class="win">
          <div class="row"><span>${esc(w.label)}</span><span>${w.usedPercent.toFixed(0)}% used</span></div>
          <div class="bar"><div class="${cls}" style="width:${pct}%"></div></div>
          <div class="row small muted"><span>${esc(w.detail || "")}</span><span data-reset="${w.resetsAt || ""}">${countdown(w.resetsAt)}</span></div>
        </div>`;
      }).join("");
      const err = p.error ? `<div class="${p.notConfigured ? "hint" : "err"}">${esc(p.error)}</div>` : "";
      return `<div class="card">
        <div class="top"><span class="name">${esc(p.name)}</span><span class="plan">${esc(p.plan || "")}</span><span class="acct">${esc(p.account || "")}</span></div>
        ${wins}${err}
      </div>`;
    }).join("");
  }
  document.getElementById("updated").textContent = state.refreshing ? "Refreshing…" : `Updated ${ago(state.lastRefresh)}`;
  document.getElementById("next").textContent = state.refreshMinutes ? `every ${state.refreshMinutes} min` : "manual refresh";
}

function tick() {
  // Only the countdown text changes between refreshes; no re-render, no polling of the backend.
  document.querySelectorAll("[data-reset]").forEach((el) => {
    const ts = Number(el.dataset.reset);
    if (ts) el.textContent = countdown(ts);
  });
  if (state) document.getElementById("updated").textContent = state.refreshing ? "Refreshing…" : `Updated ${ago(state.lastRefresh)}`;
}

async function load() {
  state = await invoke("get_state");
  render();
}

document.getElementById("refresh").addEventListener("click", () => invoke("refresh_now"));
document.getElementById("settings").addEventListener("click", () => invoke("open_settings"));
document.addEventListener("keydown", (e) => { if (e.key === "Escape") invoke("close_panel"); });
listen("usage-updated", load);
load();
setInterval(tick, 30000);
