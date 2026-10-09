# 图标

- `lightbookinput-mark.svg`：README 页头使用的透明竹简图标，与官网品牌图标一致。

应用图标的源文件在仓库根：`icon.png`（1024×1024，四角磨圆、圆角外透明）。README 页头、macOS 的 `LightBookInput.icns`、
Linux 的 hicolor 桌面图标都用它，改图只改这一份：

- `apps/macos/scripts/bundle.sh` 用 `sips` + `iconutil` 生成 `LightBookInput.icns`，生成物不进仓库。
- `apps/linux/scripts/files.py` 把它装到 `~/.local/share/icons/hicolor/128x128/apps/lightbookinput.png`；
  `apps/linux/scripts/build-appimage.sh` 另拷一份进 AppDir 根当 AppImage 规范要求的目录图标。
- `apps/windows/tsf/resources/lightbookinput.ico` 是同一个图样导出的多帧图标，帧位 16 / 24 / 32 / 48 / 64 / 128 / 256，
  每帧都从大图重新算圆角（不缩放 alpha），所以小尺寸下圆角不发虚。
- 圆角半径统一取 1024 画布的 1/9（约 112px），换成新图时按同一比例磨圆再导出上面的派生文件。
- `menu.svg`：macOS 输入法图标源文件，品牌雪山的单峰剪影（主峰取自应用图标 `icon.png`）：一座尖主峰加山顶一枚 V 形雪线缺口，
  没有实心底（模板图，系统只取 alpha，深浅色下反色成白色山形）。`menu.pdf` 是它导出的 22×16pt 矢量版，打包时拷成
  `lightbookinput-menu.pdf`，Info.plist 的图标键都指向它。为什么是这个形式和尺寸见 `docs/design/architecture.md`
  「Info.plist 约定」。峰形与缺口按 22×16 的实像素手调过：细于 2px 的发丝、圆弧和贴基线的深洞在这个尺寸下只会糊成
  毛边，或把底部切出细白腿，所以峰线一律取直、只留山顶一枚缺口，不要改成从 `icon.png` 自动提轮廓。改了 svg 重新导出：

  ```sh
  rsvg-convert -f pdf --page-width 22pt --page-height 16pt -w 22pt -h 16pt assets/icon/menu.svg -o assets/icon/menu.pdf
  ```
- `windows/mode-zh.svg` / `mode-en.svg` / `mode-caps.svg`：Windows 任务栏的中 / 英 / A 图标源文件（16×16 画布，单色）。
  `windows/render-mode-icons.sh` 用 rsvg-convert + magick 栅格化成 16 / 20 / 24 / 32 四档的 8 位 alpha 蒙版，
  写到 `apps/windows/tsf/resources/mode/`，DLL 用 `include_bytes!` 嵌入、运行时按任务栏深浅色填色（`com/mode/icon.rs`）。
  改了 svg 重跑脚本，生成物随仓库提交。
