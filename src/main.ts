import { invoke } from "@tauri-apps/api/core";

// Tela de configurações (janela principal, aberta pelo ícone da bandeja).

type KeySource = "vault" | "envFile" | "missing";

type SettingsView = {
  shortcut: string;
  defaultStyle: string;
  groqModel: string;
  geminiModel: string;
  groqKey: KeySource;
  geminiKey: KeySource;
  provider: string;
  autostart: boolean;
};

type AiTest = { provider: string; millis: number; sample: string };

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const input = (id: string) => $<HTMLInputElement>(id);

const KEY_LABEL: Record<KeySource, string> = {
  vault: "salva no Windows",
  envFile: "vinda do .env",
  missing: "não configurada",
};

// Chaves marcadas para remoção ao salvar.
const toRemove = new Set<string>();

function setStatus(text: string, kind: "ok" | "error" | "" = "") {
  const el = $("status");
  el.textContent = text;
  el.className = `status ${kind}`;
}

function fill(view: SettingsView) {
  input("shortcut").value = view.shortcut;
  $("shortcut-hint").textContent = view.shortcut;
  $<HTMLSelectElement>("defaultStyle").value = view.defaultStyle;
  input("groqModel").value = view.groqModel;
  input("geminiModel").value = view.geminiModel;
  input("autostart").checked = view.autostart;
  input("groqKey").value = "";
  input("geminiKey").value = "";
  toRemove.clear();
  for (const key of ["groqKey", "geminiKey"] as const) {
    const badge = $(`${key}-status`);
    badge.textContent = KEY_LABEL[view[key]];
    badge.dataset.source = view[key];
  }
  $("provider").textContent = view.provider;
}

/** undefined = manter · "" = remover · texto = nova chave */
function keyChange(id: string): string | undefined {
  const value = input(id).value.trim();
  if (value) return value;
  return toRemove.has(id) ? "" : undefined;
}

async function save(event: SubmitEvent) {
  event.preventDefault();
  setStatus("Salvando…");
  try {
    const view = await invoke<SettingsView>("save_settings", {
      input: {
        shortcut: input("shortcut").value,
        defaultStyle: $<HTMLSelectElement>("defaultStyle").value,
        groqModel: input("groqModel").value,
        geminiModel: input("geminiModel").value,
        groqKey: keyChange("groqKey"),
        geminiKey: keyChange("geminiKey"),
        autostart: input("autostart").checked,
      },
    });
    fill(view);
    $("alert").hidden = true;
    setStatus("Configurações salvas.", "ok");
  } catch (err) {
    setStatus(String(err), "error");
  }
}

async function testAi() {
  setStatus("Testando a IA…");
  try {
    const result = await invoke<AiTest>("test_ai");
    setStatus(`${result.provider} respondeu em ${result.millis} ms: "${result.sample}"`, "ok");
  } catch (err) {
    setStatus(String(err), "error");
  }
}

/** Transforma o teclado pressionado no formato do atalho, ex.: "Ctrl+Alt+O". */
function captureShortcut(event: KeyboardEvent) {
  if (event.key === "Tab") return;
  event.preventDefault();
  if (["Control", "Alt", "Shift", "Meta"].includes(event.key)) return;

  const parts: string[] = [];
  if (event.ctrlKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  if (event.metaKey) parts.push("Super");

  // event.code não depende do layout (ABNT2), ex.: "KeyO", "Digit5", "F8", "Space".
  const code = event.code.replace(/^Key/, "").replace(/^Digit/, "");
  parts.push(code);
  input("shortcut").value = parts.join("+");
}

window.addEventListener("DOMContentLoaded", async () => {
  input("shortcut").addEventListener("keydown", captureShortcut);
  $("form").addEventListener("submit", save);
  $("test").addEventListener("click", testAi);

  document.querySelectorAll<HTMLButtonElement>("[data-remove]").forEach((button) => {
    button.addEventListener("click", () => {
      const id = button.dataset.remove!;
      toRemove.add(id);
      input(id).value = "";
      $(`${id}-status`).textContent = "será removida ao salvar";
    });
  });

  fill(await invoke<SettingsView>("get_settings"));

  const hotkeyError = await invoke<string | null>("hotkey_error");
  if (hotkeyError) {
    $("alert").textContent = hotkeyError;
    $("alert").hidden = false;
  }
});
