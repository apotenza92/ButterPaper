# UI Standardization Notes

## 2026-02-08

### Completed
- Fixed settings sidebar navigation labels not rendering by restoring `nav_item(...)` label content in `crates/gpui-app/src/components/nav_item.rs` and standardizing selected/unselected/hover text colors.
- Added global danger semantics in `crates/gpui-app/src/theme.rs` by mapping theme `error`, `error.background`, and `error.border` into shared `ThemeColors` tokens (`danger`, `danger_bg`, `danger_border`) with deterministic fallbacks.
- Consolidated remaining shared visual literals into global UI tokens in `crates/gpui-app/src/ui.rs` and migrated consumers:
  - selected-surface alpha (`tab` + `chrome`),
  - scrollbar geometry/alpha (`components/scrollbar.rs`),
  - tab close radius/icon sizing (`components/tab.rs`),
  - thumbnail card width + transparent border helper (`sidebar.rs`),
  - left-cluster width/handle sizing (`app/editor.rs`, `ui_preferences.rs`).
- Updated `ButtonLikeVariant::Danger` in `crates/gpui-app/src/components/button_like.rs` to source colors from theme danger tokens instead of local hardcoded red values.
- Expanded visual tooling to include settings capture/comparison:
  - new `scripts/capture_settings_visuals.sh`,
  - updated `scripts/compare_visuals.sh` and `scripts/promote_visual_baselines.sh` for editor + settings suites.

## 2026-02-07

### Completed
- Added a shared `button_like` foundation in `crates/gpui-app/src/components/button_like.rs`.
- Refactored `button`, `icon_button`, and `text_button` to use the shared button-like contract.
- Unified settings dropdown rendering to the reusable `components::Dropdown` implementation.
- Updated checkbox visuals to use the same icon system (`Icon::Check`) and subtle border treatment.
- Added shared UI token helpers in `crates/gpui-app/src/ui.rs` (`BORDER_ALPHA_*`, `DISABLED_ALPHA`, `ui::color::*`).
- Added standardized primitives: `radio`, `segmented_control`, `slider`, `settings_row`, `settings_group`, `context_menu`, `popover_menu`, and `tab`.
- Migrated `SettingsView` layout to `settings_group` + `settings_row` composition and `segmented_control` for appearance mode selection.
- Migrated editor tab rendering to reusable `tab_item(...)` from `crates/gpui-app/src/components/tab.rs`.
- Fixed tab-close active-index behavior with deterministic selection logic and unit tests in `crates/gpui-app/src/app/editor.rs`.
- Added unit tests for slider clamping/step behavior and button-like color derivation contracts.
- Renamed desktop package id from `butterpaper-gpui` to `butterpaper` in `crates/gpui-app/Cargo.toml` (CLI remains `butterpaper-cli`).
- Refactored editor shell to fixed rows: title bar -> in-window menu row -> full-width tab row -> content row.
- Moved thumbnail lane below tab row and introduced a left tool rail with sidebar toggle (`Icon::PanelLeft`).
- Updated thumbnail rail polish: resizable left cluster (tool rail + thumbnails) with persisted width, neutral selected thumbnail border styling, and dedicated thumbnail toggle icon (`Icon::PageThumbnails`).
- Added compact canvas toolbar above viewport with page navigation (first/prev/next/last), fit-page/fit-width, and zoom controls.
- Added editable zoom combo behavior (`NNN%` input + preset dropdown) with deterministic parsing/clamping.
- Added new viewport APIs for deterministic reader controls: `first_page`, `last_page`, `fit_width`, `fit_page`, `reset_zoom`, and `set_canvas_metrics`.
- Added action and shortcut parity for reader controls (`ResetZoom`, `FitWidth`, `FitPage`, `FirstPage`, `LastPage`).
- Added icon assets/contracts for new shell controls (`panel_left`, `page_first`, `page_last`, `fit_width`, `fit_page`).
- Added Zed-style shared `ButtonSize` ladder (`Large/Medium/Default/Compact/None` => `32/28/22/18/16`) in `crates/gpui-app/src/components/button_like.rs`.
- Removed per-component size enums (`IconButtonSize`, `TextButtonSize`, `InputSize`) and migrated shared controls to `ButtonSize`.
- Added style foundation modules under `crates/gpui-app/src/styles/`:
  - `units.rs` (`rems_from_px`)
  - `spacing.rs` (`UiDensity`, `DynamicSpacing`)
  - `typography.rs` (`TextSize`)
- Added persisted UI preferences in `crates/gpui-app/src/ui_preferences.rs` for `AppearanceMode`, `ThemeSettings`, and `UiDensity`.
- Updated startup initialization in `crates/gpui-app/src/main.rs` to load persisted UI preferences instead of default-only globals.
- Added user-visible `UI Density` control in `crates/gpui-app/src/settings.rs` and wired persistence on appearance/theme/density changes.
- Standardized interactive control dimensions in editor and component surfaces using shared size tokens/constants (tab/menu/slider/toggle/radio/dropdown/context-menu).
- Added CI hard gate `scripts/check_control_sizes.sh` and wired it into `.github/workflows/ci.yml` to block new ad-hoc control sizing drift.
- Added sizing contract tests for button/input/text-button and spacing/rem helpers.
- Added a shared `open_editor_window(...)` window factory in `crates/gpui-app/src/main.rs` and wired `Application::on_reopen(...)` so macOS dock re-open creates a new editor window when none are open, matching native lifecycle expectations.
- Restored in-window app menu row on macOS and expanded both native + in-window menus to include `ButterPaper` and `File`, so macOS and in-app affordances now expose the same core menu entry points.
- Standardized in-window menu row spacing (`gap=0`, uniform horizontal padding) and added horizontal hover menu switching (when any menu is open) so moving across menu labels opens adjacent menus without re-clicking.
- Reworked tab-strip overflow behavior in `crates/gpui-app/src/app/editor.rs` + `crates/gpui-app/src/components/tab.rs`:
  - removed tab-title truncation so tabs grow to fit full titles,
  - replaced manual offset scrolling with native GPUI horizontal overflow tracking,
  - mapped wheel-driven scrolling through native horizontal overflow behavior,
  - kept `+` inline after the last tab until overflow, then pinned it at the right edge,
  - auto-revealed the active tab on open/switch/close/navigation,
  - preserved double-click-to-new-tab in available empty tab-strip space.
- Added shared editor chrome primitives in `crates/gpui-app/src/components/chrome.rs`:
  - `chrome_icon_button(...)` for compact icon actions with selected/disabled states.
  - `toolbar_group(...)` for pill-style grouped toolbar controls.
- Rebuilt the canvas toolbar in `crates/gpui-app/src/app/editor.rs` to grouped capsules (nav/fit/zoom), removed manual separator lines, and added persistent active fit-mode visual state based on viewport zoom mode.
- Added `PdfViewport::zoom_mode()` in `crates/gpui-app/src/viewport.rs` so toolbar rendering can reflect current fit mode deterministically.
- Removed UI density variants (`Compact/Default/Comfortable`) from runtime/settings/persistence:
  - dropped density globals from startup (`crates/gpui-app/src/main.rs`),
  - removed UI Density control from settings (`crates/gpui-app/src/settings.rs`),
  - removed persisted density field from preferences (`crates/gpui-app/src/ui_preferences.rs`),
  - simplified spacing model to fixed values in `crates/gpui-app/src/styles/spacing.rs`.
- Added editor visual regression scripts and CI plumbing:
  - `scripts/capture_editor_toolbar_visuals.sh`
  - `scripts/compare_visuals.sh`
  - `scripts/promote_visual_baselines.sh`
  - CI visual job now captures and compares committed baselines and uploads candidate artifacts on failure.
- Standardized typography usage across app/components with semantic helpers in `crates/gpui-app/src/ui.rs` (`TypographyExt`), replacing direct `text_xs/text_sm/text_base/text_xl` calls in UI surfaces (menu, toolbar, settings, context menu, sidebar, tabs, controls) to keep scale changes centralized.

### Why
- Reduce style drift between controls.
- Make future Zed-style UI alignment straightforward without copying GPL `ui` crate code.
- Keep interactions and visual states predictable across the app.

### Next
- Introduce a full reusable `tab_bar` container component (scroll/overflow/new-tab affordance) so `app/editor.rs` only owns tab lifecycle, not tab-bar layout.
- Migrate remaining inline editor/sidebar chrome styling blocks to shared primitives where equivalents exist.
- Add GPUI interaction tests (`#[gpui::test]`) for dropdown open/close, segmented/radio interaction, and tab overflow behavior.
