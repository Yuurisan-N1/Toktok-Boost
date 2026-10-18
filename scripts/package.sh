#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

NAME="Toktok-Boost"
VERSION="$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)"
STAGE="dist/$NAME"

echo "Toktok Boost packager"
echo "    version : $VERSION"
echo "    root    : $NAME/"
echo "    mode    : source only"

echo "Staging files"
rm -rf "$STAGE"
mkdir -p "$STAGE"

copy() {
    local src="$1" dest="${2:-}"
    if [[ ! -e "$src" ]]; then
        echo "    missing, skipped: $src" >&2
        return 0
    fi
    mkdir -p "$STAGE/$dest"
    cp -r "$src" "$STAGE/$dest/"
    echo "    ${dest:+$dest/}$(basename "$src")"
}

copy Cargo.toml
copy Cargo.lock
copy build.rs
copy Makefile
copy run.sh
copy config.json
copy data.txt
copy proxy.txt
copy README.md
copy HOWTO.txt
copy LICENSE
copy .gitignore
copy assets
copy src
copy scripts
copy .github

rm -rf "$STAGE/assets/captcha_samples"

chmod +x "$STAGE/run.sh" 2>/dev/null || true
chmod +x "$STAGE/scripts/"*.sh 2>/dev/null || true

echo "Writing archive"
mkdir -p dist
OUT="dist/${NAME}-${VERSION}-source.zip"
rm -f "$OUT"

if command -v zip >/dev/null 2>&1; then
    ( cd dist && zip -qr "$(basename "$OUT")" "$NAME" \
        -x "$NAME/target/*" -x "$NAME/dist/*" -x "$NAME/*.log" \
        -x '*__pycache__*' -x '*.pyc' -x '*/.DS_Store' )
else
    echo "    zip not found, falling back to python"
    python3 - "$STAGE" "$OUT" <<'PY'
import os, sys, zipfile
stage, out = sys.argv[1], sys.argv[2]
base = os.path.dirname(stage)
with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
    for root, _dirs, files in os.walk(stage):
        for f in files:
            full = os.path.join(root, f)
            z.write(full, os.path.relpath(full, base))
PY
fi

rm -rf "$STAGE"

SIZE="$(du -h "$OUT" | cut -f1)"
MD5="$(md5sum "$OUT" | cut -d' ' -f1)"
echo
echo "Done"
echo "    archive : $OUT"
echo "    size    : $SIZE"
echo "    md5     : $MD5"
