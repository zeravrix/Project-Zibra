# Zibrax Design System & Architecture Specification

> **Design System Version:** 2.2.0  
> **Philosophy:** High-Performance, Minimalist Precision, Native Vector Graphics, Kotatsu Material 3 Glass.

---

## 1. Minimalist Native Design Principles

1. **No Clutter Chrome**:
   - The top application bar displays **only the transparent native logo** and essential high-utility actions (Fullscreen toggle, AMOLED high-contrast toggle).
   - No generic global search bars forced into title bars where they don't belong. Search belongs contextually in shelf and catalog screens where content queries happen.
   - No microscopic unreadable badge spam or redundant app title text taking up vertical viewing real-estate.

2. **Native GPU Vector Icons**:
   - Never rely on platform-dependent OS emojis for core UI controls (which look inconsistent, blurry, or unprofessional across Linux desktop environments and Windows).
   - All standard controls (History, Favorites, Explore, Feed, Downloads, Settings, Fullscreen, AMOLED, Back, Close, Search, Zoom) are rendered natively via `egui::Painter` vector geometry with antialiased bezier curves and strokes matching the active theme's primary color.

3. **Motion & Interaction (21st.dev inspired)**:
   - Floating continue buttons, HUD bars, and navigation rails utilize critical-damped spring physics (`SpringDamper` with stiffness 280.0, damping 26.0) for fluid 60/120 FPS transitions without linear jarring animation cuts.

---

## 2. Token Architecture

### 2.1 Color Tokens (`GlassTokens`)
Zibrax uses a three-layer token architecture adapting across 13 Kotatsu Material 3 color themes, plus an AMOLED pitch-black mode:

| Token Name | AMOLED Mode | Dark Mode | Light Mode | Usage |
|:---|:---|:---|:---|:---|
| `surface_dim` | `#000000` | `#0D1117` | `#F6F8FA` | Background foundation |
| `surface_container` | `#080808` | `#161B22` | `#FFFFFF` | Cards & Shelves |
| `surface_container_high` | `#121212` | `#21262D` | `#EAEFF5` | Modals & Overlays |
| `surface_glass` | `rgba(0,0,0,0.88)` | `rgba(22,27,34,0.82)` | `rgba(255,255,255,0.85)` | Navigation Rail & HUD |
| `border_glass` | `rgba(255,255,255,0.08)` | `rgba(255,255,255,0.12)` | `rgba(0,0,0,0.08)` | Subtle 1px card borders |
| `primary` | Dynamic Theme Tint | Dynamic Theme Tint | Dynamic Theme Tint | Active states, primary CTAs |
| `text_primary` | `#F0F6FC` | `#F0F6FC` | `#1F2328` | Main headlines & titles |
| `text_secondary` | `#8B949E` | `#8B949E` | `#656D76` | Chapter counts, subtitles |

### 2.2 Typography Scale
- **Display**: 22.0pt Proportional Bold (Screen titles, manga detail banners)
- **Headline**: 17.0pt Proportional Semi-bold (Section headers, dialogs)
- **Title**: 14.5pt Proportional Semi-bold (Manga card titles, reader headers)
- **Body**: 13.0pt Proportional Regular (Reading text, descriptions)
- **Caption**: 11.0pt Proportional Medium (Metadata chips, chapter numbers)

---

## 3. High-Fidelity Reader & Windowing Architecture

### 3.1 Memory Windowing (`[k-2, k+3]`)
To prevent out-of-memory crashes and stutter during long reading sessions:
- Active viewport page index $k$ maintains only $[k - 2, k + 3]$ in GPU texture memory and uncompressed tile RAM.
- Pages outside this window have their GPU textures and decoded tiles immediately pruned via `reader_pipeline.prune_except(&active_list)`.
- Placeholder heights are retained dynamically from measured image metadata to prevent scroll jumps and layout shifts.

### 3.2 High-DPI Image Quality
- Cover thumbnails and chapter pages are rendered using Catmull-Rom bicubic resampling (`FilterType::CatmullRom`) instead of bilinear triangle filtering, eliminating blurriness on 1080p, 1440p, and 4K displays.
- High-resolution textures are clamped up to 2048px maximum slice dimension to support high-DPI screens without GPU texture coordinate overflow.

---

## 4. Source Engine & Extractor Standards

- **Standard HTTP Headers**: Every source engine must provide realistic modern browser headers (`User-Agent`, `Accept`, `Referer`, `sec-ch-ua`) to prevent automated scraper detection and 403 Forbidden responses.
- **Regex Safety**: Strict avoidance of unsupported PCRE features (such as lookarounds) in the Rust standard `regex` crate to guarantee zero panics during cover URL normalization.
