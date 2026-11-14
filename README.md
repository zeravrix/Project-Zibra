<div align="center">

<img src="assets/logo.png" alt="Zibrax Logo" width="140" />

# Zibrax (Project Zibra)

**Production-Grade Native Manga, Manhwa & Webtoon Desktop Application for Linux & Windows**

[![Official Website](https://img.shields.io/badge/Website-zeravrix.pages.dev-7C4DFF?style=for-the-badge&logo=googlechrome&logoColor=white)](https://zeravrix.pages.dev)
[![Release](https://img.shields.io/badge/Release-v2.1.0-00E676?style=for-the-badge)](https://github.com/zeravrix/Project-Zibra/releases)
[![Platform](https://img.shields.io/badge/Platform-Linux%20%7C%20Windows-00B0FF?style=for-the-badge&logo=linux&logoColor=white)](#repository-structure)
[![Language](https://img.shields.io/badge/Language-Rust%20%2B%20Svelte%205-DEA584?style=for-the-badge&logo=rust&logoColor=black)](https://www.rust-lang.org/)
[![UI Framework](https://img.shields.io/badge/Architecture-Tauri%20v2%20%2B%20WebKitGTK-FF6D00?style=for-the-badge)](#architecture)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg?style=for-the-badge)](LICENSE)

</div>

---

## Overview

**Zibrax** is an ultra-fast, modern desktop reader engineered for comic, manga, manhwa, and webtoon enthusiasts. Built with a high-performance **100% pure Rust** backend and a responsive **Tauri v2 + Svelte 5** glassmorphic interface, Zibrax delivers instant startup (`< 100 ms`), ultra-low memory usage, and zero reliance on external JVM or runtime servers.

Frontend assets are bundled directly into the standalone binary via custom in-memory protocols, allowing the application to run out of the box with zero external dependencies.

---

## Key Highlights

- **1,250+ High-Speed Pure Rust Sources**:
  - Global catalog covering **18 languages** (`AR`, `EN`, `MULTI`, `JA`, `KO`, `ZH`, `FR`, `ES`, `PT`, `ID`, `TR`, `RU`, `VI`, `DE`, `IT`, `PL`, `TH`).
  - Native scrapers for **MangaDex API v5**, **MangaSwat**, **Azora**, **Team-X**, **Dilar**, **Asura Scans**, **Flame Comics**, **Toonily**, **HariManga**, **Komga / Kavita personal servers**, and **Local `.cbz` / archive directories**.
  - Built-in **NSFW Content Isolation Filter** (enabled by default) to filter adult sources across catalog views and search results.
- **In-App Cloudflare CAPTCHA Verification**:
  - Direct in-app Webview solver window allowing users to seamlessly view and manually pass Cloudflare Turnstile and DDoS-GUARD challenges.
  - Automatic session cookie sync back to the native HTTP client for uninterrupted reading.
- **Glassmorphic Material Design UI**:
  - Svelte 5 reactive component hierarchy with glassmorphic accents, fluid navigation rail, and dynamic theme switching (including AMOLED pitch black and high contrast modes).
  - Multi-tab Explore view with real-time ping latency indicators, quick filtering, and continuous search.
  - Interactive details view with synopsis, genre pills, chapter list, and instant bookmarking.
- **Continuous Infinite Webtoon & Paged Reader**:
  - Continuous vertical reading with seamless multi-chapter preloading.
  - Automatic `2048px` GPU tile slicing to prevent texture memory exhaustion on integrated graphics chips.
  - Real-time image adjustments: zoom, brightness, contrast, color inversion, and page gap trimming.
- **Resilient Offline Architecture**:
  - SQLite WAL (Write-Ahead Logging) for atomic state persistence: library, reading progress, and history.
  - Parallel `.cbz` archiver for complete offline chapter reading.

---

## Screenshots

<p align="center">
  <img src="docs/assets/screenshot-library.png" alt="Zibrax Library View" width="48%" />
  <img src="docs/assets/screenshot-reader.png" alt="Zibrax Reader View" width="48%" />
</p>

---

## Repository Structure

```text
Project-Zibra/
├── Cargo.toml                     # Root workspace manifest
├── Cargo.lock                     # Locked deterministic dependencies
├── index.html                     # Frontend entry point
├── package.json                   # Svelte 5 & Vite configuration
├── vite.config.js                 # Frontend build bundler configuration
├── dist/                          # Pre-built bundled webview assets
├── assets/                        # Icons, logos & branding materials
├── crates/
│   ├── zibra-core/                # SQLite WAL storage, models, settings & parallel downloader
│   ├── zibra-sources/             # 1,250+ pure Rust source catalog & multi-engine scrapers
│   └── zibra-reader/              # 2048px GPU strip slicer & LRU tile memory cache
├── src/                           # Svelte 5 glassmorphic UI components & state stores
│   ├── lib/
│   │   ├── components/            # CoverCard, CoverGrid, NavRail, SearchBar
│   │   ├── stores/                # Reactive application state stores
│   │   └── views/                 # Library, Explore, Details, Reader, Settings
│   ├── app.css                    # Design system tokens, glassmorphism & typography
│   └── main.js                    # UI bootstrap
├── src-tauri/                     # Tauri v2 native bridge & WebKitGTK host
│   ├── Cargo.toml                 # Desktop package configuration
│   ├── tauri.conf.json            # Tauri runtime configuration & window specifications
│   ├── capabilities/              # Security capabilities & window permissions
│   └── src/
│       ├── commands.rs            # Native IPC invocations & Cloudflare solver
│       ├── state.rs               # Thread-safe global application state
│       └── lib.rs                 # Tauri application builder & entry
└── distribution/                  # Packaging specs, desktop entries & launchers
```

---

## Launching the Standalone Application

The pre-compiled standalone binary is ready in the repository:

```bash
./target/release/zibra-desktop
```

---

## License

Licensed under the **GNU General Public License v3.0** — see [LICENSE](LICENSE) for details.
