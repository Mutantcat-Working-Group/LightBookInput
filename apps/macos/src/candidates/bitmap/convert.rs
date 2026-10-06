//! 壳的帧类型 → 渲染器的帧类型。两边字段一一对应，spike 定型后壳直接用渲染器的类型，这层就没了。

use crate::candidates::frame::Frame;
use crate::candidates::preedit::{Preedit, PreeditStyle};
use crate::candidates::row::{Row, Tone};

pub(super) fn frame(frame: &Frame) -> lightbookinput_render::Frame {
    lightbookinput_render::Frame {
        preedit: frame.preedit.as_ref().map(preedit),
        rows: frame.rows.iter().map(row).collect(),
        highlighted: Some(frame.highlighted),
        columns: frame.columns,
        column_ems: frame.column_ems.clone(),
        footer: frame.footer.clone(),
        sentence: frame.sentence.clone(),
        status: frame.status.clone(),
    }
}

fn preedit(preedit: &Preedit) -> lightbookinput_render::Preedit {
    lightbookinput_render::Preedit {
        segments: preedit
            .segments
            .iter()
            .map(|segment| lightbookinput_render::PreeditSegment {
                text: segment.text.clone(),
                style: match segment.style {
                    PreeditStyle::Typed => lightbookinput_render::PreeditStyle::Typed,
                    PreeditStyle::Rest => lightbookinput_render::PreeditStyle::Rest,
                    PreeditStyle::Struck => lightbookinput_render::PreeditStyle::Struck,
                },
            })
            .collect(),
        cursor: preedit.cursor,
    }
}

fn row(row: &Row) -> lightbookinput_render::Row {
    lightbookinput_render::Row {
        index: row.index.clone(),
        text: row.text.clone(),
        code: None,
        annotation: row
            .annotation
            .iter()
            .map(|(text, tone)| {
                let tone = match tone {
                    Tone::Gloss => lightbookinput_render::Tone::Gloss,
                    Tone::Fresh => lightbookinput_render::Tone::Fresh,
                    Tone::Faint => lightbookinput_render::Tone::Faint,
                };
                (text.clone(), tone)
            })
            .collect(),
        cloud: row.cloud,
    }
}
