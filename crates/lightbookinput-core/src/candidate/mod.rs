//! 候选词数据模型。
//!
//! 候选就是上屏文本加展示所需的属性（来源、音节、读音、辅码），不带译词一类的辅助语言内容。

mod kind;
mod layout;
mod list;

use serde::{Deserialize, Serialize};

pub use kind::CandidateKind;
pub use layout::{CandidateLayout, Cell, GRID_ROWS, Grid, MAX_CELL_EMS};
pub use list::CandidateList;
/// 一个可上屏的候选。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    /// 上屏文本。
    pub text: String,

    /// 来源类型。
    pub kind: CandidateKind,

    /// 该候选对应的拼音音节，供平台层高亮已匹配部分。
    pub syllables: Vec<String>,

    /// 读音（如日语假名），中文候选暂不使用。
    pub reading: Option<String>,

    /// 候选带的辅码：筛码时是命中当前码段的那条，没在筛码（纯拼音态、辅码态空码段）时是词的
    /// 首条码（一词多码、多张码表取第一条）；没装码表或这个词没有码时为 `None`。
    /// 壳按 `[general] aux_code_show` 决定要不要显示。
    #[serde(default)]
    pub aux_code: Option<String>,
}
