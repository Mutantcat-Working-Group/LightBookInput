//! annotation 片段的深浅。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// 程序员模式的英文释义。
    Gloss,

    /// 强调的释义片段：比普通释义醒目。
    Fresh,

    /// 词性与分隔符，最浅。
    Faint,

    /// 辅码态命中的那条码（`[general] aux_code_show` 打开时才有）：与释义同一个淡色。
    Code,
}
