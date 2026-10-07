/// 一次联想请求要的是什么。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum PredictionKind {
    /// 组句中：这段拼音对应的词，可能还要整句补全。
    #[default]
    Compose,

    /// 问字模式（`?` 开头）：用拼音问「三个木是什么字」之类的问题，要答案（字或短答案）与读音。
    Question,
}
