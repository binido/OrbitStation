#!/bin/sh
# Installs or updates OrbitStation for the current user, builds it from source with cargo.
#   sh install.sh               install, or update when already installed
#   sh install.sh --no-udev     skip the udev rule (needs root otherwise)
#   sh install.sh --uninstall   remove the program, the desktop entry and the udev rule
# PREFIX (default ~/.local) is where bin/ and share/applications/ go.
set -eu

REPO=https://github.com/binido/OrbitStation.git
RUST_MIN=1.95
PREFIX=${PREFIX:-$HOME/.local}
SRC=${ORBITSTATION_SRC:-${XDG_DATA_HOME:-$HOME/.local/share}/orbitstation/src}
RULE=/etc/udev/rules.d/70-orbitstation.rules
BIN=$PREFIX/bin/orbitstation
DESKTOP=$PREFIX/share/applications/orbitstation.desktop
ICONS=$PREFIX/share/icons/hicolor
ICON_SIZES="16 24 32 48 64 128 256"

say() { printf '%s\n' "$*"; }
die() { printf 'Ошибка: %s\n' "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

# sudo or doas for the one root step, empty when neither exists
as_root() {
    if [ "$(id -u)" -eq 0 ]; then "$@"
    elif have sudo; then sudo "$@"
    elif have doas; then doas "$@"
    else return 1
    fi
}

# 1.95 <= 1.99.1
version_ok() {
    printf '%s\n%s\n' "$RUST_MIN" "$1" | sort -t. -k1,1n -k2,2n -k3,3n -C
}

uninstall() {
    rm -f "$BIN" "$DESKTOP" "$ICONS/scalable/apps/orbitstation.svg"
    for s in $ICON_SIZES; do rm -f "$ICONS/${s}x$s/apps/orbitstation.png"; done
    have gtk-update-icon-cache && gtk-update-icon-cache -q -t "$ICONS" 2>/dev/null || true
    say "Программа удалена из $PREFIX."
    if [ -f "$RULE" ]; then
        if as_root rm -f "$RULE"; then
            as_root udevadm control --reload 2>/dev/null || true
            say "Правило udev удалено."
        else
            say "Удали правило udev вручную: $RULE"
        fi
    fi
    say "Профили и копии остались в ${XDG_DATA_HOME:-$HOME/.local/share}/orbitstation, удали их сам, если не нужны."
}

NO_UDEV=
case "${1:-}" in
    "") ;;
    --no-udev) NO_UDEV=1 ;;
    --uninstall) uninstall; exit 0 ;;
    *) die "неизвестный параметр $1, есть --no-udev и --uninstall" ;;
esac

[ "$(id -u)" -ne 0 ] || die "запусти без sudo, от своего пользователя: программа ставится в домашнюю папку"

have cargo || die "нужен Rust. Установи его так: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
rust=$(rustc --version | cut -d' ' -f2)
version_ok "$rust" || die "нужен Rust $RUST_MIN или новее, сейчас $rust. Обнови: rustup update"
have cc || die "нужен компилятор C для сборки (пакет gcc или clang)"

# The checkout this script sits in, otherwise a clone kept for later updates.
here=$(cd "$(dirname "$0")" 2>/dev/null && pwd) || here=
if [ -n "$here" ] && grep -qs '^name = "orbitstation"' "$here/Cargo.toml"; then
    SRC=$here
    say "Собираю из $SRC"
else
    have git || die "нужен git"
    if [ -d "$SRC/.git" ]; then
        say "Обновляю исходники в $SRC"
        git -C "$SRC" pull --ff-only
    else
        say "Скачиваю исходники в $SRC"
        mkdir -p "$(dirname "$SRC")"
        git clone --depth 1 "$REPO" "$SRC"
    fi
fi

old=
[ -x "$BIN" ] && old=$(cksum < "$BIN")
cargo build --release --locked --manifest-path "$SRC/Cargo.toml"

mkdir -p "$PREFIX/bin" "$PREFIX/share/applications"
# Copy to a temporary name first: replacing a running binary in place would crash it.
cp "$SRC/target/release/orbitstation" "$BIN.new"
chmod 755 "$BIN.new"
mv -f "$BIN.new" "$BIN"
cat > "$DESKTOP" <<EOF
[Desktop Entry]
Type=Application
Name=OrbitStation
Comment=Flydigi controller settings
Exec=$BIN
Icon=orbitstation
StartupWMClass=orbitstation
Terminal=false
Categories=Settings;HardwareSettings;
EOF
mkdir -p "$ICONS/scalable/apps"
cp "$SRC/assets/icons/orbitstation.svg" "$ICONS/scalable/apps/orbitstation.svg"
for s in $ICON_SIZES; do
    mkdir -p "$ICONS/${s}x$s/apps"
    cp "$SRC/assets/icons/orbitstation-$s.png" "$ICONS/${s}x$s/apps/orbitstation.png"
done
have gtk-update-icon-cache && gtk-update-icon-cache -q -t "$ICONS" 2>/dev/null || true
have update-desktop-database && update-desktop-database "$PREFIX/share/applications" 2>/dev/null || true

if [ -z "$NO_UDEV" ] && ! cmp -s "$SRC/70-orbitstation.rules" "$RULE"; then
    say "Нужен доступ к геймпаду: ставлю правило udev в $RULE (спросит пароль)"
    if as_root install -m 644 "$SRC/70-orbitstation.rules" "$RULE"; then
        as_root udevadm control --reload
        as_root udevadm trigger --subsystem-match=hidraw
        say "Правило udev установлено. Если геймпад был подключён, переподключи его."
    else
        say "Не получилось без root. Выполни сам: sudo install -m 644 $SRC/70-orbitstation.rules $RULE"
    fi
fi

if [ -z "$old" ]; then say "Установлено: $BIN"
elif [ "$old" = "$(cksum < "$BIN")" ]; then say "Уже последняя версия: $BIN"
else say "Обновлено: $BIN"
fi
case ":$PATH:" in
    *":$PREFIX/bin:"*) ;;
    *) say "Папки $PREFIX/bin нет в PATH: запускай из меню приложений или по полному пути." ;;
esac
