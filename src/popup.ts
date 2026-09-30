import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// Popup junto ao cursor: escolher o tom → ver a prévia → aplicar ou cancelar.

type Style = "grammar" | "professional" | "polite" | "casual" | "shorter" | "detailed";
type View = "tones" | "loading" | "preview" | "error";

type PopupOpen = {
  snippet: string;
  totalChars: number;
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
// Invalida respostas antigas se o popup for reaberto durante uma chamada.
let requestId = 0;

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

async function choose(index: number) {
  selected = index;
  const tone = TONES[index];
  const current = ++requestId;
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

function cancel() {
  requestId++;
  invoke("popup_cancel");
}

function apply() {
  invoke("popup_apply").catch((err) => {
    $("error").textContent = String(err);
    show("error");
  });
}

window.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    event.preventDefault();
    cancel();
    return;
  }

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
  } else if (view === "preview" && event.key === "Enter") {
    event.preventDefault();
    apply();
  }
});

listen<PopupOpen>("popup-open", (event) => {
  requestId++;
  selected = 0;
  const { snippet, totalChars } = event.payload;
  $("snippet").textContent = totalChars > snippet.length ? `${snippet}…` : snippet;
  renderTones();
  show("tones");
  window.focus();
});

renderTones();
