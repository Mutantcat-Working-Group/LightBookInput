//! 候选窗口的一行：[`Candidate`] → 渲染器的 [`Row`]（序号、候选词、annotation 片段），与 macOS 端 `candidates/row.rs` 一致。
//! GDI 画法也用同一个类型。

use lightbookinput_core::{Candidate, CandidateKind};
use lightbookinput_render::{Row, Tone};

/// `position` 是页内下标（从 0 起）。`show_code` 是 `[general] aux_code_show`：
/// 打开且候选带码时，码用方括号括起来紧跟在候选词后面（`鹤[rbm]`），不进 annotation。
/// `gloss_selected` 是程序员模式里当前选中的释义下标（只有高亮那一行会画它的光标）。
pub(crate) fn from_candidate(
    position: usize,
    candidate: &Candidate,
    show_code: bool,
    gloss_selected: usize,
) -> Row {
    let code = candidate
        .aux_code
        .as_ref()
        .filter(|_| show_code)
        .map(|code| format!("[{code}]"));
    let mut annotation = Vec::new();
    if let Some(reading) = &candidate.reading {
        annotation.push((reading.clone(), Tone::Gloss));
    }
    // 程序员模式（点按 `~` 后）候选旁单独一行英文释义：按 `; ` 拆成多条，方向键切换选中的那条。
    // 即使只有一条也拆成一行，这样第二行照样有选择光标，用户一眼看出数字键上屏哪条。
    let gloss: Vec<String> = candidate
        .gloss
        .as_deref()
        .map(|gloss| gloss.split("; ").map(str::to_owned).collect())
        .unwrap_or_default();
    Row {
        index: (position + 1).to_string(),
        text: candidate.text.clone(),
        code,
        annotation,
        gloss,
        gloss_selected,
        cloud: candidate.kind == CandidateKind::Cloud,
    }
}
