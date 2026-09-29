# AGENTS.md

## Project overview

This repository is a Tauri desktop app built with:
- Frontend: Vite + TypeScript + vanilla HTML/CSS
- Desktop shell: Tauri 2
- Backend: Rust commands exposed to the frontend via `tauri::command`

The app is intentionally small and lightweight. Favor straightforward, targeted changes over framework-heavy abstractions.

## Key locations

- `src/`: frontend code and UI assets
  - `src/main.ts`: browser-side logic and event wiring
  - `src/styles.css`: UI styling
  - `index.html`: DOM structure
- `src-tauri/src/`: Rust backend
  - `src-tauri/src/lib.rs`: Tauri app setup and command registration
- `package.json`: frontend scripts and dependencies
- `src-tauri/Cargo.toml`: Rust crate configuration and Tauri dependencies

## Working conventions

- Keep the frontend and Rust layers explicit. When adding a new backend command, update the corresponding TypeScript call site and keep the invocation contract obvious.
- Prefer small, local edits. This project does not appear to have a large architecture; broad refactors are likely unnecessary.
- Stay aligned with the existing minimal Tauri pattern: one HTML page, one `main.ts` entry, and a small Rust command surface.
- If a change affects UI state, update the front-end logic in `src/main.ts` and related DOM selectors without creating unnecessary abstractions.
- For Rust changes, keep command signatures simple and serializable (`String`, primitive types, or small data structures) because Tauri invokes them from JS.

## Commands

Use the project scripts from the repository root:

- `npm install` to install frontend dependencies
- `npm run dev` to run the Vite frontend
- `npm run tauri dev` to launch the full Tauri desktop app during development
- `npm run build` to run TypeScript + Vite build

For Rust-side checks, prefer the Tauri/Cargo workflow used by the app, and keep command registration in `src-tauri/src/lib.rs` consistent with the JS invoke calls.

## Quality bar

- Prefer readability and clarity over clever abstractions.
- Preserve the existing minimal structure.
- Do not add new frameworks or build systems unless the task explicitly requires it.
- If you introduce a new Tauri command, ensure it is registered in the handler list and that the frontend call matches its signature.

## Documentation

For project details beyond this file, see:
- [README.md](README.md)
