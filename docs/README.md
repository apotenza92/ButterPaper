# ButterPaper Download Page

This folder powers the GitHub Pages download site for ButterPaper:

- https://apotenza92.github.io/ButterPaper/

## What lives here

- `index.html`: interactive download page
- `.nojekyll`: disables Jekyll processing
- `assets/`: page icons and static visuals

## Behavior

- Resolves release data from GitHub Releases API.
- Supports stable and beta channels.
- Beta channel targets the newest overall version (stable or beta).
- Falls back to deterministic `releases/latest/download/...` links if API lookup fails.

## Publishing

Enable GitHub Pages in repository settings:

1. Source branch: `main`
2. Folder: `/docs`

Changes to files in this directory publish automatically through GitHub Pages.
