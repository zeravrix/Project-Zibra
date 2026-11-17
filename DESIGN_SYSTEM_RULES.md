# ZIBRA Design System & UI Integration Rules

Authoritative reference for design tokens, UI components, styling, and glassmorphic layout rules for the native desktop reader (**ZibraX**).

---

## 1. Token Definitions

### File Locations
- **Active Rust Model**: `crates/zibra-core/src/settings.rs`
- **Active CSS Tokens**: `src/app.css`
- **Global Svelte Stores**: `src/lib/stores/app.svelte.js`

### Token Structure & Material Design Schemes
The application uses 13 preset Material Design 3 palettes, switchable dynamically with AMOLED pure-black overrides:

| Theme Name | Enum ID | Primary Hex | Dark Surface | Accent Role |
| :--- | :--- | :--- | :--- | :--- |
| **Totoro (Indigo)** | `totoro` | `#4285F4` | `#0E1116` | Default Primary |
| **Dynamic Monet** | `monet` | `#A8C7FA` | `#0D1219` | Dynamic Material Blue |
| **Expressive (Lilac)** | `expressive` | `#A855F7` | `#140F19` | Vibrant Purple |
| **Miku (Teal)** | `miku` | `#14B8A6` | `#0B1213` | Cyber Teal |
| **Asuka (Crimson)** | `asuka` | `#EF4444` | `#160E0D` | Coral Red |
| **Mion (Emerald)** | `mion` | `#22C55E` | `#0D140C` | Forest Sage |
| **Sakura (Rose)** | `sakura` | `#EC4899` | `#160E12` | Cherry Blossom |
| **Itsuka (Amber)** | `itsuka` | `#F97316` | `#16100A` | Sunset Peach |
| **AMOLED Void** | `amoled` | `#4285F4` | `#000000` | True Pitch Black |
| **High Contrast** | `mono` | `#FFFFFF` | `#050505` | Ultra Contrast |

---

## 2. Component Hierarchy

- **`NavRail.svelte`**: Icon navigation rail (History, Favorites, Explore, Settings).
- **`SearchBar.svelte`**: Integrated instant search bar with clear button.
- **`CoverGrid.svelte`**: Virtualized responsive grid of manga cover cards with 3:4.1 aspect ratio.
- **`CoverCard.svelte`**: Manga cover card with hover bloom, status badge, title label, and favorite button.
- **`Details.svelte`**: Split-pane manga details with synopsis, genre pills, chapter list, and in-app Cloudflare solver action.
- **`Reader.svelte`**: Continuous vertical webtoon reader with zoom, brightness, contrast, inverted colors, and auto-preloading.
- **`Settings.svelte`**: Settings view for appearance, themes, cache limits, download limits, and NSFW filter toggle.
