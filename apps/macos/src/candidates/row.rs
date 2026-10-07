//! 候选窗口的一行：序号、候选词、annotation 片段。只是 Core 输出的展示形态，不含任何排序或查词。

use lightbookinput_core::Candidate;

/// annotation 片段的深浅。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// 译文。
    Gloss,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// 显示用序号文本，如 `1`。
    pub index: String,

    /// 候选词。
    pub text: String,

    /// 右侧 annotation，按顺序绘制；没有译文时为空。
    pub annotation: Vec<(String, Tone)>,

    /// 程序员模式的英文释义：按 `; ` 拆开的各条；空表示不在程序员模式。
    pub gloss: Vec<String>,

    /// `gloss` 里当前选中的下标（越界时按 0 处理）。
    pub gloss_selected: usize,

    /// 来自云联想：词前画一个小云朵，与本地候选区分。
    pub cloud: bool,
}

impl Row {
    pub fn from_candidate(position: usize, candidate: &Candidate) -> Self {
        let mut annotation = Vec::new();
        // 读音（问字模式答案的带声调拼音）放在最前
        if let Some(reading) = &candidate.reading {
            annotation.push((reading.clone(), Tone::Gloss));
        }
        Self {
            index: (position + 1).to_string(),
            text: if matches!(
                candidate.kind,
                lightbookinput_core::CandidateKind::Custom(_)
            ) {
                lightbookinput_core::CustomPhrase::preview(&candidate.text, 60)
            } else {
                candidate.text.clone()
            },
            annotation,
            gloss: Vec::new(),
            gloss_selected: 0,
            cloud: false,
        }
    }

    /// 程序员模式（点按 `~` 切换）时在候选右侧补一行英文释义，数字 / 空格上屏的就是它。
    /// `senses` 是拆好的各条释义，`selected` 是当前选中的下标。
    pub fn with_gloss(mut self, senses: Option<Vec<String>>, selected: usize) -> Self {
        if let Some(senses) = senses {
            self.gloss = senses;
            self.gloss_selected = selected;
        }
        self
    }
}
