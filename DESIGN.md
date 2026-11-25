# DESIGN.md: Ruthenium UI Specification

- **Status:** Normative UI source of truth
- **Target platform:** Windows 11 desktop first (WebView2 content surface, Rust + Wry/Tao shell)
- **Visual direction:** Ruthenium Browser (Two-row unified chrome: `44px` Floating Pill Tab Strip + `48px` Rounded Navigation & Pill Omnibox Toolbar, three synced themes with `Arctic Minimal` default `#e8eef6` / `#f8fafc`, plus `Midnight Ruthenium` `#080d1a` / `#0d1426` and `Nebula Aurora` `#070913` / `#0c1021`, Windows 11 native caption controls)
- **Dial:** `ENERGY 1 / RHYTHM 1 / MOTION 1`

---

## 1. Product Intent & Reference Boundaries

This browser is a quiet desktop instrument built to frame the open web without competing with it. Every pixel of browser chrome exists to orient the user, execute navigation quickly, and step out of the way. Think matte dark anodized aluminum, basalt slate, birch paper in light mode, crisp hairline joinery, and honest typographic hierarchy rather than sci-fi HUDs or consumer social software.

### What we learn from modern desktop browsers (and what we reject)

We study **Arc Browser** and **Zen Browser** strictly as ergonomic references, never as visual templates to copy:

- **Adopted spatial lessons:**
  - Modern desktop monitors have surplus horizontal width and scarce vertical height. Moving tab management to a collapsible vertical rail reclaims vertical reading space while keeping long page titles legible.
  - Separating persistent, user-curated pinned anchors from ephemeral session tabs prevents tab-strip clutter.
  - Lightweight workspaces isolate task contexts (such as work, research, and local development) inside one window without tab hoarding.
  - A keyboard-triggered command palette (`Ctrl+K` / `Ctrl+T`) reduces pointer travel for opening URLs, switching tabs, and running browser commands.
  - A dedicated Focus Mode (`Ctrl+Shift+F`) lets the user collapse all chrome when reading or working full-canvas.
- **Rejected from Arc Browser:**
  - Hiding the persistent address/origin readout, which leaves users unsure which domain they are viewing and complicates security inspection.
  - Multi-stop saturated Space gradients, full-window color tinting per workspace, noisy grain overlays, playful stickers/Boosts, and thick `12px` outer window borders that waste screen area.
  - macOS-derived window chrome conventions forced onto Windows.
- **Rejected from Zen Browser:**
  - Address bars that dynamically teleport between a top toolbar input and a centered floating spotlight box depending on how focus arrived, breaking spatial predictability.
  - Merging Windows caption buttons (Minimize, Maximize, Close) into a narrow vertical tab strip where they crowd drag zones, break top-right Fitts's Law muscle memory, or interfere with Windows 11 Snap Layouts.

### Core design decisions (one-line rationale)

- **Dark-first mineral shell (`Basalt & Ice`) with first-class Light mode (`Limestone & Fjord`):** acts as a low-glare neutral darkroom mat around arbitrary bright or dark websites during long desktop sessions while supporting bright ambient environments on toggle.
- **Single `36px` top utility bar + left vertical tab rail:** preserves standard Windows caption geometry and a permanent origin readout on top while giving tabs vertical breathing room on the left.
- **Mineral basalt and limestone neutrals with one Fjord Blue accent:** keeps the browser shell perceptually neutral next to website palettes while making focus and active state unmistakable.
- **Native Windows optical sans (`Segoe UI Variable`) + un-ligatured monospace (`Cascadia Mono`):** delivers crisp DirectWrite/ClearType rendering with zero bundled font weight and prevents ligature-based URL spoofing.
- **Hairline `1px` borders over soft drop shadows:** separates trusted browser chrome from untrusted web content cleanly without floating-card visual noise.
- **In-place address bar + separate Command Palette:** guarantees the top origin bar never jumps across the screen while still giving keyboard users a fast centered launcher on demand.

### Identity motif

Two quiet, repeated structural cues establish product character across every surface:
1. **Hairline mineral frame:** matte chrome meets the web viewport along a crisp `1px` structural seam (`--border-subtle`) with a single `6px` inset curve at the top-left inner join where the vertical rail meets the utility bar; bottom and right viewport edges sit flush (`0px`) against the window edge.
2. **Origin-first tabular readout:** registrable domains (`eTLD+1`), port numbers, HTTP status codes, find-in-page counts, and keyboard shortcuts render with tabular alignment so technical and security-relevant details scan like a precision instrument.

---

## 2. Color Tokens (Dark-First)

The browser ships **Dark-first (`Basalt & Ice`)** as the default `:root` palette, with **Light (`Limestone & Fjord`)** as a complete, contrast-verified counterpart selectable in Settings (`Dark` [Default], `Light`, `System`). Browser chrome theme applies to shell surfaces and internal `browser://` pages; web content `prefers-color-scheme` follows the active theme unless overridden per user preference.

### Surface, text, border, and accent tokens

| Token | Dark (`Basalt & Ice`, Default) | Light (`Limestone & Fjord`) | Role & implementation rule |
| :--- | :--- | :--- | :--- |
| `--bg-window` | `#12161C` | `#F2F4F7` | Base window frame: top utility bar, vertical tab rail, and initial WebView2 background |
| `--bg-surface` | `#1B212A` | `#FFFFFF` | Active tab row, resting address pill, settings group panels |
| `--bg-subtle` | `#242C38` | `#E5E9EF` | Hover fill on chrome controls, favicon fallback tiles, `<kbd>` caps |
| `--bg-active` | `#2E3846` | `#D8DFE8` | Pressed (`:active`) fill for buttons, tabs, and menu rows |
| `--bg-overlay` | `#1E2530` | `#FCFDFF` | Elevated popovers: context menus, omnibox list, Command Palette, find bar |
| `--fg-primary` | `#EDF1F7` | `#141920` | Active tab title, registrable domain, primary UI text |
| `--fg-secondary` | `#B0BCCB` | `#434E5E` | Inactive tab titles, resting toolbar icons, menu labels |
| `--fg-muted` | `#8E9CB0` | `#546072` | URL protocol/path, shortcut hints, timestamps, unfocused window chrome text |
| `--border-subtle` | `#2A3340` | `#CFD6E0` | `1px` structural seam between chrome, rail sections, settings rows, and viewport |
| `--border-control` | `#738398` | `#6B788A` | Interactive input borders and high-visibility control boundaries (`>= 3:1`) |
| `--accent-primary` | `#6BA8DE` | `#245682` | Focus rings, active-tab `2px` bar, loading seam, primary button |
| `--accent-hover` | `#87BBE8` | `#1B4366` | Hover state for primary accent buttons and links |
| `--accent-subtle` | `#1C3147` | `#E1ECF7` | Pre-selected omnibox/palette row and subtle active filter wash |
| `--fg-on-accent` | `#0D1520` | `#FFFFFF` | Label/icon foreground placed directly on `--accent-primary` |

### Semantic status & OS-native exception tokens

| Token | Dark (Default) | Light | Role & constraint |
| :--- | :--- | :--- | :--- |
| `--status-success` | `#57C48D` | `#1E6B47` | Verified download completion or valid certificate detail |
| `--status-warning` | `#E5A93B` | `#8A5200` | Insecure HTTP origin badge, permission prompt caution |
| `--status-danger` | `#F27277` | `#B4232A` | TLS/certificate error, blocked download, destructive confirmation |
| `--win-close-hover-bg` | `#C42B1C` | `#C42B1C` | Native Windows 11 Close caption button hover fill (with `#FFFFFF` glyph) |
| `--win-close-active-bg` | `#B22518` | `#B22518` | Native Windows 11 Close caption button pressed fill (with `#FFFFFF` glyph) |

### Inactive window state (`WindowEvent::Focused(false)`)

When the native OS window is unfocused, keep `--bg-window` stable (never flash the whole window background) and step top-bar icons, caption glyphs, and address-bar text down one tier (`--fg-primary` -> `--fg-secondary`, `--fg-secondary` -> `--fg-muted`) so multi-monitor users immediately distinguish the active window.

### Verified WCAG 2.2 AA contrast floor

Every token pairing above has been verified with the WCAG 2.x relative luminance formula:
- `--fg-primary` on all surfaces (`--bg-window`, `--bg-surface`, `--bg-subtle`, `--bg-overlay`): `>= 11.7:1` in both themes.
- `--fg-secondary` on all surfaces: `>= 7.5:1` in Dark and `>= 6.9:1` in Light.
- `--fg-muted` on all surfaces: `>= 4.5:1` in both themes (passes normal-text AA even at `11px` to `12px` caption sizes).
- `--accent-primary` against `--bg-window` and `--bg-surface`: `7.16:1` / `6.38:1` in Dark and `6.99:1` / `7.70:1` in Light.
- `--fg-on-accent` on `--accent-primary`: `7.23:1` in Dark and `7.70:1` in Light.
- `--border-control` against `--bg-window` and `--bg-surface`: `>= 4.07:1` in both themes (exceeds the `3.0:1` WCAG 1.4.11 non-text contrast requirement).

Disabled controls use `opacity: 0.45` with `cursor: not-allowed` (or hidden when irrelevant) and never rely on fake low-contrast grey text for readable content.

---

## 3. Typography

Typography relies on native Windows 11 system fonts to achieve zero startup download cost, native DirectWrite subpixel hinting, and optical clarity at compact desktop UI sizes.

### Font stacks

- **UI Sans (`--font-sans`):** `"Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif`
  - Use `"Segoe UI Variable Small"` for `11px` keycaps and compact status chips where optical small-text legibility matters.
- **Technical Mono (`--font-mono`):** `"Cascadia Mono", "Consolas", ui-monospace, monospace`
  - Strictly use the un-ligatured `Cascadia Mono` family (never `Cascadia Code`) so character sequences inside URLs, paths, and ports never combine into misleading ligatures.

### Type scale

| Scale token | Font family | Size | Line height | Weight | Tracking | Usage |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `ui-micro` | `--font-mono` / `Small` | `11px` | `14px` | `500` | `0` | `<kbd>` shortcut caps, workspace tab count, HTTP port/status chip |
| `ui-caption` | `--font-sans` | `12px` | `16px` | `400` | `0` | Context menu hints, settings descriptions, bottom-left link hover preview |
| `ui-body` | `--font-sans` | `13px` | `18px` | `400` | `0` | Default desktop chrome size: tab titles, menu items, settings labels |
| `ui-emphasis` | `--font-sans` | `13px` | `18px` | `500` | `0` | Active tab title, active workspace name, group headers, primary buttons |
| `url-display` | `--font-sans` / `--font-mono` | `13px` | `18px` | `500` / `400` | `0` | Resting address bar uses `--font-sans` (`500` registrable domain, `400` path); focused raw URL uses `--font-mono` (`400`) |
| `palette-input` | `--font-sans` | `15px` | `22px` | `400` | `-0.01em` | Command Palette (`Ctrl+K` / `Ctrl+T`) query field |
| `page-heading` | `--font-sans` | `18px` | `24px` | `600` | `-0.01em` | Top heading on `browser://settings` and error/interstitial views |
| `dialog-title` | `--font-sans` | `16px` | `22px` | `600` | `-0.01em` | Permission prompt title, section headings on internal pages |

### Typographic rules

- Enable `font-variant-numeric: tabular-nums` on all counters, find-in-page match counts, shortcut labels, download progress figures, timestamps, and URL ports so numbers never jitter horizontally as values update.
- Use **sentence case** across all menus, buttons, tooltips, settings headings, and empty states (`Reopen closed tab`, `Pin tab`, `Clear browsing data`). Never use mechanical Title Case on every word, and never use wide-tracked ALL-CAPS eyebrow labels.
- Keep prose widths on internal pages (`browser://settings`, error views) at or below `68ch`.

---

## 4. Spacing Scale

All browser chrome uses a strict `4px` base unit with a single `2px` micro-step reserved for tight internal icon/cap alignment.

| Token | Value | Primary usage |
| :--- | :--- | :--- |
| `--space-0-5` | `2px` | Gap between adjacent `<kbd>` keycaps, session tab vertical gap, `2px` active indicator bar |
| `--space-1` | `4px` | Toolbar cluster button gap, context menu container padding, compact rail padding |
| `--space-1-5` | `6px` | Vertical padding inside `28px`/`32px` controls, icon-to-label tight gap |
| `--space-2` | `8px` | Standard horizontal padding inside tab rows, buttons, and menu items; icon-to-title gap |
| `--space-3` | `12px` | Horizontal padding inside address bar, Command Palette rows, and settings row vertical padding |
| `--space-4` | `16px` | Dialog padding, settings card horizontal inset, separation between toolbar zones |
| `--space-5` | `20px` | Internal card padding on diagnostic/error pages |
| `--space-6` | `24px` | Vertical spacing between settings groups, outer margin on internal utility screens |
| `--space-8` | `32px` | Maximum structural breathing room on empty/error internal views |

---

## 5. Radii, Borders, Shadows, and Iconography

### Radii (hierarchy over uniform pills)

| Token | Value | Usage |
| :--- | :--- | :--- |
| `--radius-none` | `0px` | Window outer frame (OS DWM handles native Windows 11 `8px` corner clipping), bottom/right webview edges, Windows caption buttons |
| `--radius-xs` | `4px` | `<kbd>` keycaps, inline status chips, favicon fallback tiles, compact tooltips, switch thumbs |
| `--radius-sm` | `6px` | Toolbar buttons, tab rows, address bar, context menu rows, text inputs, toggle switches, inner top-left viewport join |
| `--radius-md` | `8px` | Context menus, omnibox popover, find-in-page bar, settings group panels, Command Palette |

Pill radius (`9999px`) is forbidden on buttons, inputs, switches, tabs, cards, and address bars. Its only permitted use is a `6px` circular status dot when paired with a text or icon cue.

### Borders

- **Default structural seam:** `1px solid var(--border-subtle)` separates the `36px` top utility bar and left vertical rail from the content viewport, divides pinned tabs from session tabs, and separates rows inside settings panels.
- **Interactive field border:** `1px solid var(--border-control)` on text inputs, selects, toggle tracks, and the hovered address bar (`>= 3:1` boundary contrast).
- **Popover border:** every floating surface (`--bg-overlay`) carries `1px solid var(--border-subtle)` so popovers stay razor-sharp against dark web pages.
- **Active indicator bar:** a `2px` vertical `--accent-primary` bar marks the active tab and pre-selected omnibox/palette row.

### Shadows (elevation only, flat ground plane)

| Token | Dark value (Default) | Light value | Permitted surfaces |
| :--- | :--- | :--- | :--- |
| `--shadow-none` | `none` | `none` | Mandatory for top bar, sidebar, resting address bar, tabs, settings cards, and buttons |
| `--shadow-popover` | `0 6px 16px rgba(0, 0, 0, 0.42), 0 1px 2px rgba(0, 0, 0, 0.28)` | `0 4px 12px rgba(15, 23, 34, 0.10), 0 1px 2px rgba(15, 23, 34, 0.06)` | Context menus, anchored omnibox list, find-in-page bar, downloads/site-info popovers |
| `--shadow-modal` | `0 16px 40px rgba(0, 0, 0, 0.56), 0 2px 8px rgba(0, 0, 0, 0.36)` | `0 12px 32px rgba(15, 23, 34, 0.16), 0 2px 6px rgba(15, 23, 34, 0.08)` | Centered Command Palette (`Ctrl+K` / `Ctrl+T`) and modal confirmation dialogs |

Colored glow shadows are forbidden across all surfaces.

### Iconography

- **Grid & stroke:** `16px x 16px` viewBox (`1.5px` stroke, round caps/joins), pixel-snapped across `100%` to `200%` Windows DPI scales. Inline micro-icons (lock, mute, chevron, spinner arc) use `14px x 14px` (`1.5px` stroke).
- **Windows caption glyphs:** `10px` crisp `1px`-stroke glyphs matching Windows 11 Segoe Fluent Icons proportions.
- **Color & accessibility:** monochrome `currentColor` (`--fg-secondary` at rest, `--fg-primary` on hover/active). Every icon-only control carries an explicit `aria-label` plus shortcut tooltip, with `aria-hidden="true"` on the `<svg>`.
- **Forbidden glyphs:** sparkles, stars-as-magic, lightning bolts, glowing orbs, robot heads, decorative `->` / `↗` button arrows, and emoji characters anywhere in browser UI.

---

## 6. Browser Window Layout & Responsive Chrome

The window shell frames a native WebView2 content surface below a two-row `92px` horizontal header (`44px` Floating Pill Tab Strip + `48px` Rounded Navigation & Pill Omnibox Toolbar) and an optional collapsible `220px` left side panel (closed by default).

```text
+-----------------------------------------------------------------------------+
| (Tab 1)  (Tab 2)  [+]                (drag)                        [_][#][X]| 44px Row 1 (Tab Strip)
+-----------------------------------------------------------------------------+
| / [<] [>] [R]  ((o) Cari di Google atau masukkan URL        (copy)) [|][*] \| 48px Row 2 (Toolbar)
+------------------+----------------------------------------------------------+
| [Optional 220px  |                                                          |
|  Side Panel      |  Content Viewport (WebView2)                             |
|  Panel /         |  x: sidebar_w (0 default, 220 when open), y: 92px        |
|  Setelan]        |  w: win_w - sidebar_w, h: win_h - 92px                   |
+------------------+----------------------------------------------------------+
```

### Window bounds & WebView2 layering rules

- **Default / minimum window size:** `1280px x 800px` logical default; `520px x 360px` logical minimum so the top toolbar never crushes caption controls.
- **Resize borders:** when using a custom frameless titlebar on Windows 11, preserve native DWM resize hit-testing (`4px` to `6px` invisible resize border when windowed; `0px` when maximized or fullscreen so pixels never bleed onto adjacent monitors).
- **Content viewport rect:** occupies `x = sidebar_width`, `y = 92px`, `width = window_width - sidebar_width`, `height = window_height - 92px`.
- **Two-row Ruthenium Chrome (`44px` Row 1 + `48px` Row 2):**
  - **Row 1 (Floating Pill Tab Strip, `44px`):** Floating `12px`-radius pill tabs (`#080d1a` Midnight / `#e8eef6` Arctic / `#070913` Aurora frame; active pill in `#131d36` / `#ffffff` / `#151c38` with subtle border), `+` New Tab pill (`8px` radius), drag strip, and native Windows 11 caption controls (`44px x 38px` Minimize, Maximize/Restore, `#e11d48`-hover Close).
  - **Row 2 (Rounded Navigation & Pill Omnibox Toolbar, `48px`):** Rounded top-left/top-right toolbar (`16px` top radii, `#0d1426` / `#f8fafc` / `#0c1021`) containing Back (`Alt+Left`), Forward (`Alt+Right`), Reload/Stop (`Ctrl+R`/`Escape`), full-width `9999px`-radius Pill Omnibox with accent search/security badge and trailing Copy URL trigger, Side Panel toggle (`Ctrl+B`), and gear Settings trigger (`#btn-settings`).
  - **Collapsible Side Panel (`0px` default -> `220px` when toggled, `180px..320px` resizable):** Houses switchable views for `Panel` (Open Tabs showing website favicon + title + domain, and persistent `Catatan Cepat` scratchpad; bookmarks/Markah omitted) and `Setelan` (`Arctic` [Default] | `Midnight` | `Aurora` 3-theme selector + Default Search Engine selector); auto-closes cleanly below `640px` window width.
- **Overlay layering above WebView2:** because a child WebView2 renders in its own native `HWND` on Windows, transient overlays that overlap the content viewport (omnibox suggestion dropdown, Command Palette, context menus, Downloads popover, Find-in-Page bar) must render in an elevated host surface/webview layer above the content `HWND` (or temporarily reserve overlay hit bounds) so popovers are never clipped behind web page content.

### Auxiliary viewport overlays

- **Find-in-Page bar (`Ctrl+F`):** floats anchored at top-right of the content viewport (`top: 8px`, `right: 12px`, `height: 32px`, `width: 288px`, `--bg-overlay`, `1px solid var(--border-subtle)`, `--radius-md`, `--shadow-popover`). Contains query input, tabular match count (`0 / 0` or `2 / 9` in `--fg-muted` `--font-mono`), Previous (`Shift+Enter`), Next (`Enter`), and Close (`Escape`). Never pushes the webview downward.
- **Link hover URL preview:** when hovering an `<a href>` inside web content, show a single-line transient status pill at bottom-left of the viewport (`left: 8px`, `bottom: 8px`, `max-width: min(60vw, 480px)`, `height: 22px`, `padding: 0 8px`, `--bg-overlay`, `1px solid var(--border-subtle)`, `--radius-xs` `4px`, `ui-caption` `12px` `--fg-secondary`, middle-truncated so both scheme/domain and filename tail stay readable). Disappears immediately when pointer leaves the link.

### Responsive breakpoints (Windows 11 Snap Layouts)

| Window width | Sidebar behavior | Top utility bar behavior |
| :--- | :--- | :--- |
| `>= 960px` (Full / wide snap) | Expanded `220px` default (user-resizable `180px..320px`) | Centered Origin Pill (`width: min(46vw, 560px)`, `min-width: 260px`) with wide drag shoulders |
| `640px .. 959px` (Half-screen snap) | Auto-collapses to `44px` Compact Rail unless explicitly toggled open | Origin Pill flexes (`min-width: 180px`, `flex: 1 1 auto`), preserving `>= 24px` drag zones on both sides |
| `520px .. 639px` (Minimum / quarter snap) | `44px` Compact Rail (or `0px` hidden, summonable as overlay via `Ctrl+B`) | Forward button remains in place; Origin Pill shrinks down to `140px` showing registrable domain first |

---

## 7. Navigation Toolbar (`36px` Top Utility Bar)

The `36px` top bar is divided into four deterministic horizontal zones so controls never jump or crowd Windows caption buttons:

1. **Left Navigation Cluster (`padding-left: 6px`, `4px` gap, `app-region: no-drag`):**
   - **Sidebar Toggle (`Ctrl+B`):** `28px x 28px` button; `aria-expanded="true|false"`.
   - **Back (`Alt+Left`):** `28px x 28px`; disabled when back stack is empty; right-click opens tab back-history menu; middle-click or `Ctrl+Click` opens previous page in an adjacent background tab.
   - **Forward (`Alt+Right`):** `28px x 28px`; always remains rendered (never hides/shifts neighbors) and disables cleanly when forward stack is empty.
   - **Reload / Stop (`Ctrl+R` / `F5` / `Escape`):** `28px x 28px`; swaps to Stop (`X`) glyph while loading, updating `aria-label` between `"Reload page"` and `"Stop loading"`.
2. **Center Origin Zone + Drag Shoulders:**
   - Unoccupied horizontal stretches left and right of the Address Bar (`min-width: 24px` each) are native OS window drag zones (`app-region: drag`, double-click to maximize/restore).
3. **Right Utility Cluster (`4px` gap, `8px` right margin before caption zone, `app-region: no-drag`):**
   - Strictly capped at **at most two controls** to prevent toolbar bloat:
     1. **Downloads button (`Ctrl+J`):** hidden when zero downloads exist in the session; appears once a download starts (displaying a calm `16px` progress ring or completion indicator and opening the anchored Downloads popover on click).
     2. **Application Menu (`Alt+F`):** `28px x 28px` horizontal three-dot button opening the browser menu (`New tab`, `New window`, `Zoom - / 100% / +`, `Find in page`, `Downloads`, `Settings`, `About`).
4. **Windows 11 Caption Zone (`138px x 36px` total, flush top-right, `0px` radius):**
   - Three `46px x 36px` native-proportioned buttons: **Minimize**, **Maximize / Restore** (must expose native `HTMAXBUTTON` hit-testing or native frame hooks so hovering triggers the Windows 11 Snap Layouts flyout), and **Close** (`--win-close-hover-bg` `#C42B1C` hover, `--win-close-active-bg` `#B22518` active, `#FFFFFF` glyph, flush to top-right pixel for infinite corner targeting).

---

## 8. Tab Bar & Tab States

Tabs live in the left vertical rail by default so 20+ open tabs remain readable without shrinking into `24px` mystery slivers.

### Two-tier tab hierarchy

1. **Pinned Essentials (top zone):**
   - Compact responsive grid (`2` to `4` columns depending on rail width, `34px` tile height, `4px` gap, `--radius-sm` `6px`) at the top of the rail for persistent daily tools. In `44px` Compact Rail mode, collapses cleanly into a single vertical stack of `28px` tiles above the divider.
   - Pinned tabs survive restarts, omit the hover `X` button to prevent accidental closure, and support `Reset to pinned URL` and `Unpin tab` via context menu.
   - Separated from Session Tabs by a `1px solid var(--border-subtle)` horizontal rule (`6px` vertical margin).
2. **Session Tabs (scrollable middle zone) + Sticky New Tab Action:**
   - Vertical stack of `32px`-tall rows (`2px` vertical gap, `6px` horizontal rail inset).
   - **Row anatomy:** `[2px state bar] + [16px favicon / spinner / fallback slot] + [13px single-line title] + [20px audio/close slot]`.
   - **Sticky New Tab row:** anchored directly below the session list (scrolling with list until list overflows, then sticking cleanly above the sidebar footer with a `1px` top `--border-subtle` seam): `32px`-tall quiet button (`+ New tab` on left, `Ctrl+T` in `--fg-muted` `--font-mono` on right).

### Complete tab state matrix

| Tab state | Visual specification | Interaction contract |
| :--- | :--- | :--- |
| **Resting (Inactive)** | Transparent fill, `--fg-secondary` `400` weight title, close button hidden | Click activates; middle-click closes immediately |
| **Hovered (`:hover`)** | `--bg-subtle` fill, `--fg-primary` title, `20px x 20px` close (`X`) button revealed on right | `80ms` transition; tooltip shows full title + registrable domain after `400ms` |
| **Pressed (`:active`)** | `--bg-active` fill | Immediate tactile feedback without scale bounce |
| **Active (`aria-selected="true"`)** | `--bg-surface` fill, `1px solid var(--border-subtle)` perimeter, `2px` left `--accent-primary` bar, `--fg-primary` `500` weight title, close (`X`) visible | Distinguishable by luminance, border, weight, and `2px` bar without color vision |
| **Unread (Background-opened)** | Resting fill, `--fg-primary` `500` weight title + `4px` `--fg-muted` left dot until first selected | Signals a `Ctrl+Click` / middle-click tab that has finished loading unseen |
| **Audible / Muted** | `20px x 20px` speaker or muted-speaker toggle button in right slot (`--fg-primary` when audible, `--fg-muted` with slash when muted) | Remains visible at rest; clicking toggles mute without switching active tab; hover shifts close `X` alongside or swaps on modifier/context |
| **Loading** | Favicon slot shows `14px` monochrome rotating arc (`--accent-primary` on active tab, `--fg-secondary` on background tab) | Under `prefers-reduced-motion: reduce`, renders a static progress glyph |
| **Discarded (Memory-saved)** | Favicon at `0.55` opacity, title in `--fg-muted` | Reloads transparently when selected; keeps lightweight RAM footprint honest |
| **Crashed / Error** | `14px` `--status-danger` warning triangle in favicon slot | Selecting displays recovery interstitial (`Reload tab`) |

### Favicon fallback & tab reordering

- **Missing favicon fallback:** pages without a favicon (`about:blank`, `browser://settings`, local dev ports) render a `16px x 16px` `--bg-subtle` tile (`--radius-xs` `4px`) with a `12px` monochrome globe/settings glyph or deterministic 1-character uppercase domain initial in `--fg-secondary`. Never render broken image icons.
- **Drag-and-drop reordering:** dragging a tab vertically shows a crisp `2px` horizontal `--accent-primary` insertion line in the `2px` gap between rows; dragging across the Pinned separator pins or unpins the tab.
- **Last-tab close safeguard:** closing the final remaining session tab (`Ctrl+W` or row close `X`) releases that tab's WebView and opens a fresh `browser://newtab` rather than closing the browser window.

---

## 9. Address and Search Bar (Origin Pill & In-Place Omnibox)

Anchored permanently in the `36px` top utility bar (`height: 28px`) so users always see the verified origin without hunting for a hidden or teleporting field.

### Resting state (Origin-First Readout)

- **Surface:** `--bg-surface`, `1px solid var(--border-subtle)` (`--border-control` on `:hover`), `--radius-sm` (`6px`).
- **Left security trigger (`20px x 20px`):**
  - Valid `https://`: quiet `14px` lock glyph in `--fg-secondary` (`aria-label="View site information"`), opening a compact Site Info popover (full origin, TLS status, Clear site cookies/storage).
  - `browser://` internal pages: quiet `14px` app/sliders glyph + `Settings` / `New tab` label.
  - Insecure `http://` (except `localhost` / `127.0.0.1`): `14px` `--status-warning` warning glyph **plus** explicit `"Not secure"` chip (`11px`). `http://localhost` shows a neutral `localhost` monospace port readout without false alarmist red banners.
  - Broken TLS: `14px` `--status-danger` glyph **plus** `"Certificate error"` chip.
- **Origin & path formatting:** registrable domain (`eTLD+1`, e.g., `crates.io`) renders in `--fg-primary` (`500` weight), while trailing path (`/crates/wry`) renders in `--fg-muted` (`400` weight), truncated at the right edge with ellipsis so the registrable domain is **never** scrolled out of view on the left (anti-subdomain-spoofing rule).
- **Right contextual slot:** empty at rest unless zoom != `100%` (shows clickable `11px` `--font-mono` reset badge such as `125%`).

### Focused state (`Ctrl+L`, `Alt+D`, or click) & Search Disambiguation

- Expands smoothly *in place* within the `36px` top bar (up to `min(58vw, 640px)`), switches input text to `--font-mono` displaying the full raw URL (`https://crates.io/crates/wry`) selected in full, and applies a `2px solid var(--accent-primary)` outline.
- **Pointer vs caret behavior:** first click when unfocused selects all text; a second click places the text caret; `Ctrl+Shift+V` executes `Paste and go`.
- **Anchored suggestion popover (`4px` below address bar, max `8` rows at `36px` height):**
  - **Row 0 (Default `Enter` target, pre-selected with `--accent-subtle` + `2px` `--accent-primary` bar):** explicitly disambiguates URL navigation vs Search so the user never guesses what `Enter` will do:
    - *URL / host / localhost input (`docs.rs`, `localhost:5173`, `192.168.1.1`):* `16px` globe/arrow glyph + `docs.rs` (`--fg-primary`) + `Visit site` hint on right.
    - *Search query input (`rust wry webview2`):* `16px` search magnifier glyph + `rust wry webview2` (`--fg-primary`) + `Search <Engine>` hint (`--fg-muted`) on right.
  - **Rows 1..7:** ranked matches across **Open Tabs** (marked with a right-aligned `Switch to tab` chip), **Pinned Tabs**, and **Local History** (`[16px favicon] + [Page title in --fg-primary] + [domain/path in --fg-muted --font-mono]`).
  - **Keyboard modifiers:** `Enter` opens in current tab (or switches to matched open tab); `Alt+Enter` opens input/selection in a new active tab; `Escape` reverts unsubmitted edits and closes popover on first press, then returns focus to webview on second press.

---

## 10. Sidebar & Workspaces

The left vertical rail organizes tabs and optional workspaces without visual clutter or Arc-style window tinting.

### Three sidebar display modes

| Mode | Width | Trigger | Behavior |
| :--- | :--- | :--- | :--- |
| **Expanded (Default)** | `220px` (`180px..320px` resizable) | Default / `Ctrl+B` | Shows optional Workspace Switcher, Pinned Essentials, full Session Tab titles, sticky `+ New tab`, and `32px` footer |
| **Compact Rail** | `44px` | `Ctrl+B`, drag `< 120px`, or window `< 960px` | Collapses rows cleanly to centered `28px` favicon tiles (never clipping text mid-word); keeps `2px` `--accent-primary` active bar and exposes title + domain in a `120ms` right-side tooltip |
| **Focus Mode** | `0px` (`36px` top bar also hidden) | `Ctrl+Shift+F` | Dedicates `100%` of window pixels to WebView2; `Ctrl+L` or `Ctrl+K` opens Command Palette over the viewport without exiting Focus Mode |

- **Resize handle:** `4px` interactive hit strip along the right border (`cursor: col-resize`), highlighting the `1px` seam to `--accent-primary` while dragging; double-click resets width to `220px`.
- **Sidebar footer (`32px` height, `1px` top `--border-subtle` seam):** contains only real utility triggers: Workspace add/manage button (left) and Settings button (`Ctrl+,`, right). In `44px` Compact Rail mode, stacks or collapses cleanly to the single Settings icon.

### Workspaces (Progressive Disclosure)

- **Zero-overhead single workspace:** when only one default workspace exists, hide the top Workspace Switcher header completely so casual sessions pay `0px` vertical overhead. Users can create a second workspace via the sidebar footer, tab context menu, or Command Palette (`New workspace`).
- **Multi-workspace header (`28px` height, top of sidebar when `>= 2` workspaces exist):**
  - Displays `[14px monochrome workspace glyph or 1-char index] + [Workspace name in 13px 500 weight] + [tab count in 11px --fg-muted --font-mono] + [chevron]`.
  - Clicking or pressing `Ctrl+Shift+W` opens a compact popover listing workspaces (`Ctrl+Shift+1..9` switches directly), with inline `Rename`, `New workspace`, and `Delete workspace`.
- **Isolation & calm visual rule:** switching workspaces swaps the **Session Tabs** list in `< 80ms` (Pinned Essentials remain global by default unless scoped later). Workspaces **never** change `--bg-window`, `--accent-primary`, or apply colored borders/gradients to the browser frame.

---

## 11. Context Menus & Command Palette

### Context menu architecture

- **Container:** `min-width: 200px`, `max-width: 268px`, `padding: 4px`, `--bg-overlay`, `1px solid var(--border-subtle)`, `--radius-md` (`8px`), `--shadow-popover`. Clamps `4px` inside window edges and flips upward/leftward on edge collision.
- **Row (`28px` height, `0 8px` padding, `--radius-sm` `6px`):** sentence-case action label on left (`--fg-primary`, or `--status-danger` only on destructive actions), shortcut hint on right (`--fg-muted`, `--font-mono`, `11px`), `1px` `--border-subtle` group dividers (`4px` vertical margin), max `1` submenu level.
- **Four canonical context menus** (unimplemented items must be omitted until wired):
  1. **Tab Context Menu:** `Pin tab` / `Unpin tab`, `Duplicate tab`, `Mute tab` / `Unmute tab` | `Move to workspace >` (only when `>= 2` workspaces) | `Close tab` (`Ctrl+W`), `Close tabs below`, `Close other tabs`.
  2. **Address Bar Context Menu:** `Undo` | `Cut`, `Copy`, `Paste`, `Paste and go` (`Ctrl+Shift+V`) | `Select all`.
  3. **Toolbar / Rail Background Menu:** `New tab` (`Ctrl+T`), `Reopen closed tab` (`Ctrl+Shift+T`) | `Compact sidebar` (`Ctrl+B`), `Focus mode` (`Ctrl+Shift+F`) | `Settings` (`Ctrl+,`).
  4. **Web Content Context Menu:** keep native WebView2 or themed shell menu concise: `Back`, `Forward`, `Reload` | `Open link in new tab`, `Copy link address` | `Copy`, `Paste`, `Select all` | `Inspect` (`F12`).

### Command Palette (`Ctrl+K` / `Ctrl+T`)

- Centered overlay (`top: 14vh`, `width: min(90vw, 560px)`), `--bg-overlay`, `1px solid var(--border-subtle)`, `--radius-md` (`8px`), `--shadow-modal`, over a `rgba(8, 11, 16, 0.45)` scrim (`rgba(15, 23, 34, 0.22)` in Light Mode).
- Header has a `40px` input (`palette-input` `15px`) with subtle mode badge on the right (`New tab` when opened via `Ctrl+T`, `Command` when opened via `Ctrl+K`), followed by up to `8` `36px` results unifying:
  1. URL navigation or default search query (`Ctrl+T` opens target in a new tab).
  2. Fuzzy-jump across open tabs in all workspaces (`Switch` chip + workspace name).
  3. Browser actions (`Toggle sidebar`, `Focus mode`, `New workspace`, `Switch theme`, `Settings`, `Clear browsing data`, `Toggle DevTools`).
  4. Reopen recently closed tabs.

---

## 12. Browser Settings (`browser://settings`)

Settings open as a standard internal tab (`browser://settings`, triggered via `Ctrl+,` or Application Menu). Pressing `Ctrl+,` when a Settings tab is already open activates that existing tab instead of opening duplicates.

### Layout & surface rules

- **Two-column centered shell on `--bg-window` (`padding: 32px 24px`, `max-width: 840px`, centered):**
  - **Left category rail (`180px` width, sticky, `2px` row gap):** `32px`-tall category buttons using the same active treatment as tabs (`--bg-surface` + `2px` left `--accent-primary` bar + `500` weight). Ship only categories backed by real settings: `General`, `Appearance`, `Search`, `Privacy & data`, `Shortcuts`.
  - **Right settings column (`flex: 1`, `max-width: 600px`):** starts with `page-heading` (`18px` `600` weight) + compact filter input (`height: 32px`, `"Filter settings..."`, focused on `Ctrl+F`), followed by flat grouped panels (`--bg-surface`, `1px solid var(--border-subtle)`, `--radius-md` `8px`, `--shadow-none`) separated by `24px` vertical rhythm.
- **Setting row anatomy (`min-height: 52px`, `padding: 12px 16px`, `1px solid var(--border-subtle)` between sibling rows):**
  - **Left column:** `13px` `500`-weight setting name (`--fg-primary`) + optional `12px` single-line factual explanation (`--fg-muted`, `max-width: 46ch`).
  - **Right control slot (aligned middle-right, `16px` gap from text):**
    - **Segmented control (`28px` height, `--bg-window` track, `2px` inset, `--radius-sm` `6px`):** used for `Theme` (`Dark` | `Light` | `System`) and `Sidebar density` (`Expanded` | `Compact`); active segment gets `--bg-surface`, `1px solid var(--border-control)`, and `500` weight.
    - **Tactile rectangular toggle (`32px x 18px` track, `--radius-sm` `6px`, `12px x 12px` `--radius-xs` `4px` thumb):** off state uses `--bg-subtle` with `1px solid var(--border-control)`; on state uses `--accent-primary` fill with `--fg-on-accent` thumb (`role="switch"`, `aria-checked`). Avoids generic pill-shaped mobile switches.
    - **Select control (`height: 28px`, `min-width: 160px`, `--bg-subtle`, `1px solid var(--border-control)`, `--radius-sm` `6px`):** used for Default Search Engine (`Google`, `DuckDuckGo`, `Bing`).
- **Immediate persistence:** every change applies and persists immediately with zero `Save` / `Cancel` footer ceremony. Destructive operations (`Clear cookies and cache`, `Reset settings`) use a quiet `--bg-subtle` button (`--status-danger` text on confirmation modal only) requiring explicit confirmation.

---

## 13. Empty States, Loading States, and Error Views

### New Tab (`browser://newtab`) & Ruthenium Notes (`browser://blog`)

- **Canvas & Themes:** Three glassmorphic ambient themes (`Arctic Minimal` light default `#edf3fa`, `Midnight Ruthenium` `#080d1a`, and `Nebula Aurora` `#070913`) persisted in `localStorage` (`ruthenium_ntp_theme_v2` / `ruthenium_ntp_dark_preset_v1`) and synchronized bi-directionally with browser chrome settings via `window.__syncTheme`. WebView2 initializes dark viewport backgrounds to `(8, 13, 26, 255)` and light viewport backgrounds to `(248, 250, 252, 255)`.
- **Composition (`max-width: 672px` centered inside viewport):**
  - **Top Right Widget:** Real-time Indonesian digital clock (`HH:MM` with `tabular-nums`) and formatted local date (`weekday, day month`) inside a glass pill (`no` static weather or fake blocker status chips).
  - **Brand Header:** `72x72` hexagonal 3D `Facet R` emblem (`viewBox="0 0 120 120"`) with soft ambient glow and gentle float motion (guarded by `prefers-reduced-motion: no-preference`) paired with gradient `Ruthenium` display wordmark (`52px` `Space Grotesk`) and `Next-Gen Web Experience` subtitle.
  - **Merged Google Search Card (`border-radius: 9999px` -> `24px 24px 0 0` + `0 0 24px 24px`) & Live Google Suggestions:** Interactive search-engine switcher cycling `Google`, `DuckDuckGo`, and `Bing` (synced to host via `window.__newtabSetSearchEngine` and `window.__syncSearchEngine`), combobox search input (`/` focus shortcut) with Google-style `✕` clear button, recent search history on empty focus with `Hapus` / `Hapus semua riwayat` controls, `65ms` debounced live suggestions backed by `browser://localhost/suggest` (`suggestqueries.google.com`) with trailing `↖` query-refine button and `Arrow`/`Tab`/`Enter`/`Escape` keyboard support, plus Chrome-style inline type-ahead completion in the top omnibox.
  - **Shortcuts Grid:** Responsive 5-column `localStorage`-persisted shortcuts (`Ruthenium Notes`, `YouTube`, `GitHub`, `Gmail`, up to 10 items) with automatic website favicon resolution and clean letter-badge fallback, `+ Tambah` modal dialog (`Escape` closable), and per-shortcut remove action. No theme customizer button in the homepage footer (theme switching lives in the browser Settings panel).
- **Built-in `Ruthenium Notes` (`http://browser.localhost/blog` -> `browser://blog`):** offline release-notes card matching Light/Dark theme with a `"Kembali ke Tab Baru"` link back to `browser://newtab`.
- **Collection empty states (Downloads popover, History, Filtered Settings/Palette):** plain centered or left-aligned two-line text (`13px` `--fg-secondary` state + `12px` `--fg-muted` next action, e.g., `"No matching commands"` / `"Try a page title, URL, or browser action"`). Never render illustrations or fake sample items.

### Page loading states & anti-flash rules

1. **WebView2 background initialization (anti-white-flash):** every WebView2 instance must initialize its native background color (`WebViewBuilder::with_background_color`) to `--bg-window` (`(18, 22, 28, 255)` `#12161C` in Dark, `(242, 244, 247, 255)` `#F2F4F7` in Light) so creating a tab or navigating never flashes a bright `#FFFFFF` frame before page CSS paints.
2. **`120ms` anti-flicker threshold:** navigations completing in `< 120ms` suppress the viewport progress bar so fast local/cached loads feel instantaneous without visual strobing.
3. **In-flight load (`>= 120ms`):**
   - **Viewport seam bar:** a `2px` `--accent-primary` line animates smoothly along the top `1px` seam of the content viewport (`0% -> 85%` ease-out during network/DOM load; `85% -> 100%` over `100ms` on load finish, then `120ms` opacity fade-out).
   - **Tab & toolbar sync:** tab row displays the `14px` spinner arc in its favicon slot, and the top toolbar `Reload` button swaps to `Stop` (`X`).

### Navigation & TLS error interstitials

- Rendered inside the viewport on `--bg-window` using a left-aligned technical card (`max-width: 520px`, `padding: 20px`, `--bg-surface`, `1px solid var(--border-subtle)`, `2px` left `--status-danger` or `--status-warning` seam, `--radius-sm` `6px`):
  - `18px` plain factual heading (`"Cannot reach this site"` or `"Connection is not private"`).
  - `13px` `--fg-secondary` one-sentence cause naming the host (`"127.0.0.1 refused to connect on port 3000."`).
  - `12px` `--font-mono` `--fg-muted` raw error diagnostic (`ERR_CONNECTION_REFUSED`).
  - Action row (`8px` gap): primary `Retry` button (`--accent-primary` fill, `--fg-on-accent` label, activated on `Enter`) and secondary `Go back` button (`--bg-subtle`). No mascots, no jokes, and no apologies.

---

## 14. Keyboard Interaction

Every browser shell control must be reachable and operable without a pointer.

### Global Windows shortcuts

| Shortcut | Action |
| :--- | :--- |
| `Ctrl+L` or `Alt+D` | Focus and select the top Address Bar in place |
| `Ctrl+T` | Open Command Palette in New Tab mode (or focus New Tab input) |
| `Ctrl+K` or `Ctrl+Shift+P` | Open Command Palette in Universal Switch / Command mode |
| `Ctrl+N` | Open a new browser window |
| `Ctrl+W` or `Ctrl+F4` | Close active session tab (ignores or confirms on pinned tab) |
| `Ctrl+Shift+T` | Reopen most recently closed tab |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Cycle next / previous tab |
| `Ctrl+1` .. `Ctrl+8`, `Ctrl+9` | Activate tab `1..8`, or last tab (`Ctrl+9`) |
| `Ctrl+B` | Toggle left sidebar between Expanded and Compact Rail |
| `Ctrl+Shift+W` / `Ctrl+Shift+1..9` | Open Workspace Switcher / switch directly to workspace `1..9` |
| `Ctrl+Shift+F` | Toggle distraction-free Focus Mode |
| `Ctrl+F` / `F3` | Open Find-in-Page bar (or focus Settings filter on `browser://settings`) |
| `Ctrl+J` | Toggle Downloads popover |
| `Ctrl+,` | Open or switch to `browser://settings` |
| `Alt+Left` / `Alt+Right` | Navigate history Back / Forward |
| `Ctrl+R` or `F5` / `Ctrl+Shift+R` | Reload page / Reload bypassing cache |
| `Ctrl++` / `Ctrl+-` / `Ctrl+0` | Zoom webview in / out / reset to `100%` |
| `F12` or `Ctrl+Shift+I` | Toggle WebView2 DevTools |
| `Escape` | Close active menu/popover/palette/find bar and restore focus; otherwise stop page load |

### Focus order & roving tabindex

- Logical `Tab` order follows visual geometry: **Top Utility Bar (Sidebar toggle -> Back -> Forward -> Reload -> Address Bar -> Right utilities) -> Left Sidebar (Workspace Switcher if visible -> Pinned grid via arrow keys -> Session Tab list via roving `tabindex="0"` + `ArrowUp`/`ArrowDown` -> `+ New tab` -> Footer) -> WebView2 content surface**.
- Menus, omnibox suggestions, and Command Palette trap `ArrowUp` / `ArrowDown` / `Home` / `End`, activate on `Enter` (`Alt+Enter` for new tab where applicable), and close on `Escape` while returning focus to the exact originating element.

---

## 15. Hover, Active, Focus, and Disabled States

Every interactive element implements all five states distinctly and never relies on hue change alone.

| State | Visual & CSS contract |
| :--- | :--- |
| **Resting** | Transparent fill on chrome buttons/inactive tabs (`--bg-surface` on address bar, active tab, and settings panels); `cursor: pointer` set once in the base CSS layer on all enabled interactive elements (`button`, `a[href]`, `[role="tab"]`, `[role="switch"]`, `[role="menuitem"]`, `summary`, `label[for]`) |
| **Hover (`:hover`)** | Background shifts to `--bg-subtle`, foreground sharpens from `--fg-secondary` to `--fg-primary` (`80ms` transition) |
| **Pressed (`:active`)** | Background shifts to `--bg-active` (`60ms`); no bouncy `scale(0.92)` shrink effects |
| **Keyboard Focus (`:focus-visible`)** | `outline: 2px solid var(--accent-primary); outline-offset: -2px` (or `+1px` on standalone inputs/buttons so glyphs never clip), `>= 3.0:1` contrast against adjacent surfaces in both themes. Removing focus outlines (`outline: none`) without an explicit `:focus-visible` ring is forbidden |
| **Disabled (`[disabled]` / `aria-disabled="true"`)** | `opacity: 0.45`, `cursor: not-allowed`, hover/active fills suppressed, click handler inert |

---

## 16. Motion Principles

Governed by **`MOTION 1` (Calm / Functional)**: motion exists solely to confirm user input or clarify spatial origin, never to entertain.

- **Micro-states (hover / active):** `80ms linear` for color/background shifts; instant (`0ms`) on keyboard `ArrowUp`/`ArrowDown` list highlight so rapid key repeat feels zero-latency.
- **Popovers, Find bar, & Command Palette:** `120ms cubic-bezier(0.16, 1, 0.3, 1)` on open (`opacity: 0 -> 1` paired with `transform: translateY(-4px -> 0)`); `80ms ease-in` on close.
- **Sidebar width toggle (`Ctrl+B`):** `150ms cubic-bezier(0.2, 0, 0, 1)`, fully interruptible mid-transition. While dragging the resize handle with the pointer, disable transition (`0ms`) so the rail tracks the cursor `1:1` without lag.
- **Workspace switch:** `< 80ms` instant swap (no horizontal slide carousel).
- **Reduced motion:** wrap all non-essential transitions inside `@media (prefers-reduced-motion: no-preference)`, or collapse transition durations to `0.01ms` under `@media (prefers-reduced-motion: reduce)`.
- **Forbidden motion:** spring bounce/overshoot curves, stagger-fading tab rows on launch, parallax, pulsing status dots, and any animation running longer than `5s` without an explicit pause control.

---

## 17. Accessibility & System Resilience

1. **Contrast guarantees:** maintain `>= 4.5:1` for all normal UI text, `>= 3.0:1` for large text (`18px+`), and `>= 3.0:1` for interactive boundaries, toggle tracks, icons conveying state, and `:focus-visible` indicators across both Dark and Light themes.
2. **Windows High Contrast (`forced-colors: active`):** preserve `1px solid transparent` borders on buttons, tabs, switches, and popovers so Windows High Contrast themes automatically render crisp `ButtonText` / `Highlight` system outlines.
3. **Native semantics:** build actions with `<button type="button">`, landmarks with `<header>`, `<nav aria-label="Tabs">`, and `<main>`, tabs with `role="tablist"` / `role="tab"` / `aria-selected`, toggles with `role="switch"` / `aria-checked`, and dialogs/palettes with `<dialog>` or `role="dialog"` + `aria-modal="true"`. Clickable `<div>` or `<span>` wrappers without full ARIA role and keyboard handlers are forbidden.
4. **Non-color state signaling:** pair every semantic status (insecure HTTP, TLS failure, unread tab, muted tab, failed download, active tab) with an icon, border/indicator shape, or plain text label.
5. **DPI & text scaling:** all chrome layouts and `browser://settings` pages must hold up without clipped labels or overlapping controls across Windows display scales (`100%`, `125%`, `150%`, `175%`, `200%`).

---

## 18. Anti-Patterns: What Agents Must NOT Do

Future coding agents modifying or building UI in this repository must treat every item below as a hard build failure:

1. **Do not clone Arc, Zen, Chrome, Edge, Linear, or Vercel.** Never copy Arc's tinted gradient frames/Spaces bar or Zen's morphing floating URL bar.
2. **Do not add blue-to-purple gradients, neon cyan/acid-green accents, warm-terracotta AI defaults, radial background orbs, or background dot/grid wallpaper.**
3. **Do not stack glassmorphism (`backdrop-filter: blur`) across chrome surfaces.** The top bar, sidebar, and settings cards are solid matte mineral surfaces; solid `--bg-overlay` is used on menus and popovers for crisp contrast and WebView2 compositing performance.
4. **Do not move or hide Windows caption controls** in a way that breaks top-right corner Fitts's Law closing, window dragging (`app-region: drag`), or Windows 11 Maximize Snap Layouts. Never render macOS traffic-light dots on Windows.
5. **Do not let HTML popovers clip behind the WebView2 child `HWND`** or allow WebView2 to flash `#FFFFFF` white frames on Dark Mode startup.
6. **Do not make every element pill-shaped (`border-radius: 9999px`)** or apply soft floating drop shadows to tabs, buttons, settings panels, or toolbars.
7. **Do not ship dead or placeholder chrome buttons.** Every visible icon, menu item, setting control, or shortcut must execute real logic; unimplemented features (extensions, cloud sync, split-view, bookmarks manager) stay out of the UI until built.
8. **Do not invent fake browsing history, fake bookmarks, fake statistics, fake user profiles/avatars, wallpaper feeds, or AI marketing copy.** Empty lists stay honestly empty.
9. **Do not use AI-slop copywriting or decorative glyphs:** no em dashes, no emojis in UI, no sparkles/lightning icons, no ALL-CAPS tracked-out eyebrows, and no buzzwords (`Seamless`, `Next-Gen`, `AI-Powered`, `Revolutionary`, `Blazingly Fast`).
10. **Do not patch CSS or UI source files via external string-replacement scripts** (`.py`/`.js` regex patchers). Write clean, modular source styles directly.
