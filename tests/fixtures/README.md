# PDF Test Fixtures

Deterministic fixtures for CLI, renderer, and UI tests.

- `small.pdf`: 1 page
- `medium.pdf`: 5 pages
- `large.pdf`: 20 pages
- `invalid.pdf`: non-PDF text file
- `encrypted-marker.pdf`: synthetic encrypted marker fixture

Regenerate with:

```bash
python3 tests/fixtures/generate_fixtures.py
```
