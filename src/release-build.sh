#!/usr/bin/env bash

set -eux

REV=$(git rev-parse --short=9 HEAD)
ARCHIVE_NAME="aflak-$REV.tar.gz"
OS=linux64
DOWNLOAD_LINK="https://aflak-vis.github.io/download/build/$OS/$ARCHIVE_NAME"

cargo clean
echo '[profile.release]' >> Cargo.toml
echo 'lto = true' >> Cargo.toml
cargo build --release
strip target/release/aflak
tar cvf - -C target/release aflak | gzip --best > "$ARCHIVE_NAME"

rm -rf aflak-vis
git clone --depth 1 git@github.com:aflak-vis/aflak-vis.github.io.git aflak-vis
mkdir -p "aflak-vis/download/build/$OS"
mv "$ARCHIVE_NAME" "aflak-vis/download/build/$OS"
cd aflak-vis
git add "download/build/$OS/$ARCHIVE_NAME"
git commit -m "Release $REV for $OS"
git push

cd ..
sed -i 's|^- \[Linux\]\(.*\)$|- [Linux]('"$DOWNLOAD_LINK"')|' ../README.md
git add ../README.md
git commit -m "Release $REV for $OS"
