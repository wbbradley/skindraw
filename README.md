# SkinDraw

SkinDraw is a desktop editor for painting Minecraft-compatible player skins directly on a 3D
model. It supports classic and slim models, base and outer layers, exact brush-footprint previews,
body-part soloing, editable colors, and face-bounded flood fill.

## Install on Ubuntu

The Linux package supports amd64 Ubuntu 22.04 and newer. Download the `.deb` for the desired version
from the repository's GitHub Releases page, then install it with APT:

```bash
sudo apt install ./skindraw_0.1.0-1_amd64.deb
```

SkinDraw will appear in GNOME's application search. Installing a newer `.deb` upgrades the existing
installation. To remove it:

```bash
sudo apt remove skindraw
```

Application preferences remain specific to each desktop user and are stored in
`~/.local/state/skindraw.json`.

## Build from source

SkinDraw uses stable Rust. On Ubuntu 22.04, install the native build dependencies before building:

```bash
sudo apt-get update
sudo apt-get install -y \
  build-essential pkg-config \
  libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libxkbcommon-dev libssl-dev
cargo build --release --locked
```

The release workflow also installs `liblzma-dev` and the Debian, desktop-entry, XML, and SVG tools
used to build and validate the package. Runtime dependencies are derived by `cargo-deb`, recorded in
the workflow's package metadata report, and resolved by APT during installation.

## Package and release

To test packaging without publishing a release, manually run the **Package Linux** workflow in
GitHub Actions, or use the GitHub CLI:

```bash
gh workflow run release-linux.yml --ref main
```

The workflow produces a `skindraw-ubuntu-amd64` artifact containing the `.deb` and its metadata
report. Workflow artifacts are temporary; tagged packages are attached to GitHub Releases.

For a release, first update `version` in `Cargo.toml` and refresh `Cargo.lock`. Commit and push those
changes, then create and push the exactly matching tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The workflow rejects tags that do not equal `v` followed by the Cargo package version. A successful
tag build creates or updates the matching GitHub Release and attaches the `.deb`.

The initial packaging targets amd64 Ubuntu only. Arm64 packages, Flatpak, Snap, a PPA, and automatic
client updates are not currently provided.
