import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// Tela de teste da captura (issue #4).
// Na Fase 2 este arquivo vira a lógica do popup.

type Selection = {
  text: string;
  sourceWindow: number;
};

window.addEventListener("DOMContentLoaded", async () => {
  const statusEl = document.querySelector<HTMLElement>("#status");
  const capturedEl = document.querySelector<HTMLElement>("#captured");

  // Avisa se o atalho global não pôde ser registrado.
  const hotkeyError = await invoke<string | null>("hotkey_error");
  if (hotkeyError && statusEl) statusEl.textContent = `Atenção: ${hotkeyError}`;

  await listen<Selection>("selection-captured", (event) => {
    const { text } = event.payload;
    if (statusEl) statusEl.textContent = `Texto capturado (${text.length} caracteres):`;
    if (capturedEl) capturedEl.textContent = text;
  });

  await listen<string>("capture-failed", (event) => {
    if (statusEl) statusEl.textContent = `Falha: ${event.payload}`;
    if (capturedEl) capturedEl.textContent = "";
  });
});
