; 轻书（LightBookInput）Windows 输入法 NSIS 安装脚本。
;
; 由 apps/windows/installer/build-nsis.ps1 用 makensis /D 传入：
;   /DAppVersion=<版本>     如 1.0.20261006
;   /DAppVersionNumeric=<版> 如 1.0.2026.1006（VIProductVersion 只认四点数字）
;   /DRepo=<仓库根>         反斜杠形式
;   /DAppArch=x86_64|x86    决定输出文件名与 stage 目录
;   /DBUILD_X64             仅 64 位包定义
;
; 布局与升级策略移植自原 Inno 脚本：
;   * TSF DLL 按版本起名并排装（覆盖会因被各应用进程加载而失败），安装后删旧版本（删不掉的登记重启后删）；
;   * Server / 设置程序覆盖前先改名腾位再 taskkill；安装 / 卸载期间持全局互斥体 Global\LightBookInputInstaller，
;     新 DLL 看到它就不去拉 Server（见 tsf 的 launch.rs）；
;   * 完成页勾选「运行轻书」即起一次 Server（uiAccess=0，直接 Exec 即可）；
;   * icacls 给安装目录授予 ALL APPLICATION PACKAGES 读+执行；regsvr32 双视图注册文本服务。
; 64/32 位 regsvr32：安装器恒为 32 位进程，$SYSDIR 在 64 位系统上被重定向到 SysWOW64（正好是 32 位 regsvr32），
; 真 64 位程序用 $WINDIR\Sysnative\...。

!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"
!include "WinVer.nsh"

; ---------- 版本与路径（打包脚本传入，缺省值便于本地试编） ----------
!ifndef AppVersion
  !define AppVersion "1.0.0"
!endif
!ifndef AppVersionNumeric
  !define AppVersionNumeric "1.0.0.0"
!endif
!ifndef Repo
  !define Repo "..\..\.."
!endif
!ifndef AppArch
  !define AppArch "x86_64"
!endif

!define AppName "轻书"
!define AppNameEN "LightBookInput"
!define ProductName "${AppName} ${AppNameEN}"
!define Publisher "Mutantcat Working Group"
!define WebsiteUrl "https://lightbookinput.app"
!define RegKey "Software\LightBookInput"
!define UninstKey "Software\Microsoft\Windows\CurrentVersion\Uninstall\LightBookInput"

; ---------- 架构相关 ----------
; 64 位包：64 位 DLL 用 <版本>.dll，另带 32 位 <版本>-x86.dll；
; 32 位包：只装一个 32 位 <版本>.dll。
!ifdef BUILD_X64
  !define TsfDll64 "lightbookinput_tsf-${AppVersion}.dll"
  !define TsfDll32 "lightbookinput_tsf-${AppVersion}-x86.dll"
!else
  !define TsfDll32 "lightbookinput_tsf-${AppVersion}.dll"
!endif
; build-nsis.ps1 铺好的暂存目录（二进制 / 图标 / 自包含运行时），已按上表改名好 DLL。
!define StageDir "${Repo}\target\installer\stage-${AppArch}"

; ---------- 安装器属性 ----------
Name "${ProductName}"
OutFile "${Repo}\target\installer\lightbookinput-${AppVersion}-windows-${AppArch}-setup.exe"
!ifdef BUILD_X64
InstallDir "$PROGRAMFILES64\LightBookInput"
!else
InstallDir "$PROGRAMFILES\LightBookInput"
!endif
InstallDirRegKey HKLM "${RegKey}" "InstallDir"
; 左下角显示软件信息与版本号（取代默认的 NullSoft Install System ...）
BrandingText "轻书 LightBookInput ${AppVersion}"
RequestExecutionLevel admin

VIProductVersion "${AppVersionNumeric}"
VIAddVersionKey "ProductName" "${ProductName}"
VIAddVersionKey "CompanyName" "${Publisher}"
VIAddVersionKey "LegalCopyright" "Copyright (C) Mutantcat Working Group"
VIAddVersionKey "FileDescription" "${ProductName} 安装程序 ${AppVersion}"
VIAddVersionKey "FileVersion" "${AppVersion}"
VIAddVersionKey "ProductVersion" "${AppVersion}"

SetCompressor /SOLID lzma

; ---------- 界面：中文 UI ----------
!define MUI_ABORTWARNING
!define MUI_ICON "${Repo}\apps\windows\tsf\resources\lightbookinput.ico"
!define MUI_UNICON "${Repo}\apps\windows\tsf\resources\lightbookinput.ico"
!define MUI_FINISHPAGE_RUN
!define MUI_FINISHPAGE_RUN_FUNCTION LaunchServer

!define MUI_TEXT_WELCOME_INFO_TITLE "欢迎安装轻书 ${AppVersion}"
!define MUI_TEXT_WELCOME_INFO_TEXT "本向导将引导你安装轻书（LightBookInput）${AppVersion}。$\r$\n$\r$\n轻书是一款跨平台中文输入法，安装前请保存好其它程序里的工作。$\r$\n$\r$\n点击「下一步」继续。"
!define MUI_TEXT_DIRECTORY_TITLE "选择安装位置"
!define MUI_TEXT_DIRECTORY_SUBTITLE "选择轻书的安装文件夹。"
!define MUI_TEXT_INSTALLING_TITLE "正在安装轻书"
!define MUI_TEXT_INSTALLING_SUBTITLE "正在安装 ${ProductName} ${AppVersion}，请稍候…"
!define MUI_TEXT_FINISH_TITLE "安装完成"
!define MUI_TEXT_FINISH_SUBTITLE "轻书 ${AppVersion} 已安装到你的电脑。"
!define MUI_TEXT_FINISH_INFO_TEXT "安装完成。请注销后重新登录（或重启电脑），轻书才会在所有应用里生效。$\r$\n$\r$\n不方便注销的话，先关掉再重新打开要打字的应用也可以。"
!define MUI_TEXT_FINISH_RUN "运行轻书(&R)"
!define MUI_TEXT_ABORTWARNING "确定要退出轻书安装程序吗？"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

; 简体中文 + English；NSIS 按用户系统 UI 语言自动选择，无需语言选择页
!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

; ---------- 完成页：运行轻书 ----------
Function LaunchServer
  Exec '"$INSTDIR\lightbookinput-server.exe"'
FunctionEnd

; ---------- 辅助函数 ----------
; 安装 / 卸载期间持有互斥体（名字与 tsf launch.rs 一致），旧 DLL 一时拉不起 Server；句柄不关，进程退出系统回收。
Function HoldInstallerMutex
  System::Call 'kernel32::CreateMutexW(p 0, b 0, w "Global\LightBookInputInstaller") p.r0'
FunctionEnd

; 运行中的 exe 覆盖不了但能改名：改成 .old.exe 腾出名字，旧 DLL 就拉不起它；装完 / 卸载时删。
Function RetireServerExe
  IfFileExists "$INSTDIR\lightbookinput-server.exe" 0 rse_done
    IfFileExists "$INSTDIR\lightbookinput-server.old.exe" 0 +2
      Delete /REBOOTOK "$INSTDIR\lightbookinput-server.old.exe"
    Rename "$INSTDIR\lightbookinput-server.exe" "$INSTDIR\lightbookinput-server.old.exe"
  rse_done:
FunctionEnd

; 同版本重装：目标 DLL 被某进程加载、覆盖不了，改名成 .dll.old 腾位；装完删（删不掉登记重启后删）。
Function RetireLoadedDll
  Pop $0
  IfFileExists "$INSTDIR\$0" 0 rld_done
    IfFileExists "$INSTDIR\$0.old" 0 +2
      Delete /REBOOTOK "$INSTDIR\$0.old"
    Rename "$INSTDIR\$0" "$INSTDIR\$0.old"
  rld_done:
FunctionEnd

; taskkill 结束进程（用 32 位 taskkill 即可，杀进程与位宽无关）。参数为映像名。
Function KillByName
  Pop $0
  nsExec::Exec '"$SYSDIR\taskkill.exe" /im $0 /f'
  Pop $1
FunctionEnd

; 安装后清掉历次升级留下的旧版本 DLL（保留当前版本）。
Function DeleteStaleDlls
  Push $0
  Push $1
  FindFirst $0 $1 "$INSTDIR\lightbookinput_tsf*.dll"
  dsd_loop:
    StrCmp $1 "" dsd_done
    StrCmp $1 "${TsfDll32}" dsd_next
  !ifdef BUILD_X64
    StrCmp $1 "${TsfDll64}" dsd_next
  !endif
    Delete /REBOOTOK "$INSTDIR\$1"
  dsd_next:
    FindNext $0 $1
    Goto dsd_loop
  dsd_done:
    FindClose $0
  Pop $1
  Pop $0
FunctionEnd

; 删掉更早版本建的登录自启计划任务（现在改用「启动」文件夹快捷方式）。
Function DeleteLegacyLogonTask
  nsExec::Exec '"$SYSDIR\schtasks.exe" /delete /tn "LightBookInput Server" /f'
  Pop $0
FunctionEnd

; ---------- 卸载器用的同名函数（un. 前缀，安装与卸载的函数表不相通） ----------
Function un.HoldInstallerMutex
  System::Call 'kernel32::CreateMutexW(p 0, b 0, w "Global\LightBookInputInstaller") p.r0'
FunctionEnd

Function un.RetireServerExe
  IfFileExists "$INSTDIR\lightbookinput-server.exe" 0 un_rse_done
    IfFileExists "$INSTDIR\lightbookinput-server.old.exe" 0 +2
      Delete /REBOOTOK "$INSTDIR\lightbookinput-server.old.exe"
    Rename "$INSTDIR\lightbookinput-server.exe" "$INSTDIR\lightbookinput-server.old.exe"
  un_rse_done:
FunctionEnd

Function un.KillByName
  Pop $0
  nsExec::Exec '"$SYSDIR\taskkill.exe" /im $0 /f'
  Pop $1
FunctionEnd

Function un.DeleteLegacyLogonTask
  nsExec::Exec '"$SYSDIR\schtasks.exe" /delete /tn "LightBookInput Server" /f'
  Pop $0
FunctionEnd

; ---------- 安装器初始化 ----------
Function .onInit
  Call HoldInstallerMutex
  SetShellVarContext all
  ${IfNot} ${AtLeastBuild} 17763
    MessageBox MB_OK|MB_ICONSTOP "轻书需要 Windows 10 版本 1809 或更高版本。"
    Abort
  ${EndIf}
!ifdef BUILD_X64
  ${IfNot} ${RunningX64}
    MessageBox MB_OK|MB_ICONSTOP "这是轻书 64 位安装包，需要 64 位 Windows。请改用 32 位安装包。"
    Abort
  ${EndIf}
  SetRegView 64
!endif
FunctionEnd

; ---------- 安装 ----------
Section "安装" SecMain
  SectionIn RO

  ; 1) 覆盖前腾位并结束旧实例（互斥体已持，旧 DLL 不会把它拉回来）
  Call RetireServerExe
  Push "lightbookinput-server.exe"
  Call KillByName
  Push "lightbookinput-settings.exe"
  Call KillByName
  Push "${TsfDll32}"
  Call RetireLoadedDll
!ifdef BUILD_X64
  Push "${TsfDll64}"
  Call RetireLoadedDll
!endif

  ; 2) 落文件：图标 / 设置程序 / 自包含运行时 / DLL / 数据 / 资源，Server 排最后
  SetOutPath "$INSTDIR"
  File "${StageDir}\lightbookinput.ico"
  File "${StageDir}\lightbookinput-settings.exe"
  File /r "${StageDir}\settings-runtime\*"
  File "${StageDir}\${TsfDll32}"
!ifdef BUILD_X64
  File "${StageDir}\${TsfDll64}"
!endif

  SetOutPath "$INSTDIR\data\generated"
  File "${Repo}\data\generated\dict.qj"
  File "${Repo}\data\generated\lm.qj"
  File "${Repo}\data\generated\glossary-en.qj"
  File "${Repo}\data\generated\glossary-ja.qj"
  File "${Repo}\data\generated\glossary-zh.qj"
  File "${Repo}\data\generated\glossary-es.qj"
  File /nonfatal "${Repo}\data\generated\english.tsv"
  SetOutPath "$INSTDIR\data\generated\dicts"
  File /nonfatal /r "${Repo}\data\generated\dicts\*.qj"
  SetOutPath "$INSTDIR\data\generated\codes"
  File /nonfatal "${Repo}\data\generated\codes\*.qj"
  File "${Repo}\assets\stroke\LICENSE-CNS11643.txt"
  SetOutPath "$INSTDIR\data\models\hanzhang-zhiwei"
  File "${Repo}\data\models\hanzhang-zhiwei\hanzhang-zhiwei-small.qjm"
  SetOutPath "$INSTDIR\data\models\hanzhang-tongbian"
  File "${Repo}\data\models\hanzhang-tongbian\hanzhang-tongbian-small.qjm"

  SetOutPath "$INSTDIR\assets\emoji"
  File "${Repo}\assets\emoji\emoji-zh.tsv"
  File "${Repo}\assets\emoji\emoji-en.tsv"
  SetOutPath "$INSTDIR\assets\levels"
  File "${Repo}\assets\levels\levels-en.tsv"
  File "${Repo}\assets\levels\levels-ja.tsv"
  SetOutPath "$INSTDIR\assets\wubi"
  File "${Repo}\assets\wubi\wubi86.tsv"
  SetOutPath "$INSTDIR\assets\sample"
  File "${Repo}\assets\sample\dict.tsv"

  ; Server 放最后：一落地旧版 DLL 就能把它拉起来占住数据
  SetOutPath "$INSTDIR"
  File "${StageDir}\lightbookinput-server.exe"

  ; 3) 注册表与卸载信息
  WriteRegStr HKLM "${RegKey}" "InstallDir" "$INSTDIR"
  WriteRegStr HKLM "${RegKey}" "Version" "${AppVersion}"
  WriteRegStr HKLM "${UninstKey}" "DisplayName" "${ProductName}"
  WriteRegStr HKLM "${UninstKey}" "DisplayVersion" "${AppVersion}"
  WriteRegStr HKLM "${UninstKey}" "Publisher" "${Publisher}"
  WriteRegStr HKLM "${UninstKey}" "URLInfoAbout" "${WebsiteUrl}"
  WriteRegStr HKLM "${UninstKey}" "DisplayIcon" "$INSTDIR\lightbookinput.ico"
  WriteRegStr HKLM "${UninstKey}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "${UninstKey}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegDWORD HKLM "${UninstKey}" "NoModify" 1
  WriteRegDWORD HKLM "${UninstKey}" "NoRepair" 1
  WriteUninstaller "$INSTDIR\uninstall.exe"

  ; 4) 快捷方式（所有用户开始菜单 + 登录「启动」）
  CreateDirectory "$SMPROGRAMS\LightBookInput"
  CreateShortcut "$SMPROGRAMS\LightBookInput\轻书设置.lnk" "$INSTDIR\lightbookinput-settings.exe" "" "$INSTDIR\lightbookinput.ico"
  CreateShortcut "$SMPROGRAMS\LightBookInput\卸载轻书.lnk" "$INSTDIR\uninstall.exe" "" "$INSTDIR\lightbookinput.ico"
  CreateShortcut "$SMSTARTUP\轻书 Server.lnk" "$INSTDIR\lightbookinput-server.exe" "" "$INSTDIR\lightbookinput.ico"

  ; 5) 安装后：清旧任务 / 旧 DLL / 腾位文件，配权限，注册文本服务
  Call DeleteLegacyLogonTask
  Call DeleteStaleDlls
  nsExec::Exec '"$SYSDIR\icacls.exe" "$INSTDIR" /grant *S-1-15-2-1:(OI)(CI)RX /T /C /Q'
  Pop $0
!ifdef BUILD_X64
  nsExec::Exec '"$WINDIR\Sysnative\regsvr32.exe" /s "$INSTDIR\${TsfDll64}"'
  Pop $0
  nsExec::Exec '"$WINDIR\SysWOW64\regsvr32.exe" /s "$INSTDIR\${TsfDll32}"'
  Pop $0
!else
  nsExec::Exec '"$SYSDIR\regsvr32.exe" /s "$INSTDIR\${TsfDll32}"'
  Pop $0
!endif
SectionEnd

; ---------- 卸载 ----------
Section "Uninstall"
  Call un.RetireServerExe
  Call un.DeleteLegacyLogonTask
  Push "lightbookinput-server.exe"
  Call un.KillByName
  Push "lightbookinput-settings.exe"
  Call un.KillByName
!ifdef BUILD_X64
  nsExec::Exec '"$WINDIR\Sysnative\regsvr32.exe" /u /s "$INSTDIR\${TsfDll64}"'
  Pop $0
  nsExec::Exec '"$WINDIR\SysWOW64\regsvr32.exe" /u /s "$INSTDIR\${TsfDll32}"'
  Pop $0
!else
  nsExec::Exec '"$SYSDIR\regsvr32.exe" /u /s "$INSTDIR\${TsfDll32}"'
  Pop $0
!endif

  ; 删除 $INSTDIR 下所有文件与子目录（数据 / 资源 / 自包含运行时 / 各语言目录 / exe / dll）；
  ; uninstall.exe 本进程占用跳过，由 NSIS 退出时自删。占用中的文件 /REBOOTOK 登记重启后删。
  FindFirst $0 $1 "$INSTDIR\*"
  un_cl_loop:
    StrCmp $1 "" un_cl_done
    StrCmp $1 "uninstall.exe" un_cl_next
    Delete /REBOOTOK "$INSTDIR\$1"
    RMDir /r "$INSTDIR\$1"
  un_cl_next:
    FindNext $0 $1
    Goto un_cl_loop
  un_cl_done:
    FindClose $0

  ; 快捷方式
  Delete "$SMPROGRAMS\LightBookInput\轻书设置.lnk"
  Delete "$SMPROGRAMS\LightBookInput\卸载轻书.lnk"
  RMDir "$SMPROGRAMS\LightBookInput"
  Delete "$SMSTARTUP\轻书 Server.lnk"

  ; 注册表
  DeleteRegKey HKLM "${UninstKey}"
  DeleteRegKey HKLM "${RegKey}"

  RMDir "$INSTDIR"
SectionEnd

; ---------- 卸载器初始化 ----------
Function un.onInit
  Call un.HoldInstallerMutex
  SetShellVarContext all
!ifdef BUILD_X64
  SetRegView 64
!endif
FunctionEnd
