use serde::{Deserialize, Serialize};

/// 候选窗口排布。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LayoutMode {
    /// 横排（缺省）：候选排成一行，空格或数字键选高亮那个。
    #[default]
    Horizontal,

    /// 竖排：一行一个候选，一页九个排成矩阵。
    Vertical,
}

impl LayoutMode {
    /// 全部取值，设置界面按这个顺序列出。
    pub const ALL: [Self; 2] = [Self::Horizontal, Self::Vertical];

    /// 配置文件里的写法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Vertical => "vertical",
            Self::Horizontal => "horizontal",
        }
    }

    /// 界面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Vertical => "竖排",
            Self::Horizontal => "横排",
        }
    }
}
