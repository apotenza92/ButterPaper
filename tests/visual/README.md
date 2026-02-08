# Visual Regression Tests

Visual regression uses screenshot baselines and committed hash/image artifacts per platform.

Editor toolbar visual flow:
- Capture candidates: `./scripts/capture_editor_toolbar_visuals.sh`
- Capture settings candidates: `./scripts/capture_settings_visuals.sh`
- Compare against baselines: `./scripts/compare_visuals.sh`
- Promote local candidates: `./scripts/promote_visual_baselines.sh`

Directory layout:
- Baselines: `tests/visual/baselines/{darwin,linux,windows}/editor/*.png`
- Candidates: `tests/visual/candidates/{darwin,linux,windows}/editor/*.png`
- Settings baselines: `tests/visual/baselines/{darwin,linux,windows}/settings/*.png`
- Settings candidates: `tests/visual/candidates/{darwin,linux,windows}/settings/*.png`
