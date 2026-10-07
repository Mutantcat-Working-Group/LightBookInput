//! 学习与统计的挂钩：输入统计、输入日志、删候选、定时落盘。

use super::Engine;
use super::input_log::{CommitEntry, InputLogEntry, InputLogger, InputSource, LOGGED_CANDIDATES};
use super::statistics::Usage;
use crate::candidate::{Candidate, CandidateKind};
use crate::sentence;

mod forgotten;
mod learner;
mod muted;

pub use forgotten::Forgotten;
pub use learner::{Learner, NoLearner};
pub(super) use muted::MutedLearner;

impl Engine {
    /// 往输入统计记一次上屏：汉字与英文词按文字数，中文词数按来源定（选一个词算一个，整句按语言模型切出来的词数，
    /// 切不了就按字数）。`english_word` 是原样上屏的字母串算不算一个英文词（拼音回车不算）。
    pub(super) fn meter_commit(&mut self, text: &str, source: InputSource, english_word: bool) {
        let mut usage = Usage::of_text(text);
        usage.words = match source {
            InputSource::Word | InputSource::Cloud => 1,
            InputSource::Sentence | InputSource::CloudSentence => {
                sentence::segment_text(text, &*self.language_model).map_or(usage.hanzi, |clauses| {
                    clauses.iter().map(|words| words.len() as u64).sum()
                })
            }
            InputSource::English
            | InputSource::Custom
            | InputSource::Shortcut
            | InputSource::Emoji
            | InputSource::Raw => 0,
        };
        if source == InputSource::Raw && !english_word {
            usage.english_words = 0;
        }
        self.meter.record(usage);
    }

    /// 往输入日志记一次上屏。`keys` 是这次消耗掉的原始键，`index` 从上一次查询的候选里找。
    /// 攒着的直通字符先写出去（顺序要对）；这段组句里退格过且最终键串变了，紧跟着记一条 `retype`。
    pub(super) fn log_commit(&mut self, keys: &str, text: &str, source: InputSource) -> u64 {
        self.flush_passthrough();
        self.log_sequence += 1;
        let snapshot = self.last_query.borrow().clone().unwrap_or_default();
        let index = snapshot.candidates.iter().position(|c| c == text);
        let ms = self
            .composition_started
            .map_or(0, |started| started.elapsed().as_millis() as u64);
        self.logger.record(InputLogEntry::Commit(CommitEntry {
            id: self.log_sequence,
            scope: snapshot.scope,
            keys: keys.to_owned(),
            pinyin: snapshot.pinyin,
            corrected: snapshot.corrected,
            text: text.to_owned(),
            source,
            index,
            top: snapshot
                .candidates
                .into_iter()
                .take(LOGGED_CANDIDATES)
                .collect(),
            scheme: self.scheme_key(),
            english: self.english_mode,
            rescored: snapshot.rescored,
            app: self.application.clone(),
            pages: self.page_turns,
            ms,
        }));
        self.committed_since_break = true;
        self.page_turns = 0;
        if let Some(before) = self.retype_snapshot.take() {
            let after = self.composition.scope();
            if before != after {
                self.logger.record(InputLogEntry::Retype {
                    before,
                    after: after.to_owned(),
                    of: self.log_sequence,
                });
            }
        }
        self.log_sequence
    }

    /// 用户要求删掉一个候选（修饰键 + 数字）：中文词与云端词交给 Learner 删用户词、清学习；英文词删个人英文词；
    /// 整句、快捷候选、emoji 没什么可删。删完缓存作废，它也不再当下一个词的上文。
    pub fn forget(&mut self, candidate: &Candidate) -> Forgotten {
        let mut candidate_owned = candidate.clone();
        if self.traditional
            && let Some(simp) = self.traditional_map.borrow().get(&candidate_owned.text)
        {
            candidate_owned.text = simp.clone();
        }
        let candidate = &candidate_owned;
        let forgotten = match candidate.kind {
            CandidateKind::Chinese | CandidateKind::Code | CandidateKind::Cloud => {
                self.learner.forget(&candidate.text)
            }
            CandidateKind::English => Forgotten {
                user_word: self.learner.forget_english(&candidate.text),
                learning: false,
            },
            CandidateKind::Sentence
            | CandidateKind::Shortcut
            | CandidateKind::Custom(_)
            | CandidateKind::Emoji
            | CandidateKind::Generated => Forgotten::default(),
        };
        if !forgotten.is_nothing() {
            self.forget_span_cache();
            *self.correction_cache.borrow_mut() = None;
            if self.chain.mentions(&candidate.text) {
                self.chain.reset();
            }
            self.recent_commits.retain(|c| c.text != candidate.text);
            tracing::debug!(text = %candidate.text, ?forgotten, "删除候选");
        }
        forgotten
    }

    /// 把学习数据与输入日志落盘。壳在停用输入法时调，激活期间也可以定时调（进程被杀时少丢）：
    /// 落盘不改学习状态，所以不像 [`Self::learner_mut`] 那样作废格子缓存。
    pub fn flush_learning(&mut self) {
        self.learner.flush();
        self.logger.flush();
        self.meter.flush();
    }

    /// 词库 / 用户词 / 学习数据变了，整句格子候选全部作废。
    pub(super) fn forget_span_cache(&self) {
        self.span_cache.borrow_mut().clear();
    }
}
