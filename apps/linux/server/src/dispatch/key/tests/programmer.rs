//! 组句中的程序员模式：按住 `~`（·）期间数字键 / 空格上屏候选的英文释义。与 Windows 端对齐。

use super::support::{compose, key};
use crate::dispatch::{Router, RouterConfig};
use lightbookinput_core::Engine;
use lightbookinput_dictionary::{Dictionary, EnglishGlossary};
use lightbookinput_platform::protocol::{
    ClientMessage, Frame, KeyModifiers, KeyOutcome, PROTOCOL_VERSION, SessionId,
};

/// 带释义表的样例词库：`你好` 有英文，`上` 没有。
fn router() -> Router {
    let mut engine =
        Engine::new(Dictionary::parse("你\tni\t100\n好\thao\t90\n上\tshang\t80\n").unwrap());
    engine.set_english_glossary(EnglishGlossary::parse("你好\thello\n").unwrap());
    let mut router = Router::new(
        engine,
        RouterConfig {
            page_size: 9,
            ..Default::default()
        },
    );
    router.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    router
}

fn preedit(frame: &Frame) -> String {
    frame
        .preedit
        .iter()
        .map(|segment| segment.text.as_str())
        .collect()
}

/// 主键盘数字键 `n`（1–9）的键码与字符。
fn digit(n: u32) -> (u32, Option<char>) {
    (0x30 + n, Some(char::from_digit(n, 10).unwrap()))
}

/// 当前页里 `text` 排第几（1 起）。
fn slot_of(frame: &Frame, text: &str) -> u32 {
    let texts: Vec<&str> = frame
        .candidates
        .items
        .iter()
        .map(|item| item.text.as_str())
        .collect();
    texts
        .iter()
        .position(|t| *t == text)
        .expect("候选应在当前页") as u32
        + 1
}

#[test]
fn tilde_digit_commits_the_candidate_gloss() {
    let normal = KeyModifiers::default();
    let mut router = router();
    compose(&mut router, "nihao", normal);
    let frame = key(&mut router, 0, None, normal).2;
    let slot = slot_of(&frame, "你好");
    // 按住 ~ 进程序员模式，再按候选序号：上屏那个候选的英文，组句结束。
    let (entered, _, held) = key(&mut router, 0x60, Some('`'), normal);
    assert_eq!(entered, KeyOutcome::Consumed);
    assert!(
        held.candidates
            .items
            .iter()
            .any(|candidate| candidate.gloss.is_some()),
        "进了模式候选就该带英文释义"
    );
    assert_eq!(
        held.notice.as_deref(),
        Some("程序员模式：数字键上屏英文，Esc 退出")
    );
    let (code, character) = digit(slot);
    let (outcome, commit, after) = key(&mut router, code, character, normal);
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("hello"))
    );
    assert!(after.is_empty());
}

#[test]
fn tilde_space_commits_the_first_candidate_gloss() {
    let normal = KeyModifiers::default();
    let mut router = router();
    compose(&mut router, "nihao", normal);
    key(&mut router, 0x60, Some('`'), normal);
    // 空格上屏第一个候选的英文，与按数字等价。
    let (outcome, commit, after) = key(&mut router, 0x20, Some(' '), normal);
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("hello"))
    );
    assert!(after.is_empty());
}

#[test]
fn tilde_digit_without_a_gloss_commits_the_chinese() {
    let normal = KeyModifiers::default();
    let mut router = router();
    compose(&mut router, "shang", normal);
    let frame = key(&mut router, 0, None, normal).2;
    let slot = slot_of(&frame, "上");
    key(&mut router, 0x60, Some('`'), normal);
    // 释义表里没有「上」：模式里按数字还是照常上屏中文，不弹英文。
    let (code, character) = digit(slot);
    let (outcome, commit, after) = key(&mut router, code, character, normal);
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("上"))
    );
    assert!(after.is_empty());
}

#[test]
fn tilde_again_leaves_programmer_mode_and_keeps_the_sentence() {
    let normal = KeyModifiers::default();
    let mut router = router();
    compose(&mut router, "nihao", normal);
    let frame = key(&mut router, 0, None, normal).2;
    let slot = slot_of(&frame, "你好");
    key(&mut router, 0x60, Some('`'), normal);
    // 再按一次 ~ 退出：不上屏东西，组句原样留着，之后数字照常选中文候选。
    let (outcome, commit, after) = key(&mut router, 0x60, Some('`'), normal);
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    let compressing = preedit(&after).replace('\'', "");
    assert_eq!(compressing, "nihao");
    assert!(
        after
            .candidates
            .items
            .iter()
            .all(|candidate| candidate.gloss.is_none())
    );
    assert_eq!(after.notice, None);
    let (code, character) = digit(slot);
    let (outcome, commit, after) = key(&mut router, code, character, normal);
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("你好"))
    );
    assert!(after.is_empty());
}
