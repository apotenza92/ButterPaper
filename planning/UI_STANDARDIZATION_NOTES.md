# UI Standardization Notes

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

### Why
- Reduce style drift between controls.
- Make future Zed-style UI alignment straightforward without copying GPL `ui` crate code.
- Keep interactions and visual states predictable across the app.

### Next
- Introduce a full reusable `tab_bar` container component (scroll/overflow/new-tab affordance) so `app/editor.rs` only owns tab lifecycle, not tab-bar layout.
- Migrate remaining inline editor/sidebar chrome styling blocks to shared primitives where equivalents exist.
- Add GPUI interaction tests (`#[gpui::test]`) for dropdown open/close, segmented/radio interaction, and tab overflow behavior.
