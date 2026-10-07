//! 程序员模式：点按 `~`（·）切换，模式里数字键 / 空格上屏候选的英文释义。与 Windows / macOS 端对齐。

use lightbookinput_core::{Candidate, CandidateKind};
use lightbookinput_platform::protocol::{KeyEvent, KeyModifiers};

use super::{Effect, codes};
use crate::dispatch::Router;

impl Router {
    /// 程序员模式的按键分流。返回 `Some(effect)` 表示这个键已经在这里处理掉；
    /// 返回 `None` 表示这儿不管，按普通按键继续走。
    ///
    /// 按下 `~` 进模式（只在组句中进：没有候选时它还是原来的标点键），再按一次、Esc、或按了别的键退出；
    /// 模式里方向键切换高亮候选的第二行释义光标（四条方向一致，多条时循环），`1`-`9` 上屏对应候选
    /// 当前选中的英文，空格上屏高亮候选的英文。
    /// 退出只靠再按一次、Esc 或敲别的键，与另两端一致。
    pub(super) fn apply_programmer(&mut self, event: &KeyEvent) -> Option<Effect> {
        // Esc：模式开着时只用来退出，不当作普通退格 / 清缓冲
        if event.virtual_key == codes::ESCAPE {
            if self.programmer {
                tracing::debug!("Esc 退出程序员模式");
                self.programmer = false;
                self.gloss_index = 0;
                return Some(Effect::Navigated);
            }
            return None;
        }
        if codes::tilde(event) {
            if self.programmer {
                tracing::debug!("再按一次 ~，退出程序员模式");
                self.programmer = false;
                self.gloss_index = 0;
                return Some(Effect::Navigated);
            }
            if self.composing() {
                tracing::debug!("点按 ~，进入程序员模式");
                self.programmer = true;
                self.gloss_index = 0;
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
            self.gloss_index = 0;
            return None;
        }
        if event.virtual_key == codes::SPACE {
            return Some(self.commit_english(1));
        }
        if let Some(digit) = codes::digit(event) {
            return Some(self.commit_english(digit));
        }
        // 方向键统一驱动第二行释义光标：不管当前候选有几条翻译，都把选中的那条画上光标；
        // 多条时循环切换，数字 / 空格上屏当前选中的那条。
        if matches!(
            event.virtual_key,
            codes::UP | codes::DOWN | codes::LEFT | codes::RIGHT
        ) {
            let Some(index) = self.slot_index(1) else {
                return Some(Effect::Navigated);
            };
            let candidate = self.layout_candidate(index);
            let count = candidate
                .as_ref()
                .and_then(|candidate| self.engine.english_gloss(&candidate.text))
                .map(|gloss| gloss.split("; ").count())
                .unwrap_or(0);
            if count > 1 {
                let delta = if matches!(event.virtual_key, codes::UP | codes::LEFT) {
                    -1
                } else {
                    1
                };
                self.gloss_index =
                    (self.gloss_index as isize + delta).rem_euclid(count as isize) as usize;
            }
            return Some(Effect::Navigated);
        }
        // 别的键（字母、翻页键…）：退出模式，按普通按键处理
        self.programmer = false;
        self.gloss_index = 0;
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
        // 一个词可能有多条释义（Core 用 "; " 拼成一条给）：上屏当前高亮的那一条，越界回第一条
        let senses: Vec<&str> = gloss.split("; ").collect();
        let text = senses
            .get(self.gloss_index)
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
        self.programmer = false;
        self.gloss_index = 0;
        Effect::Changed(Some(self.engine.commit(&english)))
    }
}
