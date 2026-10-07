//! 「Ctrl + Alt + Space 切换中英」登记成 TSF **保留键**（preserved key）。
//! 带 Alt 的组合是系统键，不经击键 sink（真机：Ctrl+Alt+Space 在 `OnTestKeyDown` 里从没出现过）；
//! 保留键由 TSF 在应用之前匹配、回调 `OnPreservedKey`，UWP 里也一样。
//! 切换键来自 `[shortcut] switch_mode`，那个值由 Server 经协议下发（DLL 不读配置文件），变了就地重登记。

use windows::Win32::UI::Input::KeyboardAndMouse::VK_SPACE;
use windows::Win32::UI::TextServices::{
    ITfKeystrokeMgr, TF_MOD_ALT, TF_MOD_CONTROL, TF_PRESERVEDKEY,
};
use windows::core::{GUID, Result};

/// Ctrl + Alt + Space 中英切换键的保留键标识。
pub(crate) const GUID_SWITCH_MODE: GUID = GUID::from_u128(0x2f6b8c51_9a34_4e7d_b2c8_5d1e0f3a7b64);

/// Ctrl + Alt + Space 的 `TF_PRESERVEDKEY`。不用 Ctrl + Space：中文 Windows 把它绑成系统的
/// 「输入法/非输入法切换」，系统先截走，保留键收不到。
fn switch_mode_key() -> TF_PRESERVEDKEY {
    TF_PRESERVEDKEY {
        uVKey: VK_SPACE.0 as u32,
        uModifiers: TF_MOD_CONTROL | TF_MOD_ALT,
    }
}

/// 登记 Ctrl + Alt + Space 为中英切换保留键（`switch_mode` 勾了它时）。
pub(crate) fn register_switch_mode(keystroke: &ITfKeystrokeMgr, tid: u32) -> Result<()> {
    let description: Vec<u16> = "切换中英文（轻书）".encode_utf16().collect();
    unsafe { keystroke.PreserveKey(tid, &GUID_SWITCH_MODE, &switch_mode_key(), &description) }
}

pub(crate) fn unregister_switch_mode(keystroke: &ITfKeystrokeMgr) {
    let _ = unsafe { keystroke.UnpreserveKey(&GUID_SWITCH_MODE, &switch_mode_key()) };
}
