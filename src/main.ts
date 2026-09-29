import { listen } from "@tauri-apps/api/event";

// Por enquanto só confirma que o atalho global chega ao front-end.
// Na Fase 2 este arquivo vira a lógica do popup.
window.addEventListener("DOMContentLoaded", async () => {
  const statusEl = document.querySelector<HTMLElement>("#status");
  let count = 0;

  await listen("shortcut-pressed", () => {
    count += 1;
    if (statusEl) {
      statusEl.textContent = `Atalho recebido ${count}x`;
    }
  });
});