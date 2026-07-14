# Birdet

**Learn to recognise birds by ear.** Birdet is a desktop app that plays you a bird
recording, shows its spectrogram, and asks you to pick the right species from four
choices. A spaced-repetition scheduler shows you the birds you find hard more often
and the distinctive ones less, so your practice time goes where it's needed.

Built with [Tauri v2](https://tauri.app) (Rust backend) + [SvelteKit](https://kit.svelte.dev)
(Svelte 5), with SQLite for local storage. Everything runs on your machine.

---

## How it works

- **You build your own collection.** Birdet ships with a few demo birds; from there you
  import local species from an [eBird](https://ebird.org) region — ranked by how commonly
  they're actually reported there — or search and add individual species by name. Each bird
  is imported with one or more recordings pulled from [Xeno-Canto](https://xeno-canto.org).

- **Train by listening.** Each question plays one recording (chosen at random from the clips
  stored for that bird) and shows its spectrogram. Replay it as often as you like, then pick
  one of four species — or *Skip* if you don't know (it counts as a miss so the bird returns
  sooner). Immediate feedback shows the correct name.

- **Spaced repetition schedules the review.** New birds start in short *learning* steps
  (~1 min, ~10 min) to lock them in, then graduate to day-scale *review* intervals that grow
  with each correct answer (SM-2 + learning-steps, conceptually the same as Anki). Miss a
  bird you'd learned and it *lapses* back into short steps. You don't manage any of this —
  just answer honestly.

- **Session modes.** *Learn & Review* (due reviews, then new birds up to a per-session cap),
  *Review due* only, *New only*, or *Practice* — a fixed-length drill that ignores the
  schedule but still tracks accuracy.

- **Packs.** Group birds into named subsets ("garden birds", "warblers", "coast trip") and
  train narrowed to that set. Packs are just filters — deleting one never removes birds or
  recordings. Packs can be exported/imported as `.birdet` files to share.

- **Manage recordings per bird.** Each bird's detail page lets you play, add more, or delete
  individual clips — with Xeno-Canto quality rating (`q:A` best), type (song/call), and a
  link back to the source. Having several clips per bird stops you keying off one recording's
  quirks instead of the actual song.

See the in-app **Guide** for the full walkthrough, and **Stats** for each bird's state and
your accuracy.

---

## Installation

### Windows (end users)

Grab the latest `*-setup.exe` from the [Releases](../../releases) page and run it. It's a
per-user install (no admin/UAC). The installer is unsigned, so Windows SmartScreen shows a
warning — choose **More info → Run anyway**. To update, run a newer installer; your data in
`%APPDATA%\com.nateml.birdet` is preserved.

### First run — API keys

Birdet needs two free API keys to import birds (set them in **Settings**):

1. **eBird** — tells Birdet which species live in a region. Get a token at
   [ebird.org/api/keygen](https://ebird.org/api/keygen) (create an account if needed).
2. **Xeno-Canto** — provides the recordings. Create an account at
   [xeno-canto.org](https://xeno-canto.org/), verify your email, then copy the key from your
   profile details.

Then go to **Library → Import**, pick a region, and Birdet downloads the most common species'
recordings. (Imports take a little while — audio has to download — but run several species in
parallel.)

---

## Development

### Prerequisites

- [Rust](https://rustup.rs) (stable) and [Node.js](https://nodejs.org) with `pnpm`
  (`corepack enable pnpm`).

**Linux / WSL (Ubuntu 24.04)** — system libraries for Tauri's webview:

```bash
sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

Audio codecs (webkit2gtk decodes `<audio>` via GStreamer; without these, MP3 recordings fail
to play):

```bash
sudo apt install -y gstreamer1.0-libav gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly
```

WSL2 also needs a working audio sink (WSLg/PulseAudio) to actually hear output.

### Setup

```bash
pnpm install
```

The Rust backend uses sqlx offline mode (a committed `.sqlx/` cache), so a fresh clone builds
with **no database required** (`SQLX_OFFLINE=true`).

### Commands

```bash
pnpm tauri dev     # run the desktop app (Vite + Tauri)
pnpm dev           # frontend only in a browser (no Tauri APIs)
pnpm check         # type-check Svelte
pnpm lint          # lint + format check
pnpm format        # auto-format
pnpm tauri build   # production build
```

---

## Distribution (Windows builds)

Tauri's webview is native per-OS, so **Windows builds can't be cross-compiled from
Linux/WSL** — they run on Windows.

- **CI** — `.github/workflows/build-windows.yml` builds on `windows-latest`. Trigger it
  manually (Actions → Build Windows → Run workflow) to get an installer artifact.
- **Release** — push a `v*` tag and `tauri-action` builds and attaches the NSIS
  `*-setup.exe` to a draft GitHub Release. Bump `version` in
  `src-tauri/tauri.conf.json` per release.

---

## Tech notes

- **Frontend**: SvelteKit + Svelte 5 (runes) + Tailwind v4 + DaisyUI. File-based routing under
  `src/routes/`; Tauri IPC through thin `invoke()` wrappers in `src/lib/api/`.
- **Backend**: Rust in `src-tauri/src/` — `commands.rs` (IPC handlers) delegating to `services/`
  (quiz scheduler, packs, stats, the eBird→Xeno-Canto import pipeline, taxonomy, settings).
- **Storage**: SQLite via sqlx, migrations in `src-tauri/migrations/`, DB created and migrated
  on launch in the app-data dir. Imported audio downloads to `app_data_dir/recordings/`.
