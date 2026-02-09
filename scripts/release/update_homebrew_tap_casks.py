#!/usr/bin/env python3
"""Update Homebrew tap casks for ButterPaper from GitHub releases.

Semantics:
- Stable cask points at latest stable release tag vX.Y.Z.
- Beta cask points at beta-track target = max(latest stable, latest beta).
- Beta cask always uses beta-branded artifacts (ButterPaper-Beta-vX.Y.Z-*).
"""

from __future__ import annotations

import argparse
import dataclasses
import json
import re
import sys
import urllib.error
import urllib.request
from pathlib import Path
from typing import Iterable


REPO = "apotenza92/ButterPaper"
RELEASES_URL = f"https://api.github.com/repos/{REPO}/releases"
STABLE_TAG_RE = re.compile(r"^v(\d+)\.(\d+)\.(\d+)$")
BETA_TAG_RE = re.compile(r"^v(\d+)\.(\d+)\.(\d+)-beta\.(\d+)$")


@dataclasses.dataclass(frozen=True)
class SemVer:
    major: int
    minor: int
    patch: int
    prerelease: int | None

    @property
    def core(self) -> str:
        return f"{self.major}.{self.minor}.{self.patch}"


@dataclasses.dataclass(frozen=True)
class Release:
    tag_name: str
    prerelease: bool
    draft: bool
    semver: SemVer


def parse_tag(tag: str) -> SemVer | None:
    m = STABLE_TAG_RE.match(tag)
    if m:
        return SemVer(int(m.group(1)), int(m.group(2)), int(m.group(3)), None)

    m = BETA_TAG_RE.match(tag)
    if m:
        return SemVer(int(m.group(1)), int(m.group(2)), int(m.group(3)), int(m.group(4)))

    return None


def semver_key(s: SemVer) -> tuple[int, int, int, int, int]:
    # stable > prerelease for the same x.y.z
    is_stable = 1 if s.prerelease is None else 0
    prerelease_num = s.prerelease if s.prerelease is not None else 0
    return (s.major, s.minor, s.patch, is_stable, prerelease_num)


def fetch_releases() -> list[Release]:
    req = urllib.request.Request(
        RELEASES_URL,
        headers={
            "Accept": "application/vnd.github+json",
            "User-Agent": "butterpaper-homebrew-sync",
        },
    )

    try:
        with urllib.request.urlopen(req, timeout=20) as resp:
            payload = json.loads(resp.read().decode("utf-8"))
    except urllib.error.URLError as exc:
        raise RuntimeError(f"Failed to fetch releases: {exc}") from exc

    releases: list[Release] = []
    for item in payload:
        tag_name = item.get("tag_name", "")
        sem = parse_tag(tag_name)
        if not sem:
            continue
        releases.append(
            Release(
                tag_name=tag_name,
                prerelease=bool(item.get("prerelease", False)),
                draft=bool(item.get("draft", False)),
                semver=sem,
            )
        )

    return [r for r in releases if not r.draft]


def pick_latest(releases: Iterable[Release]) -> Release | None:
    releases = list(releases)
    if not releases:
        return None
    return max(releases, key=lambda r: semver_key(r.semver))


def render_stable_cask(version_core: str) -> str:
    return f'''cask "butterpaper" do
  version "{version_core}"
  sha256 :no_check

  on_arm do
    url "https://github.com/apotenza92/ButterPaper/releases/download/v#{{version}}/ButterPaper-v#{{version}}-macos-arm64.zip"
  end

  on_intel do
    url "https://github.com/apotenza92/ButterPaper/releases/download/v#{{version}}/ButterPaper-v#{{version}}-macos-x64.zip"
  end

  name "ButterPaper"
  desc "Rust-native desktop PDF app"
  homepage "https://github.com/apotenza92/ButterPaper"

  livecheck do
    url :url
    strategy :github_latest
  end

  app "ButterPaper.app"

  zap trash: [
    "~/Library/Application Support/ButterPaper",
    "~/Library/Caches/com.apotenza92.butterpaper",
    "~/Library/Preferences/com.apotenza92.butterpaper.plist",
    "~/Library/Saved Application State/com.apotenza92.butterpaper.savedState",
  ]
end
'''


def render_beta_cask(version_core: str) -> str:
    return f'''cask "butterpaper@beta" do
  version "{version_core}"
  sha256 :no_check

  on_arm do
    url "https://github.com/apotenza92/ButterPaper/releases/download/v#{{version}}/ButterPaper-Beta-v#{{version}}-macos-arm64.zip"
  end

  on_intel do
    url "https://github.com/apotenza92/ButterPaper/releases/download/v#{{version}}/ButterPaper-Beta-v#{{version}}-macos-x64.zip"
  end

  name "ButterPaper Beta"
  desc "Beta channel for ButterPaper"
  homepage "https://github.com/apotenza92/ButterPaper"

  livecheck do
    url "https://api.github.com/repos/apotenza92/ButterPaper/releases"
    strategy :json do |json|
      json
        .select {{ |release| release["prerelease"] && !release["draft"] }}
        .map {{ |release| release["tag_name"] }}
    end
  end

  app "ButterPaper Beta.app"

  zap trash: [
    "~/Library/Application Support/ButterPaper Beta",
    "~/Library/Caches/com.apotenza92.butterpaper.beta",
    "~/Library/Preferences/com.apotenza92.butterpaper.beta.plist",
    "~/Library/Saved Application State/com.apotenza92.butterpaper.beta.savedState",
  ]
end
'''


def write_if_changed(path: Path, content: str) -> bool:
    current = path.read_text() if path.exists() else None
    if current == content:
        return False
    path.write_text(content)
    return True


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--tap-path",
        type=Path,
        default=Path("/Users/alex/code/homebrew-tap"),
        help="Path to local homebrew-tap clone",
    )
    args = parser.parse_args()

    releases = fetch_releases()
    stable = pick_latest(r for r in releases if r.semver.prerelease is None)
    beta = pick_latest(r for r in releases if r.semver.prerelease is not None)

    if not stable and not beta:
        print("No stable or beta releases found; skipped cask update.")
        return 0

    beta_track = None
    if stable and beta:
        beta_track = stable if semver_key(stable.semver) >= semver_key(beta.semver) else beta
    else:
        beta_track = stable or beta

    assert beta_track is not None

    casks_dir = args.tap_path / "Casks"
    casks_dir.mkdir(parents=True, exist_ok=True)

    if stable:
        changed = write_if_changed(casks_dir / "butterpaper.rb", render_stable_cask(stable.semver.core))
        print(f"stable cask => v{stable.semver.core} ({'updated' if changed else 'unchanged'})")
    else:
        print("stable cask => unchanged (no stable release)")

    beta_changed = write_if_changed(
        casks_dir / "butterpaper@beta.rb",
        render_beta_cask(beta_track.semver.core),
    )
    source = "stable" if stable and beta_track.tag_name == stable.tag_name else "beta"
    print(
        f"beta cask => v{beta_track.semver.core} from {source} track "
        f"({'updated' if beta_changed else 'unchanged'})"
    )

    return 0


if __name__ == "__main__":
    sys.exit(main())
