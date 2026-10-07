//! 按键相关：TSF 虚拟键码到协议 [`KeyEvent`](lightbookinput_platform::protocol::KeyEvent) 的翻译（[`event`]）、
//! 单击中英切换键的判定（[`tap`]，键来自 `[shortcut] switch_mode`）、中英切换快捷键的保留键登记（[`preserved`]）。

pub(crate) mod event;
mod layout;
pub(crate) mod preserved;
mod tap;

pub(crate) use self::tap::KeyTap;
