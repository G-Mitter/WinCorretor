# WinCorretor

Corretor de gramática e tom de texto para Windows. Selecione um texto em qualquer programa, aperte **Ctrl+Alt+O**, escolha o tom e aperte **Enter**: o texto é substituído no lugar.

## Como usar

1. Selecione um texto (Bloco de Notas, navegador, Word, Outlook, Teams…).
2. Aperte `Ctrl+Alt+O`. Um popup aparece perto do mouse.
3. Escolha o tom com `1`–`6` (ou setas + `Enter`):
   Corrigir gramática · Profissional · Educado · Informal · Resumir · Detalhar.
4. Na prévia: `Enter` aplica, `Ctrl+C` só copia, `R` refaz, `←` troca o tom, `Esc` cancela.

O app fica no ícone da bandeja (perto do relógio). Pelo menu dele: **Configurações**, **Pausar atalho** e **Sair**.

## Configuração

Na tela de Configurações:

- **Chaves de IA** (gratuitas), guardadas no Gerenciador de Credenciais do Windows:
  - Groq (principal): https://console.groq.com/keys
  - Gemini (reserva automática): https://aistudio.google.com/apikey
- Atalho, tom padrão, modelos e início automático com o Windows.

> Privacidade: o texto selecionado é enviado ao provedor de IA configurado (Groq e, se ele falhar, Google Gemini). Nas camadas gratuitas, os provedores podem usar o conteúdo para melhorar seus produtos.

## Desenvolvimento

Requisitos: Rust (stable), Node.js LTS e o WebView2 (já incluso no Windows 10/11).

```powershell
npm install
npm run tauri dev      # app em modo desenvolvimento
npm run tauri build    # instalador em src-tauri\target\release\bundle\nsis\
cd src-tauri; cargo test; cargo clippy -- -D warnings
```

Em desenvolvimento, as chaves também podem vir de `src-tauri/.env` (veja `.env.example`).

Stack: Tauri 2 · Rust · TypeScript (sem framework) · Vite.
