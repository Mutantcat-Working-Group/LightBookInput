//! 轻书输入法内核。
//!
//! 平台无关：词库、拼音解析、候选生成、排序与学习的接口全部在这里。
//! 平台层（IMK / TSF / IBus-Fcitx）只负责把按键喂给 [`Engine`]、把候选画出来。
//! 判断标准：换掉 IMK 换成 TSF，不应该需要改这里的任何一行。

pub mod candidate;
pub mod composition;
pub mod correction;
pub mod custom_phrase;
pub mod emoji;
pub mod engine;
pub mod english;
pub mod fuzzy;
pub mod history;
pub mod parser;
pub mod punctuation;
pub mod ranking;
pub mod sentence;
pub mod shortcut;
pub mod shuangpin;
pub mod storage;
pub mod zhuyin;

pub use custom_phrase::CustomPhrase;

pub use candidate::{
    Candidate, CandidateKind, CandidateLayout, CandidateList, Cell, GRID_ROWS, Grid, MAX_CELL_EMS,
};
pub use composition::Composition;
pub use correction::Correction;
pub use emoji::EmojiTable;
pub use engine::{
    AuxSegment, BOOKS, Book, CloudWord, CommitEntry, DEFAULT_AUX_CODE_KEY, Engine, EngineSession,
    Forgotten, INPUT_LOG_VERSION, InputLogEntry, InputLogger, InputSource, Learner, MarkedKind,
    MarkedSegment, ModeKeys, NEURAL_MARGIN, NEURAL_WEIGHT, NoInputLogger, NoLearner, NoPredictor,
    NoUsageMeter, Prediction, PredictionKind, PredictionPolicy, PredictionRequest, Predictor,
    QUESTION_PREFIX, Query, RESCORE_CONTEXT_CHARS, RawPreedit, SurroundingText, Timings, Usage,
    UsageMeter, UsageSummary, book_scale, is_valid_aux_code_key,
};
pub use fuzzy::FuzzyRules;
pub use history::InputHistory;
pub use lightbookinput_dictionary as dictionary;
pub use parser::{ParseError, Segmentation};
pub use punctuation::Punctuation;
pub use shuangpin::Scheme as ShuangpinScheme;
