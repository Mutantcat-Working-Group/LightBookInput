//! 「统计」页：用轻书打了多少字。今天 / 最近 7 天 / 累计 三行，汉字 / 中文词 / 英文词 / 上屏次数 四列，
//! 再把累计汉字数折成「几本《某书》」给个直观参照。
//! 数据来自 `Engine::usage_summary`，打开窗口时更新。

use lightbookinput_core::{Usage, UsageSummary, book_scale};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSFont, NSTextAlignment, NSTextField};
use objc2_foundation::NSString;

use crate::preferences::controls::{GROUP_GAP, caption, note_full, small_label};
use crate::preferences::layout::{LABEL_WIDTH, Layout, PAGE_PADDING, ROW_HEIGHT};

/// 三行的标题，顺序与 [`UsagePage::show`] 里取值一致。
const ROWS: [&str; 3] = ["今天", "最近 7 天", "累计"];

/// 四列的标题。
const COLUMNS: [&str; 4] = ["汉字", "中文词", "英文词", "上屏次数"];

/// 「统计」页里要按数据刷新的控件。
pub struct UsagePage {
    /// 数字格子，按行优先排：`cells[row * 4 + column]`。
    cells: Vec<Retained<NSTextField>>,

    /// 「累计 xx 字，约等于 n 本《某书》」。
    scale: Retained<NSTextField>,

    /// 「自 某日 起，记了 n 天」。
    since: Retained<NSTextField>,
}

impl UsagePage {
    /// 把「统计」页的控件摆进 `layout`。
    pub fn build(layout: &mut Layout, mtm: MainThreadMarker) -> Self {
        let column_width = (layout.inner_width() - LABEL_WIDTH) / COLUMNS.len() as f64;
        let column_x = |column: usize| PAGE_PADDING + LABEL_WIDTH + column_width * column as f64;
        for (column, title) in COLUMNS.iter().enumerate() {
            let header = small_label(mtm, title);
            header.setAlignment(NSTextAlignment::Right);
            layout.place(&header, column_x(column), column_width, ROW_HEIGHT * 0.7);
        }
        layout.next_row(ROW_HEIGHT * 0.7);
        // SAFETY: NSFontWeightRegular 是 AppKit 导出的常量，只读。
        let weight = unsafe { objc2_app_kit::NSFontWeightRegular };
        let mut cells = Vec::with_capacity(ROWS.len() * COLUMNS.len());
        for title in ROWS {
            let label = caption(mtm, title);
            layout.place(&label, PAGE_PADDING, LABEL_WIDTH, ROW_HEIGHT);
            for column in 0..COLUMNS.len() {
                let cell = NSTextField::labelWithString(&NSString::from_str("0"), mtm);
                cell.setAlignment(NSTextAlignment::Right);
                cell.setFont(Some(&NSFont::monospacedDigitSystemFontOfSize_weight(
                    13.0, weight,
                )));
                layout.place(&cell, column_x(column), column_width, ROW_HEIGHT);
                cells.push(cell);
            }
            layout.next_row(ROW_HEIGHT);
        }
        layout.space(GROUP_GAP);
        let scale = NSTextField::labelWithString(&NSString::from_str(""), mtm);
        scale.setFont(Some(&NSFont::boldSystemFontOfSize(13.0)));
        layout.place(&scale, PAGE_PADDING, layout.inner_width(), ROW_HEIGHT);
        layout.next_row(ROW_HEIGHT);
        let since = small_label(mtm, "");
        layout.place(&since, PAGE_PADDING, layout.inner_width(), ROW_HEIGHT * 0.7);
        layout.next_row(ROW_HEIGHT * 0.7);
        layout.space(GROUP_GAP);
        note_full(
            layout,
            mtm,
            "数的是上屏的文字：选一个词算一个中文词，整句按词切开数；英文候选与回车原样上屏的英文词算英文词。\
             只在这台电脑上数，与输入日志无关，关掉或清空日志不影响这里。",
        );
        Self {
            cells,
            scale,
            since,
        }
    }

    /// 按汇总刷新。
    pub fn show(&self, summary: &UsageSummary) {
        let rows = [summary.today, summary.week, summary.total];
        for (row, usage) in rows.iter().enumerate() {
            for (column, value) in columns(usage).into_iter().enumerate() {
                self.cells[row * COLUMNS.len() + column]
                    .setStringValue(&NSString::from_str(&group_digits(value)));
            }
        }
        self.scale
            .setStringValue(&NSString::from_str(&scale_line(summary.total.hanzi)));
        let since = match &summary.since {
            Some(date) => format!("自 {date} 起，有输入的天数 {}。", summary.days),
            None => "还没有记录，打几个字再来看。".to_owned(),
        };
        self.since.setStringValue(&NSString::from_str(&since));
    }
}

/// 一行四列的值，顺序同 [`COLUMNS`]。
fn columns(usage: &Usage) -> [u64; 4] {
    [usage.hanzi, usage.words, usage.english_words, usage.commits]
}

/// 「累计输入 12.3 万字，约等于 1.7 本《活着》（约 12 万字）。」；一个字没有时不比。
fn scale_line(hanzi: u64) -> String {
    if hanzi == 0 {
        return "累计输入 0 字。".to_owned();
    }
    let (book, ratio) = book_scale(hanzi);
    format!(
        "累计输入 {}，约等于 {} 本《{}》（约 {}）。",
        hanzi_text(hanzi),
        format_ratio(ratio),
        book.title,
        hanzi_text(book.hanzi)
    )
}

/// 千位分隔：`12345` → `12,345`。
fn group_digits(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// 中文习惯的字数写法：万以下写整数（`2,500 字`），万以上一位小数（`12.3 万字`），亿以上同理。
fn hanzi_text(value: u64) -> String {
    const WAN: f64 = 10_000.0;
    const YI: f64 = 100_000_000.0;
    let value_f = value as f64;
    if value_f >= YI {
        format!("{} 亿字", trim_decimal(value_f / YI))
    } else if value_f >= WAN {
        format!("{} 万字", trim_decimal(value_f / WAN))
    } else {
        format!("{} 字", group_digits(value))
    }
}

/// 一位小数，`.0` 去掉。
fn trim_decimal(value: f64) -> String {
    let text = format!("{value:.1}");
    text.strip_suffix(".0").map_or(text.clone(), str::to_owned)
}

/// 倍数：不到 10 倍留一位小数，再多就取整。
fn format_ratio(ratio: f64) -> String {
    if ratio >= 10.0 {
        format!("{}", ratio.round() as u64)
    } else {
        trim_decimal(ratio)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_digits_and_scales_in_chinese_units() {
        assert_eq!(group_digits(0), "0");
        assert_eq!(group_digits(999), "999");
        assert_eq!(group_digits(1000), "1,000");
        assert_eq!(group_digits(1_234_567), "1,234,567");
        assert_eq!(hanzi_text(9_999), "9,999 字");
        assert_eq!(hanzi_text(10_000), "1 万字");
        assert_eq!(hanzi_text(123_456), "12.3 万字");
        assert_eq!(hanzi_text(250_000_000), "2.5 亿字");
    }

    #[test]
    fn describes_the_total_against_a_book() {
        assert_eq!(scale_line(0), "累计输入 0 字。");
        assert_eq!(
            scale_line(2_500),
            "累计输入 2,500 字，约等于 0.5 本《道德经》（约 5,000 字）。"
        );
        assert_eq!(
            scale_line(204_000),
            "累计输入 20.4 万字，约等于 1.7 本《活着》（约 12 万字）。"
        );
        assert_eq!(
            scale_line(12_000_000),
            "累计输入 1200 万字，约等于 12 本《平凡的世界》（约 100 万字）。"
        );
    }
}
