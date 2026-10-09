#!/usr/bin/env bash
# 把 LightBookInput.app 打包成 DMG（ad-hoc 签名、图标、Applications 快捷方式）。
#
#   scripts/build-dmg.sh            # 打包到 target/pkg/lightbookinput-<版本>-macos-<arm64|x86_64>.dmg
#
# 环境变量：
#   LIGHTBOOKINPUT_VERSION  版本号（如 1.0.20261006）
#   LIGHTBOOKINPUT_TARGET   目标架构（aarch64-apple-darwin 或 x86_64-apple-darwin）
#
# .app 由 scripts/bundle.sh 产出（含 ad-hoc 签名），本脚本只负责 DMG 包装：
# 两个架构的包交替打（bundle.sh 的 --pkg/--install 分支不会触发），互不覆盖。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
APP_NAME="LightBookInput"
APP="$ROOT/target/$APP_NAME.app"

# 目标架构
TARGET="${LIGHTBOOKINPUT_TARGET:-}"
case "${TARGET:-$(uname -m)}" in
  aarch64-apple-darwin|arm64) ARCH="arm64" ;;
  x86_64-apple-darwin|x86_64) ARCH="x86_64" ;;
  *) echo "不认识的架构: ${TARGET:-$(uname -m)}" >&2; exit 1 ;;
esac

# 版本号与 bundle.sh 同源：CI 用 LIGHTBOOKINPUT_VERSION 覆盖成 1.0.20261006，
# 否则读 apps/macos/Cargo.toml，-dev 版接 git 短哈希。
if [[ -n "${LIGHTBOOKINPUT_VERSION:-}" ]]; then
  VERSION="$LIGHTBOOKINPUT_VERSION"
else
  VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/apps/macos/Cargo.toml" | head -1)"
  if [[ "$VERSION" == *-dev ]]; then
    VERSION="${VERSION}-$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
  fi
fi

cd "$ROOT"

# .app 与 .pkg（含 cargo 构建、数据拷贝、ad-hoc 签名）交给 bundle.sh；版本号显式传进去保持一致。
# --pkg 模式同时产出 .app 和 .pkg，DMG 里放两者：普通用户双击 .pkg 安装，开发者可手动拖 .app。
LIGHTBOOKINPUT_VERSION="$VERSION" apps/macos/scripts/bundle.sh --pkg

# ---------- DMG 包装 ----------
DMG="$ROOT/target/pkg/lightbookinput-$VERSION-macos-$ARCH.dmg"
DMG_DIR="$ROOT/target/pkg/$ARCH-dmg"
rm -rf "$DMG_DIR"
mkdir -p "$DMG_DIR"

# 不带扩展属性复制，否则载荷里全是 ._ 元数据文件
ditto --noextattr --norsrc --noacl "$APP" "$DMG_DIR/$APP_NAME.app"

# PKG 安装器（普通用户双击安装到 /Library/Input Methods/）
PKG="$ROOT/target/pkg/lightbookinput-$VERSION-macos-$ARCH.pkg"
if [[ -f "$PKG" ]]; then
  cp "$PKG" "$DMG_DIR/"
fi

# Applications 快捷方式
ln -s /Applications "$DMG_DIR/Applications"

# 临时 DMG
TEMP_DMG="$ROOT/target/pkg/.tmp-$ARCH.dmg"
hdiutil create -volname "轻书输入法" -srcfolder "$DMG_DIR" -ov -format UDRW "$TEMP_DMG"

# 挂载 DMG 配置图标布局
MOUNT_POINT="$(hdiutil attach "$TEMP_DMG" -nobrowse -noverify | grep -o '/Volumes/.*' | head -1)"
if [[ -z "$MOUNT_POINT" || ! -d "$MOUNT_POINT" ]]; then
  echo "挂载 DMG 失败（$TEMP_DMG）" >&2
  hdiutil detach "$MOUNT_POINT" 2>/dev/null || true
  exit 1
fi

# 配置图标布局（通过 AppleScript）
osascript <<EOF
tell application "Finder"
  tell disk "轻书输入法"
    open
    set current view of container window to icon view
    set toolbar visible of container window to false
    set statusbar visible of container window to false
    set bounds of container window to {400, 100, 1085, 430}
    set viewOptions to icon view options of container window
    set arrangement of viewOptions to not arranged
    set icon size of viewOptions to 72
    set position of item "$APP_NAME.app" of container window to {100, 100}
    set position of item "lightbookinput-$VERSION-macos-$ARCH.pkg" of container window to {375, 100}
    set position of item "Applications" of container window to {650, 100}
    close
  end tell
end tell
EOF

# 设置 DMG 图标
cp "$APP/Contents/Resources/LightBookInput.icns" "$MOUNT_POINT/.VolumeIcon.icns"
hdiutil seticon "$MOUNT_POINT" "$APP/Contents/Resources/LightBookInput.icns" 2>/dev/null || true

# 卸载并压缩
hdiutil detach "$MOUNT_POINT"
hdiutil convert "$TEMP_DMG" -format UDZO -o "$DMG"
rm -f "$TEMP_DMG"

echo "DMG: $DMG"
shasum -a 256 "$DMG"
