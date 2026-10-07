#!/usr/bin/env bash
# //! 打 AppImage 包 target/linux-package/lightbookinput-<版本>-linux-<cpu>.AppImage：Server、Fcitx5 插件、产品数据与启动脚本。
# //! 产品数据先由 tools/release/data-fetch.sh 下载解出（CI 打包前跑），AppDir 结构按 files.py 的安装清单映射到 usr/ 下。
set -euo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)
cd "$root"

# CI 用 LIGHTBOOKINPUT_VERSION 覆盖成 1.0.20261006 这种「语义版本 + 日期」形式；本机 -dev 版接 git 短哈希
if [[ -n "${LIGHTBOOKINPUT_VERSION:-}" ]]; then
  version="$LIGHTBOOKINPUT_VERSION"
else
  version=$(sed -n 's/^version = "\(.*\)"/\1/p' apps/linux/server/Cargo.toml | head -1)
  if [[ "$version" == *-dev ]]; then
    version+="-$(git rev-parse --short=7 HEAD)"
    git diff --quiet HEAD || version+='+'
  fi
fi

# 数据与哈希校验由 tools/release/data-fetch.sh 负责（CI 在打包前调用），这里只确认文件在位
[[ -f data/models/hanzhang-zhiwei/hanzhang-zhiwei-small.qjm && -f data/models/hanzhang-tongbian/hanzhang-tongbian-small.qjm && -f data/generated/dict.qj ]] || { echo '缺少产品数据或含章模型，先运行 tools/release/data-fetch.sh' >&2; exit 1; }

cargo_output=$(realpath -m -- "${CARGO_TARGET_DIR:-$root/target}")
cargo build --release --locked -p lightbookinput-linux-server --target-dir "$cargo_output"
cmake_output="$root/target/fcitx5-appimage"
cmake -S apps/linux/fcitx5 -B "$cmake_output" -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=OFF
cmake --build "$cmake_output" --parallel "${CMAKE_BUILD_PARALLEL_LEVEL:-2}"

# appimagetool 的 ARCH 环境变量现在算好，避免在后面的管道子 shell 里丢失
arch=$(uname -m)
name="lightbookinput-$version-linux-$arch"
out="$root/target/linux-package"
appdir="$out/$name.AppDir"
rm -rf "$appdir"
mkdir -p "$appdir/usr/bin" "$appdir/usr/lib/fcitx5" "$appdir/usr/share/fcitx5/addon" "$appdir/usr/share/fcitx5/inputmethod" "$appdir/usr/share/icons/hicolor/128x128/apps" "$appdir/usr/share/licenses/lightbookinput" "$appdir/usr/share/lightbookinput/resources"

install -m 755 "$cargo_output/release/lightbookinput-linux-server" "$appdir/usr/bin/"
install -m 755 "$cmake_output/lightbookinput.so" "$appdir/usr/lib/fcitx5/"
install -m 644 apps/linux/fcitx5/data/addon/lightbookinput.conf "$appdir/usr/share/fcitx5/addon/"
install -m 644 apps/linux/fcitx5/data/inputmethod/lightbookinput.conf "$appdir/usr/share/fcitx5/inputmethod/"
install -m 644 icon.png "$appdir/usr/share/icons/hicolor/128x128/apps/lightbookinput.png"
install -m 644 LICENSE "$appdir/usr/share/licenses/lightbookinput/LICENSE"
install -m 644 assets/stroke/LICENSE-CNS11643.txt "$appdir/usr/share/licenses/lightbookinput/LICENSE-CNS11643.txt"

# 资源文件
for kind in sample glossary emoji; do
  cp -r assets/$kind "$appdir/usr/share/lightbookinput/resources/"
done

# 产品数据
cp -r data/generated "$appdir/usr/share/lightbookinput/resources/"
mkdir -p "$appdir/usr/share/lightbookinput/resources/models/hanzhang-zhiwei" "$appdir/usr/share/lightbookinput/resources/models/hanzhang-tongbian"
cp data/models/hanzhang-zhiwei/hanzhang-zhiwei-small.qjm "$appdir/usr/share/lightbookinput/resources/models/hanzhang-zhiwei/"
cp data/models/hanzhang-tongbian/hanzhang-tongbian-small.qjm "$appdir/usr/share/lightbookinput/resources/models/hanzhang-tongbian/"

# AppRun 启动脚本
cat > "$appdir/AppRun" <<'EOF'
#!/usr/bin/env bash
SELF=$(readlink -f "$0")
HERE=$(dirname "$SELF")
export LD_LIBRARY_PATH="$HERE/usr/lib:$LD_LIBRARY_PATH"
export PATH="$HERE/usr/bin:$PATH"
export XDG_DATA_DIRS="$HERE/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
exec "$HERE/usr/bin/lightbookinput-linux-server" "$@"
EOF
chmod +x "$appdir/AppRun"

# .desktop 文件
cat > "$appdir/lightbookinput.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=轻书
Name[en]=LightBookInput
Comment=跨平台输入法
Comment[en]=Cross-platform input method
Exec=lightbookinput-linux-server
Icon=lightbookinput
Terminal=false
Categories=Utility;
Keywords=input;method;pinyin;chinese;
EOF

# 根目录图标（AppImage 规范）
cp icon.png "$appdir/lightbookinput.png"

# 下载 appimagetool。CI runner 上没有 FUSE，挂不起 AppImage：先 --appimage-extract 解开到
# target/appimagetool/，之后直接跑里面的 AppRun（普通可执行文件，不再经过 AppImage 运行时）。
appimagetool_dir="$root/target/appimagetool"
if [[ ! -x "$appimagetool_dir/AppRun" ]]; then
  echo "下载 appimagetool…"
  appimage_bin="$root/target/appimagetool.AppImage"
  curl -fL --retry 3 -o "$appimage_bin" "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage"
  chmod +x "$appimage_bin"
  rm -rf "$appimagetool_dir" "$root/target/squashfs-root"
  (cd "$root/target" && "./appimagetool.AppImage" --appimage-extract >/dev/null)
  mv "$root/target/squashfs-root" "$appimagetool_dir"
  rm -f "$appimage_bin"
fi
"$appimagetool_dir/AppRun" --version

# 打包 AppImage
mkdir -p "$out"
appimage="$out/$name.AppImage"
rm -f "$appimage"
ARCH="$arch" "$appimagetool_dir/AppRun" --no-appstream "$appdir" "$appimage"

echo "已打包：$appimage"
ls -lh "$appimage"
