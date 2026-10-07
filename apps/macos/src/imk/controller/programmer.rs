//! 程序员模式：按住 `~`（·）时数字键 / 空格上屏候选的英文释义。

use super::*;

/// `~` / `` ` `` 键的键码（ANSI 布局 Esc 左边那个键，Shift 出来是 `~`）。
const TILDE_KEY: u16 = 50;

/// Esc 的键码。
const ESC_KEY: u16 = 53;

/// 空格与小键盘空格的键码。
const SPACE_KEY: u16 = 49;

/// 上下左右键的键码。
const UP_KEY: u16 = 126;
const DOWN_KEY: u16 = 125;
const LEFT_KEY: u16 = 123;
const RIGHT_KEY: u16 = 124;

impl LightBookInputInputController {
    /// 松开 `~`：应用送得来 KeyUp 时就地退出程序员模式（送不来的靠 Esc 或再按一次退出）。
    pub(super) fn programmer_release(&self, key: u16, client: TextClient<'_>) -> bool {
        if key != TILDE_KEY {
            return false;
        }
        if !host::with(|h| h.programmer_mode).unwrap_or(false) {
            return false;
        }
        tracing::debug!("松开 ~，退出程序员模式");
        host::with(|h| h.programmer_mode = false);
        self.refresh(client);
        true
    }

    /// 程序员模式的按键分流。返回 `Some(true)` 表示这个键已经在这里处理掉，应用不该再收到；
    /// 返回 `None` 表示这儿不管，按普通按键继续走。
    ///
    /// 按下 `~` 进模式（只在组句中进：没有候选时它还是原来的标点键），再按一次、Esc、或按了别的键退出；
    /// 模式里四条方向键都在当前高亮候选的多条释义之间切换，不动第一行候选高亮；
    /// `1`-`9` 上屏对应候选当前选中的释义，空格上屏高亮候选的释义。
    pub(super) fn programmer_key(
        &self,
        key: u16,
        repeat: bool,
        pressed: Modifiers,
        client: TextClient<'_>,
    ) -> Option<bool> {
        let active = host::with(|h| h.programmer_mode).unwrap_or(false);
        // Esc：模式开着时只用来退出，不再往应用发 cancelOperation
        if key == ESC_KEY {
            if active {
                self.exit_programmer(client);
                return Some(true);
            }
            return None;
        }
        if key == TILDE_KEY {
            // 一直按着会不断送重复按下，那不是「又按了一次」，否则按住时打不了数字
            if repeat {
                return Some(true);
            }
            if active {
                self.exit_programmer(client);
                return Some(true);
            }
            let composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
            if composing {
                tracing::debug!("按住 ~，进入程序员模式");
                host::with(|h| h.programmer_mode = true);
                self.refresh(client);
                return Some(true);
            }
            return None;
        }
        if !active {
            return None;
        }
        // 带修饰键的（⇧1 是删候选、⌥1 是应用快捷键）先退出，按普通按键处理
        if !pressed.is_empty() {
            host::with(|h| h.programmer_mode = false);
            return None;
        }
        // 四条方向键都只切当前高亮候选的第二行释义光标，不移动第一行候选高亮
        if key == UP_KEY || key == DOWN_KEY || key == LEFT_KEY || key == RIGHT_KEY {
            let delta = if key == UP_KEY || key == LEFT_KEY {
                -1
            } else {
                1
            };
            if host::with(|h| h.move_gloss(delta)).unwrap_or(false) {
                self.render(client);
            }
            return Some(true);
        }
        if key == SPACE_KEY {
            return Some(self.commit_english(1, client));
        }
        if let Some(digit) = digit_key(key) {
            return Some(self.commit_english(digit, client));
        }
        // 别的键（字母、翻页键…）：退出模式，按普通按键处理
        host::with(|h| h.programmer_mode = false);
        None
    }

    /// 退出程序员模式并重画（把状态行与英文释义收掉）。
    fn exit_programmer(&self, client: TextClient<'_>) {
        host::with(|h| h.programmer_mode = false);
        self.refresh(client);
    }

    /// 上屏当前页第 `digit` 个候选当前选中的英文释义。释义表里没有这个词就照常上屏中文；
    /// 那一格没有候选就退出模式，把这个键交回普通流程。
    pub(super) fn commit_english(&self, digit: usize, client: TextClient<'_>) -> bool {
        let Some(index) = host::with(|h| h.session.index_on_page(digit - 1)).flatten() else {
            host::with(|h| h.programmer_mode = false);
            return false;
        };
        let candidate = host::with(|h| h.session.candidate(index)).flatten();
        let gloss = candidate.as_ref().and_then(|candidate| {
            host::with(|h| h.engine.english_gloss(&candidate.text)).flatten()
        });
        let Some(gloss) = gloss else {
            return self.commit_index(index, client);
        };
        let selected = host::with(|h| h.gloss_index).unwrap_or(0);
        let senses: Vec<&str> = gloss.split("; ").collect();
        let text = senses
            .get(selected)
            .copied()
            .unwrap_or(senses[0])
            .to_owned();
        let english = Candidate {
            text,
            // 快捷候选：上屏时吃掉整段拼音，不记学习
            kind: CandidateKind::Shortcut,
            syllables: Vec::new(),
            reading: None,
            aux_code: None,
            gloss: None,
        };
        let Some(text) = host::with(|h| h.engine.commit(&english)) else {
            return false;
        };
        tracing::debug!(%text, "程序员模式上屏英文");
        client.insert_text(&text);
        host::with(|h| h.programmer_mode = false);
        self.refresh(client);
        true
    }

    /// 修饰键 + 数字（缺省 ⇧）：删掉当前页第几个候选。不在组句中不管；那格没有候选就吞掉按键不动。
    /// 删完重新查一遍（排序会变），结果那句话显示在拼音行右侧，敲下一键就没了。
    pub(super) fn handle_delete_key(&self, digit: usize, client: TextClient<'_>) -> bool {
        let composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
        if !composing {
            return false;
        }
        let Some(message) = host::with(|h| h.forget_candidate(digit - 1)).flatten() else {
            tracing::debug!(digit, "这一格没有候选，没什么可删");
            return true;
        };
        tracing::info!(%message);
        self.refresh(client);
        host::with(|h| h.status = Some(message));
        self.render(client);
        true
    }
}
