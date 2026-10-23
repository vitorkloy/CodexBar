const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const $ = (id) => document.getElementById(id);
let data = null;

function status(msg, isError) {
  $("status").textContent = msg;
  $("status").className = isError ? "small err" : "small muted";
}

async function load() {
  data = await invoke("get_settings");
  const s = data.settings;
  $("refresh").value = String(s.refreshMinutes);
  $("autostart").checked = data.autostart;
  $("notifications").checked = s.notifications;
  $("warn").value = s.warnThreshold;
  $("crit").value = s.criticalThreshold;
  $("providers").innerHTML = data.providers
    .map((p) => `<label class="line"><input type="checkbox" data-id="${p.id}" ${p.enabled ? "checked" : ""}/> ${p.name}</label>`)
    .join("");
  $("copilotState").textContent = data.copilotHasToken
    ? "A GitHub token is saved in config.json."
    : "No token saved; CodexBar will also try the Copilot IDE plugin login and `gh auth token`.";
  $("copilotHost").value = data.copilotHost || "";
  $("cursorState").textContent = data.cursorManual ? "Using the manual cookie header." : "Using the Cursor app login.";
  $("configPath").textContent = data.configPath;
  $("configError").textContent = data.configError || "";
  $("version").textContent = "v" + data.version;
}

$("save").addEventListener("click", async () => {
  const enabled = {};
  document.querySelectorAll("#providers input").forEach((el) => (enabled[el.dataset.id] = el.checked));
  try {
    await invoke("save_settings", {
      settings: {
        refreshMinutes: Number($("refresh").value),
        notifications: $("notifications").checked,
        warnThreshold: Number($("warn").value) || 80,
        criticalThreshold: Number($("crit").value) || 95,
        autostartInitialized: true,
      },
      autostart: $("autostart").checked,
      enabled,
    });
    status("Saved.");
    load();
  } catch (e) {
    status(String(e), true);
  }
});

$("copilotSignIn").addEventListener("click", async () => {
  try {
    status("Requesting a device code…");
    const code = await invoke("copilot_sign_in");
    $("copilotCode").textContent = code.userCode;
    status(`Enter ${code.userCode} at ${code.verificationUri} (opened in your browser).`);
  } catch (e) {
    status(String(e), true);
  }
});
listen("copilot-signin", (e) => {
  $("copilotCode").textContent = "";
  status(e.payload.message, !e.payload.ok);
  load();
});

$("copilotSave").addEventListener("click", async () => {
  try {
    await invoke("set_copilot_token", { token: $("copilotToken").value, host: $("copilotHost").value });
    $("copilotToken").value = "";
    status("Copilot settings saved.");
    load();
  } catch (e) {
    status(String(e), true);
  }
});
$("cursorSave").addEventListener("click", async () => {
  try {
    await invoke("set_cursor_cookie", { header: $("cursorCookie").value });
    $("cursorCookie").value = "";
    status("Cursor cookie saved.");
    load();
  } catch (e) {
    status(String(e), true);
  }
});
$("cursorClear").addEventListener("click", async () => {
  await invoke("set_cursor_cookie", { header: "" });
  status("Cursor will use the app login.");
  load();
});
$("openConfig").addEventListener("click", () => invoke("open_config_folder"));
$("upstream").addEventListener("click", () => invoke("open_url", { url: "https://github.com/steipete/CodexBar" }));
load();
