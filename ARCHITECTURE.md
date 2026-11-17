# Architecture & Engineering Reference — ZibraX

Authoritative architectural reference and design system guidelines for the **ZibraX** pure Rust + Tauri v2 / Svelte 5 application.

---

## 1. System Architecture

ZibraX is architected as a modular, high-performance desktop application structured into two core layers:
1. **Native Rust Host & Engine (`crates/*`, `src-tauri`)**: Owns disk persistence, SQLite WAL storage, network scraping, anti-hotlink pipelines, GPU tile slicing, and native window management.
2. **Reactive Frontend (`src/`)**: A fluid Svelte 5 glassmorphic single-page interface with zero runtime external dependencies, compiled directly into static assets (`dist/`) and bundled into the binary via Tauri v2 custom in-memory protocols.

```text
[ Desktop Window (WebKitGTK) ]
            │
    tauri://localhost (In-Memory Protocol)
            │
[ Svelte 5 Frontend (src/) ]
            │  IPC (tauri::invoke)
[ Tauri v2 Bridge (src-tauri) ]
    ├── AppState (Thread-safe RwLock/Arc)
    ├── LibraryStore (SQLite WAL Mode)
    ├── LiveSourceRegistry (1,250+ Scraper Engines)
    ├── ReaderPagePipeline & CoverCache (LRU Memory)
    └── DesktopDownloader (Parallel Resumable CBZ)
```

---

## 2. Design System Structure

### 2.1 Design Tokens (`src/app.css`)
- **Theme Accents**: 13 Material Design 3 palettes:
  - `Totoro (Indigo)`: Primary `#4285f4`, Dark Surface `#0e1116`
  - `Dynamic Monet`: Primary `#a8c7fa`, Dark Surface `#0d1219`
  - `Expressive (Lilac)`: Primary `#a855f7`, Dark Surface `#140f19`
  - `Miku (Teal)`: Primary `#14b8a6`, Dark Surface `#0b1213`
  - `Asuka (Crimson)`: Primary `#ef4444`, Dark Surface `#160e0d`
  - `Mion (Emerald)`: Primary `#22c55e`, Dark Surface `#0d140c`
  - `Sakura (Rose)`: Primary `#ec4899`, Dark Surface `#160e12`
  - `Itsuka (Amber)`: Primary `#f97316`, Dark Surface `#16100a`
  - `AMOLED Void`: Primary `#4285f4`, Pitch Black Surface `#000000`
  - `High Contrast`: Primary `#ffffff`, Dark Surface `#050505`
- **Spacing & Radii**:
  - Small Radius: `6px` (badges, chips)
  - Medium Radius: `10px` (cover cards, action buttons)
  - Large Radius: `16px` (panels, modals, dialogs)

---

## 3. Data Flow & Security

- **Standalone In-Memory Bundling**: Built assets (`dist/`) are embedded into the binary at compile time via Tauri's `custom-protocol`. No external HTTP servers or localhost ports are exposed.
- **In-App Cloudflare Resolution**: Protected sources trigger an in-app Webview window allowing manual resolution of Turnstile challenges. Solved cookies are securely written to the local encrypted SQLite configuration.
- **NSFW Guardrail**: Content tagged with adult keywords or `SourceTag::Nsfw` is isolated and filtered by default across catalog queries and global search.
