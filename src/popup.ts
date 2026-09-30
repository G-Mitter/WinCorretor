import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// Popup junto ao cursor: escolher o tom → ver a prévia → aplicar, copiar ou cancelar.

type Style = "grammar" | "professional" | "polite" | "casual" | "shorter" | "detailed";
type View = "tones" | "loading" | "preview" | "error";

type PopupOpen = {
  snippet: string;
  totalChars: number;
  defaultStyle: Style;
};

const TONES: { style: Style; label: string }[] = [
  { style: "grammar", label: "Corrigir gramática" },
  { style: "professional", label: "Profissional" },
  { style: "polite", label: "Educado" },
  { style: "casual", label: "Informal" },
  { style: "shorter", label: "Resumir" },
  { style: "detailed", label: "Detalhar" },
];

let view: View = "tones";
let selected = 0;
// Invalida respostas antigas se o popup for reaberto ou o tom trocado durante uma chamada.
let requestId = 0;
// Evita apertar Enter duas vezes e colar em dobro.
let finishing = false;

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

function show(next: View) {
  view = next;
  for (const name of ["tones", "loading", "preview", "error"] as View[]) {
    $(`view-${name}`).hidden = name !== next;
  }
}

function renderTones() {
  const list = $("tones");
  list.innerHTML = "";
  TONES.forEach((tone, index) => {
    const item = document.createElement("li");
    item.role = "option";
    item.className = index === selected ? "tone selected" : "tone";
    item.setAttribute("aria-selected", String(index === selected));
    item.innerHTML = `<kbd>${index + 1}</kbd><span></span>`;
    item.querySelector("span")!.textContent = tone.label;
    item.addEventListener("click", () => choose(index));
    list.appendChild(item);
  });
}

function backToTones() {
  requestId++;
  renderTones();
  show("tones");
}

async function choose(index: number) {
  selected = index;
  const tone = TONES[index];
  const current = ++requestId;
  $("loading").textContent = `Reescrevendo (${tone.label})…`;
  show("loading");

  try {
    const result = await invoke<string>("popup_rewrite", { style: tone.style });
    if (current !== requestId) return;
    $("preview-label").textContent = tone.label;
    $("preview").textContent = result;
    show("preview");
  } catch (err) {
    if (current !== requestId) return;
    $("error").textContent = String(err);
    show("error");
  }
}

/** Aplica (cola no lugar) ou copia a prévia; o popup fecha pelo lado do Rust. */
async function finish(command: "popup_apply" | "popup_copy") {
  if (finishing) return;
  finishing = true;
  try {
    await invoke(command);
  } catch (err) {
    $("error").textContent = String(err);
    show("error");
  } finally {
    finishing = false;
  }
}

function cancel() {
  requestId++;
  invoke("popup_cancel");
}

/** O usuário selecionou só um trecho da prévia com o mouse? Então deixa o Ctrl+C normal agir. */
function hasPartialSelection() {
  return (window.getSelection()?.toString().length ?? 0) > 0;
}

window.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    event.preventDefault();
    cancel();
    return;
  }

  const key = event.key.toLowerCase();

  if (view === "tones") {
    const number = Number(event.key);
    if (number >= 1 && number <= TONES.length) {
      choose(number - 1);
    } else if (event.key === "ArrowDown") {
      selected = (selected + 1) % TONES.length;
      renderTones();
    } else if (event.key === "ArrowUp") {
      selected = (selected - 1 + TONES.length) % TONES.length;
      renderTones();
    } else if (event.key === "Enter") {
      choose(selected);
    } else {
      return;
    }
    event.preventDefault();
    return;
  }

  if (view === "preview") {
    if (event.key === "Enter") {
      finish("popup_apply");
    } else if (event.ctrlKey && key === "c" && !hasPartialSelection()) {
      finish("popup_copy");
    } else if (!event.ctrlKey && key === "r") {
      choose(selected);
    } else if (event.key === "ArrowLeft" || event.key === "Backspace") {
      backToTones();
    } else {
      return;
    }
    event.preventDefault();
    return;
  }

  if (view === "error") {
    if (event.key === "Enter" || key === "r") {
      choose(selected);
    } else if (event.key === "ArrowLeft" || event.key === "Backspace") {
      backToTones();
    } else {
      return;
    }
    event.preventDefault();
  }
});

listen<PopupOpen>("popup-open", (event) => {
  requestId++;
  finishing = false;
  const { snippet, totalChars, defaultStyle } = event.payload;
  selected = Math.max(0, TONES.findIndex((tone) => tone.style === defaultStyle));
  $("snippet").textContent = totalChars > snippet.length ? `${snippet}…` : snippet;
  renderTones();
  show("tones");
  window.focus();
});

renderTones();
