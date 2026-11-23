#!/usr/bin/env bash
# Build Kaptan packages into dist/:
#   kaptan_<ver>-1_amd64.deb             (cargo-deb)
#   kaptan-<ver>-1.x86_64.rpm            (rpmbuild, repackaging the release binary)
#   kaptan-<ver>-1-x86_64.pkg.tar.zst    (packaging/arch/mkpkg.py, no makepkg needed)
#   kaptan-<ver>.tar.gz                  (source tarball for rpmbuild -ba / makepkg)
# Usage: packaging/build-packages.sh [deb] [rpm] [arch] [src]   (default: all)
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT=$PWD
VER=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
DIST=$ROOT/dist
mkdir -p "$DIST"
export SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH:-$(stat -c %Y Cargo.toml)}
export PACKAGER=${PACKAGER:-"$(sed -n 's/^maintainer = "\(.*\)"/\1/p' Cargo.toml)"}

targets=("$@"); [ ${#targets[@]} -eq 0 ] && targets=(deb rpm arch src)
want() { [[ " ${targets[*]} " == *" $1 "* ]]; }

for f in packaging/rpm/kaptan.spec packaging/arch/PKGBUILD; do
    grep -qE "^(Version: +|pkgver=)$VER\$" "$f" || { echo "error: $f is not at version $VER" >&2; exit 1; }
done

if want deb || want rpm || want arch; then
    cargo build --release --locked
fi

if want deb; then
    cargo deb --no-build --output "$DIST/"
fi

if want rpm; then
    top=$(mktemp -d)
    rpmbuild -bb --quiet --nodeps \
        --define "_topdir $top" --define "_sourcedir $ROOT" --define "_kaptan_prebuilt 1" \
        --define "_binary_payload w19T0.zstdio" --define "use_source_date_epoch_as_buildtime 1" \
        packaging/rpm/kaptan.spec
    find "$top/RPMS" -name '*.rpm' -exec cp {} "$DIST/" \;
    rm -rf "$top"
fi

if want arch; then
    python3 -I packaging/arch/mkpkg.py "$ROOT" "$DIST" >/dev/null
fi

if want src; then
    tar --sort=name --mtime="@$SOURCE_DATE_EPOCH" --owner=0 --group=0 --numeric-owner \
        --exclude=./target --exclude=./dist --exclude=./.git --exclude='__pycache__' \
        --transform "s,^\.,kaptan-$VER," -czf "$DIST/kaptan-$VER.tar.gz" .
fi

cd "$DIST" && sha256sum -- *"$VER"* > SHA256SUMS && ls -l
