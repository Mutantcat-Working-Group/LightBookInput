# 轻书 Windows 安装包

用 [NSIS](https://nsis.sourceforge.io/)（makensis）打的安装包，把 TSF DLL（64 位与 32 位各一份）、Server、设置程序与随包数据一起装进
`C:\Program Files\LightBookInput`，注册文本服务，并设登录自启。对应 macOS 的 pkg。

安装界面是简体中文，左下角显示「轻书输入法 \<版本\>」而不是 NullSoft 的署名。

## 安装布局

```
C:\Program Files\LightBookInput\
    lightbookinput_tsf-<版本>.dll       TSF 文本服务（64 位；被加载进每个应用进程；按版本起名，见「升级」）
    lightbookinput_tsf-<版本>-x86.dll   同上的 32 位版（企业微信 / WPS / 32 位 QQ 这类 32 位应用只能加载它）
    lightbookinput-server.exe       输入内核 Server（跑在应用进程外）
    lightbookinput-settings.exe     设置界面
    Microsoft.UI.Xaml.dll …   设置程序自带的 Windows App Runtime（自包含部署，见下节；约 56 MB / 185 个文件）
    lightbookinput.ico              开始菜单 / 启动项快捷方式的图标（exe 里也嵌了一份）
    data\generated\           dict.qj / lm.qj / english.tsv / dicts\*.qj
    data\generated\codes\     随包辅码码表 stroke.qj + LICENSE-CNS11643.txt（tools\dict-convert 的 pack codes 生成）
    assets\                   emoji\ glossary\ sample\
```

Server 与设置程序按 **exe 相对**定位随包资源（`lightbookinput_platform::resources`）：装机时资源与 exe 同级，
开发时是仓库 `ime\`（exe 在 `target\{debug,release}\` 下往上三层）。相对写法两套布局一致，只有根不同。

用户数据仍在 `%APPDATA%\LightBookInput`（config.toml、密钥 .env、学习数据、统计），三个进程的日志在 `%LOCALAPPDATA%\LightBookInput\logs`（`server.` / `tsf.` / `settings.` 前缀，按天，留 7 天）；
卸载不动这些。图标由 `regsvr32` 写到 `%ProgramData%\LightBookInput\lightbookinput.ico`（DLL 里 include_bytes 内嵌）。

## 安装程序做的几件事

1. **结束旧进程**：安装前 `taskkill` Server 与设置程序（只有这两个 exe 要覆盖）。
2. **应用容器权限**：`icacls` 给安装目录加 `ALL APPLICATION PACKAGES`（SID `*S-1-15-2-1`）读+执行。
   不加的话 UWP/AppContainer 应用（任务栏搜索、设置）读不到 DLL，切不到轻书。
3. **注册文本服务**：64 位 DLL 用 `regsvr32`、32 位 DLL 用 `SysWOW64\regsvr32`，各注册一次（各自写进自己视图的 HKCR，`CTF\TIP` 两边共用；要管理员——安装程序本就提权）。
4. **清旧 DLL**：装完删历次版本留下的 `lightbookinput_tsf*.dll`，仍被应用占用的登记成重启后删。
5. **登录自启**：「启动」文件夹放 Server 快捷方式（Explorer 走 ShellExecute 拉起才拿到 uiAccess；计划任务拿不到）。
6. **立即启动**：完成页以当前非提升用户 ShellExecute 起一次 Server，装完就能用，不必先注销。

卸载反向：杀 Server / 设置程序 → 反注册当前版本 DLL → 删文件（占用中的 DLL 重启后删，卸载段兜住旧版本的）。

## 升级：DLL 被占用怎么办

`lightbookinput_tsf.dll` 被加载进每一个有文本框的应用进程，文件锁着覆盖不了；NSIS 的 `RMDir /r` 会直接失败，
`INVDIR` 之类也无法让所有占用者主动退出——对输入法 DLL 就是「关掉一切」。所以关掉这类强制关闭，改成：

- DLL **按版本起名并排装**（`lightbookinput_tsf-<版本>.dll`），新文件从不与旧文件撞名；
- 只 `regsvr32` 新文件（InprocServer32 指向它）。**不要**对旧 DLL `regsvr32 /u`：那会把整个 CLSID / profile 注销掉；
- 已开着的应用继续用进程里的旧 DLL 直到重启，Server 两个版本都服务（`OpenSession` 带协议版本，对不上只记警告）；
- 装完删旧 DLL，删不掉的登记成重启后删。

## 设置程序自带 Windows App Runtime

设置界面用 Windows Reactor（WinUI 3）写，而它的框架依赖引导只有 Windows 11 走得通：要 Windows 11 才有的
AppModel API 把框架包加进进程包图，Windows 10 上没有那两个函数（定位见 `docs\notes\windows-win10.md`）。
所以设置程序用**自包含部署**——`apps\windows\settings\build.rs` 让 `windows-reactor-setup` 把 Windows App Runtime
铺到 `target\release\`，打包时按 `settings-runtime.txt` 挑进 `target\installer\settings-runtime-<架构>`，`lightbookinput.nsi`
再整个目录装到 `$INSTDIR` 下、与 `lightbookinput-settings.exe` 同级。

- 这些文件是运行时必需：少一件（或层级装错）设置窗口就起不来，`build-nsis.ps1` 发现缺文件会直接失败。
- 升级 `windows-reactor` / `windows-reactor-setup` 时，照新版 crate 的 `assets/runtime.txt` 核对 `settings-runtime.txt`。
- Server 与 TSF DLL 不依赖它；装机体积的大头仍是随包数据。
- `windows-reactor-setup` 在 `cargo build` 时用系统 `curl.exe` 从 NuGet 下运行时包（无校验，失败只打印），缓存在 `%LOCALAPPDATA%\windows-reactor-setup`；CI 的 runner 每次都会重下一遍。下载失败的后果由 `build-nsis.ps1` 的缺项检查兜住。

## 打包（在编译机上）

```powershell
# 需要 MSVC 工具链 + NSIS（makensis）。数据取自仓库 data\generated 与 assets，打包前先确保 .qj 是最新的。
powershell -ExecutionPolicy Bypass -File apps\windows\installer\build-nsis.ps1
```

脚本做的事：release 构建三件套（x64 与 i686 各一份）、挑出自包含运行时、生成自签名证书并签名四个源二进制、
用 makensis 编两个架构的安装包、再签名两个安装包，成品在 `target\installer\`：

```
lightbookinput-<版本>-windows-x86_64-setup.exe    64 位包（同时带 32 位 DLL）
lightbookinput-<版本>-windows-x86-setup.exe       32 位包
```

版本号默认取 `apps\windows\server\Cargo.toml`，`-dev` 版接 git 短哈希；CI 用环境变量 `LIGHTBOOKINPUT_VERSION`
覆盖成 `1.0.20261006` 这种「语义版本 + 日期」形式。`VIProductVersion` 只认 `a.b.c.d` 四点数字，
日期段 `20261006` 会被拆成 `2026.1006`，文件属性里仍看得到完整信息。

两个开关：

- `-SkipBuild`：跳过 `cargo build`（数据或 `.nsi` 改了、二进制没变时重编安装包用）。
- `-NoSign`：跳过自签名（本机没 Windows SDK 或只想先看打包流程时用）。

也可手动单编一个架构：

```powershell
& "C:\Program Files (x86)\NSIS\makensis.exe" /DAppVersion=1.0.0 /DAppVersionNumeric=1.0.0.0 /DBUILD_X64 apps\windows\installer\lightbookinput.nsi
```

## 注意

- **NSIS 版本**：开发机与 CI 都用 NSIS **3.x**（CI 上 `choco install nsis`）。`lightbookinput.nsi` 的安装界面文案由
  `MUI2` + `SimpChinese.nsh` 提供简体中文；`WinVer.nsh` 用来在 Windows 10 上给提示。别退回 2.x，MUI2 与多语言支持都不一样。
- **界面署名**：NSIS 缺省在左下角写 `NullSoft Install System v3.xx`，本脚本用 `BrandingText` 改成
  `轻书输入法 <版本>`（脚本里那段说明来自原 Inno 脚本，迁移时保留）。
- **UTF-8 BOM**：`lightbookinput.nsi` 必须以 **UTF-8 with BOM** 保存。NSIS 3 会把带 BOM 的 `.nsi` 当 UTF-8 解，
  中文文案才不会乱码；单引号与双引号里的字符串都会展开变量。
- **签名**：发版证书就绪后把 `signtool` 那一步换成正式证书（对应 mac 的 Developer ID）；开发期用 `-NoSign` 之外的自签证书。
  因为 `uiAccess` 要求 Server 签名 + 装 Program Files，而自签证书不被别的机器信任，所以 `build-nsis.ps1` 一上来就把
  `$env:LIGHTBOOKINPUT_UIACCESS` 置 `0`：对外分发的包 Server 不嵌 uiAccess，代价是候选窗在 UWP 宿主里可能被盖住。
  CI 的 `windows` job 也是这么打的。
- **安装器是 32 位进程**：页面里 `$SYSDIR` 指向 `SysWOW64`，注册表默认视图是 32 位的；所以 `.nsi` 在 `.onInit`
  （而非 `.onGUIInit`）里 `SetRegView 64`，32 位 DLL 才注册得进 64 位视图的 HKCR。

## 相关文档

- `docs\notes\windows-win10.md`：Windows 10 上的兼容性取舍与自包含运行时的来龙去脉。
- `..\..\README.md`：Windows 侧整体部署方式。
