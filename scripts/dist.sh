#!/usr/bin/env bash
# dist: build the release binaries of the workspace and package them.
# `make dist` runs this for the host target. release-binaries.yml runs it per
# target with DIST_BUILDER=zigbuild. One script keeps the local proof and the
# CI packaging the same.
#
# Usage: dist.sh [--target TRIPLE] [--tag TAG] [--out DIR]
#   --target  Rust target triple. Default: the host triple.
#   --tag     Release tag in the file names. Default: v<workspace version>.
#   --out     Output directory. Default: dist.
# Env:
#   DIST_BUILDER  cargo (default) or zigbuild.
#   DIST_JOBS     cargo -j value. Default: 2.
#
# Output: <bin>-<tag>-<target>.tar.gz for each binary, and
# SHA256SUMS-<target>. Each archive holds the binary and LICENSE.
# The bin names come from `cargo metadata`, so renamed crates need no edit.
# The xtask crate is repo automation and is not shipped.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

target="$(rustc -vV | sed -n 's/^host: //p')"
tag=""
out="dist"
while [ $# -gt 0 ]; do
  case "$1" in
    --target) target="$2"; shift 2 ;;
    --tag) tag="$2"; shift 2 ;;
    --out) out="$2"; shift 2 ;;
    *) echo "dist: unknown argument: $1" >&2; exit 2 ;;
  esac
done
builder="${DIST_BUILDER:-cargo}"
jobs="${DIST_JOBS:-2}"

meta="$(cargo metadata --format-version 1 --no-deps --locked)"
if [ -z "$tag" ]; then
  version="$(jq -r '[.packages[] | select(.name != "xtask")][0].version' <<<"$meta")"
  tag="v$version"
fi
# One "package<TAB>bin" line for each bin target, except xtask.
mapfile -t bins < <(jq -r '
  .packages[] | select(.name != "xtask") | .name as $p
  | .targets[] | select(.kind | index("bin")) | "\($p)\t\(.name)"' <<<"$meta")
[ "${#bins[@]}" -gt 0 ] || { echo "dist: no bin crate found" >&2; exit 1; }

case "$builder" in
  cargo) build=(cargo build) ;;
  zigbuild) build=(cargo zigbuild) ;;
  *) echo "dist: bad DIST_BUILDER: $builder" >&2; exit 2 ;;
esac

rm -rf "$out"
mkdir -p "$out"
out="$(cd "$out" && pwd)"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT

for line in "${bins[@]}"; do
  pkg="${line%%$'\t'*}"
  bin="${line##*$'\t'}"
  "${build[@]}" --release --locked -j "$jobs" --target "$target" -p "$pkg" --bin "$bin"
  name="$bin-$tag-$target"
  mkdir -p "$stage/$name"
  cp "target/$target/release/$bin" LICENSE "$stage/$name/"
  tar -C "$stage" -czf "$out/$name.tar.gz" "$name"
done

(cd "$out" && sha256sum -- *.tar.gz > "SHA256SUMS-$target")
echo "dist: wrote to $out"
