# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Birdet is a Tauri v2 desktop app for learning bird calls by ear. Users listen to recordings and pick the correct bird from multiple-choice options. Built with SvelteKit (frontend) + Rust (backend), SQLite for persistence.

## Commands

```bash
# Run the Tauri dev app (starts both Vite and Tauri)
pnpm tauri dev

# Frontend only (browser, no Tauri APIs)
pnpm dev

# Type-check Svelte
pnpm check

# Lint + format check
pnpm lint

# Auto-format
pnpm format

# Production build
pnpm tauri build
```

No test suite exists yet.

## Architecture

### Frontend — SvelteKit + Svelte 5 + Tailwind + DaisyUI

- `src/routes/` — file-based routing. Key routes: `/train`, `/packs`, `/library`, `/stats`, `/settings`
- `src/lib/api/` — thin wrappers around `invoke()` calls to Tauri commands (`quiz.ts`, `packs.ts`)
- `src/lib/stores/theme.ts` — theme persistence
- All Tauri IPC goes through `@tauri-apps/api/core`'s `invoke()`

### Backend — Rust (src-tauri/src/)

- `main.rs` — initializes DB, registers commands, manages `AppState { db: Db }`
- `db.rs` — opens SQLite pool via sqlx, runs migrations from `migrations/`
- `commands.rs` — `#[tauri::command]` handlers (thin layer, delegates to services)
- `services/` — business logic: `quiz.rs` (next question, submit answer), `packs.rs` (list packs)
- `models.rs` — shared structs (`Pack`)
- `errors.rs` — error types

### Database

SQLite, managed by sqlx migrations in `src-tauri/migrations/`. Schema:

- `birds` — species (common_name, scientific_name)
- `recordings` — audio files linked to birds; actual files in `src-tauri/resources/recordings/`
- `packs` / `pack_recordings` — curated sets of recordings
- `mastery` — per-bird seen/correct counts
- `history` — per-attempt log

### IPC pattern

Tauri commands are defined in `commands.rs`, registered in `main.rs`'s `invoke_handler!`, and called from `src/lib/api/*.ts` via `invoke('command_name', { ...args })`. New commands follow this same three-step pattern.

## First-time machine setup (Linux/WSL, Ubuntu 24.04)

1. System libs for Tauri's webview (without these `cargo`/`tauri` fail to build):
   ```bash
   sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
     libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
   ```
2. `corepack enable pnpm` then `pnpm install`.
3. **`DATABASE_URL` for sqlx compile-time checks.** `services/packs.rs` uses the checked
   `sqlx::query!` macro, which verifies SQL against a real DB *at build time*. Without it, `cargo`
   fails with "cannot infer type". Create a schema-only dev DB and point `src-tauri/.env` at it:
   ```bash
   cd src-tauri
   for f in migrations/*.sql; do sqlite3 dev.db < "$f"; done   # seed insert errors are fine — schema is what matters
   printf 'DATABASE_URL=sqlite://%s/dev.db\n' "$(pwd)" > .env
   ```
   `.env` + `dev.db` are gitignored (machine-specific). Alternatively commit a `.sqlx/` offline
   cache (`cargo sqlx prepare`) to drop the per-machine step entirely.
4. **Build-script approval:** `pnpm-workspace.yaml` has an `allowBuilds:` block (this env's
   supply-chain policy). New deps with postinstall scripts (e.g. `esbuild`, `@tailwindcss/oxide`)
   appear there as `<pkg>: set this to true or false` and a hook re-adds them if deleted. Set each
   to `true` or `pnpm install` exits 1 with `ERR_PNPM_IGNORED_BUILDS`. Do not remove the block.

## Notes

- `WEBKIT_DISABLE_DMABUF_RENDERER=1` is set in the `tauri` npm script (needed on Linux/WSL)
- `dotenv` is loaded in `main.rs` — place secrets/env vars in `src-tauri/.env`
- Bird audio resources are bundled via `tauri.conf.json` `bundle.resources`
