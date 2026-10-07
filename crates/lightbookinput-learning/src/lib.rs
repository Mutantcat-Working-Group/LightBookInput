//! 用户词频学习与输入日志的落盘。
//!
//! [`FrequencyLearner`] 存聚合数据（选择次数、输入串选择、用户词、个人英文词、个人 n-gram，都是 TSV）；
//! [`InputLog`] 逐条记上屏（jsonl），给离线回归评测与个人模型用；[`UsageStats`] 按天数打了多少字（`usage.tsv`）。

mod error;
mod frequency_learner;
mod input_log;
mod usage_stats;

pub use error::LearningError;
pub use frequency_learner::FrequencyLearner;
pub use input_log::InputLog;
pub use usage_stats::UsageStats;
