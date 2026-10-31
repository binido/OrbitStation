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

# English unless the desktop runs in Russian, like the app itself
case "${LANGUAGE:-${LC_ALL:-${LC_MESSAGES:-${LANG:-}}}}" in ru*) RU=1 ;; *) RU= ;; esac
# say "English" "Русский"
say() { if [ -n "$RU" ]; then printf '%s\n' "$2"; else printf '%s\n' "$1"; fi; }
die() { say "Error: $1" "Ошибка: $2" >&2; exit 1; }
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
    say "Removed the program from $PREFIX." "Программа удалена из $PREFIX."
    if [ -f "$RULE" ]; then
        if as_root rm -f "$RULE"; then
            as_root udevadm control --reload 2>/dev/null || true
            say "Removed the udev rule." "Правило udev удалено."
        else
            say "Remove the udev rule by hand: $RULE" "Удали правило udev вручную: $RULE"
        fi
    fi
    say "Profiles and backups stay in ${XDG_DATA_HOME:-$HOME/.local/share}/orbitstation, delete them yourself if you do not need them." \
        "Профили и копии остались в ${XDG_DATA_HOME:-$HOME/.local/share}/orbitstation, удали их сам, если не нужны."
}

NO_UDEV=
case "${1:-}" in
    "") ;;
    --no-udev) NO_UDEV=1 ;;
    --uninstall) uninstall; exit 0 ;;
    *) die "unknown option $1, there are --no-udev and --uninstall" "неизвестный параметр $1, есть --no-udev и --uninstall" ;;
esac

[ "$(id -u)" -ne 0 ] || die "run it without sudo, as your own user: the program goes into your home folder" \
    "запусти без sudo, от своего пользователя: программа ставится в домашнюю папку"

have cargo || die "Rust is needed. Install it with: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" \
    "нужен Rust. Установи его так: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
rust=$(rustc --version | cut -d' ' -f2)
version_ok "$rust" || die "Rust $RUST_MIN or newer is needed, this is $rust. Update it: rustup update" \
    "нужен Rust $RUST_MIN или новее, сейчас $rust. Обнови: rustup update"
have cc || die "a C compiler is needed to build (the gcc or clang package)" "нужен компилятор C для сборки (пакет gcc или clang)"

# `sh install.sh` builds the checkout it sits in; piped (`curl | sh`, $0 is the shell) it always
# takes GitHub, so a project folder that happens to be the current directory is never picked up.
here=
case "$0" in
    *install.sh) [ -f "$0" ] && here=$(cd "$(dirname "$0")" && pwd) ;;
esac
if [ -n "$here" ] && grep -qs '^name = "orbitstation"' "$here/Cargo.toml"; then
    SRC=$here
    say "Building from $SRC" "Собираю из $SRC"
else
    have git || die "git is needed" "нужен git"
    if [ -d "$SRC/.git" ]; then
        say "Updating the sources in $SRC" "Обновляю исходники в $SRC"
        git -C "$SRC" pull --ff-only
    else
        say "Downloading the sources to $SRC" "Скачиваю исходники в $SRC"
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
Comment[ru]=Настройки геймпадов Flydigi
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
    say "The gamepad needs access: installing the udev rule to $RULE (asks for a password)" \
        "Нужен доступ к геймпаду: ставлю правило udev в $RULE (спросит пароль)"
    if as_root install -m 644 "$SRC/70-orbitstation.rules" "$RULE"; then
        as_root udevadm control --reload
        as_root udevadm trigger --subsystem-match=hidraw
        say "The udev rule is installed. If the gamepad was connected, reconnect it." \
            "Правило udev установлено. Если геймпад был подключён, переподключи его."
    else
        say "Could not do it without root. Run it yourself: sudo install -m 644 $SRC/70-orbitstation.rules $RULE" \
            "Не получилось без root. Выполни сам: sudo install -m 644 $SRC/70-orbitstation.rules $RULE"
    fi
fi

if [ -z "$old" ]; then say "Installed: $BIN" "Установлено: $BIN"
elif [ "$old" = "$(cksum < "$BIN")" ]; then say "Already the latest version: $BIN" "Уже последняя версия: $BIN"
else say "Updated: $BIN" "Обновлено: $BIN"
fi
case ":$PATH:" in
    *":$PREFIX/bin:"*) ;;
    *) say "$PREFIX/bin is not in PATH: start it from the app menu or by its full path." \
        "Папки $PREFIX/bin нет в PATH: запускай из меню приложений или по полному пути." ;;
esac
