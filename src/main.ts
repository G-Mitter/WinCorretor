import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// Tela de teste do fluxo: captura (issue #4) + correção pelo Gemini (issue #5).
// Na Fase 2 este arquivo vira a lógica do popup.

type Selection = {
  text: string;
  sourceWindow: number;
};

type RewriteDone = {
  original: string;
  result: string;
  style: string;
};

const $ = (id: string) => document.getElementById(id);

function show(id: string, text: string) {
  const el = $(id);
  if (el) el.textContent = text;
  const title = $(`${id}-title`);
  if (title) title.hidden = text.length === 0;
}

function setStatus(text: string) {
  const el = $("status");
  if (el) el.textContent = text;
}

window.addEventListener("DOMContentLoaded", async () => {
  // Avisa se o atalho global não pôde ser registrado.
  const hotkeyError = await invoke<string | null>("hotkey_error");
  if (hotkeyError) setStatus(`Atenção: ${hotkeyError}`);

  await listen<Selection>("selection-captured", (event) => {
    setStatus("Corrigindo com o Gemini…");
    show("captured", event.payload.text);
    show("result", "");
  });

  await listen<string>("capture-failed", (event) => {
    setStatus(`Falha na captura: ${event.payload}`);
    show("captured", "");
    show("result", "");
  });

  await listen<RewriteDone>("rewrite-done", (event) => {
    setStatus("Pronto.");
    show("result", event.payload.result);
  });

  await listen<string>("rewrite-failed", (event) => {
    setStatus(`Falha na IA: ${event.payload}`);
  });
});
