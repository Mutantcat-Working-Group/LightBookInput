//! 组句中的快捷键：程序员模式（按住 ~）上屏英文释义，与删候选的修饰键 + 数字。

use crate::support::*;

#[test]
fn tilde_digit_commits_the_candidate_gloss() {
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let slot = slot_of(&frame, "你好");
    // 按住 ~ 进程序员模式，再按候选序号：上屏那个候选的英文释义，组句结束。
    let (entered, _, held) = press(&mut router, tilde());
    assert_eq!(entered, KeyOutcome::Consumed);
    assert!(
        held.candidates.items.iter().any(|c| c.gloss.is_some()),
        "进了模式候选就该带英文释义"
    );
    let (outcome, commit, after) = press(&mut router, digit(slot));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("hello"))
    );
    assert!(after.is_empty());
}

#[test]
fn tilde_space_commits_the_first_candidate_gloss() {
    let mut router = router();
    type_letters(&mut router, "nihao");
    press(&mut router, tilde());
    // 空格上屏第一个候选的英文，与按数字等价。
    let (outcome, commit, after) = press(&mut router, space());
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("hello"))
    );
    assert!(after.is_empty());
}

#[test]
fn tilde_digit_without_a_gloss_commits_the_chinese() {
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "shang");
    let slot = slot_of(&frame, "上");
    press(&mut router, tilde());
    // 样例释义表里没有「上」：模式里按数字还是照常上屏中文，不弹英文。
    let (outcome, commit, after) = press(&mut router, digit(slot));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("上"))
    );
    assert!(after.is_empty());
}

#[test]
fn tilde_again_leaves_programmer_mode_and_keeps_the_sentence() {
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let slot = slot_of(&frame, "你好");
    press(&mut router, tilde());
    // 再按一次 ~ 退出：不上屏东西，组句原样留着，之后数字照常选中文候选。
    let (outcome, commit, after) = press(&mut router, tilde());
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&after), "ni'hao");
    assert!(after.candidates.items.iter().all(|c| c.gloss.is_none()));
    let (outcome, commit, after) = press(&mut router, digit(slot));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("你好"))
    );
    assert!(after.is_empty());
}

#[test]
fn shift_digit_forgets_candidate_and_requeries() {
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let slot = slot_of(&frame, "你好");
    // 缺省 Shift + 数字：删候选（词库词只清学习记录），重新查一遍，组句不变。
    let (outcome, commit, after) = press(&mut router, digit_with(slot, SHIFT));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&after), "ni'hao");
    assert!(!after.candidates.items.is_empty());
}

#[test]
fn unconfigured_modifier_digit_is_not_a_selection() {
    // 删候选改成 Ctrl+Shift：Shift+4 就是普通的 `$`；Win+1 没配到快捷键，归应用。
    let mut router = router_with(RouterConfig {
        delete_keys: KeyModifiers {
            shift: true,
            ..CTRL
        },
        ..RouterConfig::default()
    });
    type_letters(&mut router, "nihao");
    let (outcome, commit, frame) = press(&mut router, digit_with(4, SHIFT));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert!(preedit(&frame).contains('$'), "{}", preedit(&frame));
    let (outcome, commit, _) = press(&mut router, digit_with(1, WIN));
    assert_eq!((outcome, commit), (KeyOutcome::Passthrough, None));
}

#[test]
fn deleting_a_candidate_shows_a_notice_until_next_key() {
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let slot = slot_of(&frame, "你好");

    // 「你好」是词库词且没学习记录，删不掉，但提示照样给出。
    let (outcome, _, after) = press(&mut router, digit_with(slot, SHIFT));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(!after.is_empty(), "删候选后仍在组句");
    let notice = after.notice.as_deref().expect("删候选后应带屏幕提示");
    assert!(notice.contains("你好"), "提示应提到候选词，实际：{notice}");

    let (_, _, next) = press(&mut router, KeyEvent::new(0x28, None, Default::default())); // VK_DOWN
    assert_eq!(next.notice, None, "提示应只活到下一次按键");
}
