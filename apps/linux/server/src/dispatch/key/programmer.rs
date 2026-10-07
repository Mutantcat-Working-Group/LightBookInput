//! 程序员模式：按住 `~`（·）时数字键 / 空格上屏候选的英文释义。与 Windows / macOS 端对齐。

use lightbookinput_core::{Candidate, CandidateKind};
use lightbookinput_platform::protocol::{KeyEvent, KeyModifiers};

use super::{Effect, codes};
use crate::dispatch::Router;

impl Router {
    /// 程序员模式的按键分流。返回 `Some(effect)` 表示这个键已经在这里处理掉；
    /// 返回 `None` 表示这儿不管，按普通按键继续走。
    ///
    /// 按下 `~` 进模式（只在组句中进：没有候选时它还是原来的标点键），再按一次、Esc、或按了别的键退出；
    /// 模式里 `1`-`9` 上屏对应候选的英文，空格上屏第一个候选的英文。
    /// 松键由 Linux 事件的 release 分支处理（见 `dispatch::linux`）。
    pub(super) fn apply_programmer(&mut self, event: &KeyEvent) -> Option<Effect> {
        // Esc：模式开着时只用来退出，不当作普通退格 / 清缓冲
        if event.virtual_key == codes::ESCAPE {
            if self.programmer {
                tracing::debug!("Esc 退出程序员模式");
                self.programmer = false;
                return Some(Effect::Navigated);
            }
            return None;
        }
        if codes::tilde(event) {
            if self.programmer {
                tracing::debug!("再按一次 ~，退出程序员模式");
                self.programmer = false;
                return Some(Effect::Navigated);
            }
            if self.composing() {
                tracing::debug!("按住 ~，进入程序员模式");
                self.programmer = true;
                return Some(Effect::Navigated);
            }
            return None;
        }
        if !self.programmer {
            return None;
        }
        // 带修饰键的（⇧1 是删候选、⌥1 是应用快捷键）先退出，按普通按键处理
        if event.modifiers.chord() != KeyModifiers::default() {
            self.programmer = false;
            return None;
        }
        if event.virtual_key == codes::SPACE {
            return Some(self.commit_english(1));
        }
        if let Some(digit) = codes::digit(event) {
            return Some(self.commit_english(digit));
        }
        // 别的键（字母、翻页键…）：退出模式，按普通按键处理
        self.programmer = false;
        None
    }

    /// 上屏当前页第 `digit` 个候选的英文释义。释义表里没有这个词就照常上屏中文；
    /// 那一格没有候选就退出模式，把这个键交回普通流程。
    fn commit_english(&mut self, digit: usize) -> Effect {
        let Some(index) = self.slot_index(digit) else {
            tracing::debug!(digit, "这一格没有候选，退出程序员模式");
            self.programmer = false;
            return Effect::Navigated;
        };
        let candidate = self.layout_candidate(index);
        let gloss = candidate
            .as_ref()
            .and_then(|candidate| self.engine.english_gloss(&candidate.text));
        let Some(gloss) = gloss else {
            return Effect::Changed(self.commit_index(index));
        };
        let english = Candidate {
            text: gloss,
            // 快捷候选：上屏时吃掉整段拼音，不记学习
            kind: CandidateKind::Shortcut,
            syllables: Vec::new(),
            reading: None,
            aux_code: None,
            gloss: None,
        };
        self.programmer = false;
        Effect::Changed(Some(self.engine.commit(&english)))
    }
}
