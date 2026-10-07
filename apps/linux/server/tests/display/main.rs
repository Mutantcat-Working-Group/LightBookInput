//! 真正上屏才算定论；Linux 回报只按连接代次/上下文/帧验证，不能越过会话与隐私边界。
mod events;
use lightbookinput_core::Engine;
use lightbookinput_dictionary::{Dictionary, EnglishGlossary};
use lightbookinput_linux_server::{Router, RouterConfig};
use lightbookinput_platform::protocol::{
    ClientMessage, KeyEvent, KeyModifiers, PROTOCOL_VERSION, SessionId,
};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

/// 组句只是候选展示，按空格或选字才是上屏，两者不能混为一谈。
fn router() -> (Router, Arc<Mutex<Vec<String>>>) {
    let commits = Arc::new(Mutex::new(Vec::new()));
    let mut engine = Engine::new(Dictionary::parse("你好\tni hao\t100\n").unwrap());
    engine.set_english_glossary(EnglishGlossary::parse("你好\thello\tgreetings\n").unwrap());
    let mut router = Router::new(engine, RouterConfig::default());
    router.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    router.handle(ClientMessage::Privacy {
        session: SessionId(1),
        private: false,
    });
    router.handle_linux(json!({"DisplayReporting": {"session": 1, "identity": {"generation": 2, "context": "first", "revision": 0}}}));
    (router, commits)
}

/// KeyResult 上的 commit 是这个会话真正上屏的文字。
fn key(router: &mut Router, commits: &Arc<Mutex<Vec<String>>>, c: char) -> Value {
    let result = router
        .handle_linux(
            serde_json::to_value(ClientMessage::Key {
                session: SessionId(1),
                event: KeyEvent::new(c as u32, Some(c), KeyModifiers::default()),
            })
            .unwrap(),
        )
        .unwrap();
    if let Some(text) = result["KeyResult"]["commit"].as_str() {
        commits.lock().unwrap().push(text.into());
    }
    result
}

fn compose(router: &mut Router, commits: &Arc<Mutex<Vec<String>>>) -> Value {
    let mut last = Value::Null;
    for c in "nihao".chars() {
        last = key(router, commits, c);
    }
    assert_eq!(
        last["KeyResult"]["frame"]["candidates"]["items"][0]["text"],
        "你好"
    );
    last["KeyResult"]["identity"].clone()
}

#[test]
fn composing_never_commits_until_a_key_lands_the_candidate() {
    let (mut router, commits) = router();
    let identity = compose(&mut router, &commits);
    assert!(commits.lock().unwrap().is_empty(), "组句不算上屏");
    key(&mut router, &commits, ' ');
    assert_eq!(*commits.lock().unwrap(), ["你好"]);
    assert!(identity["revision"].as_u64().unwrap() >= 1);
}

/// 插件把帧 identity 回传回来只做校验；代次/上下文对不上的旧回执不能影响本次上屏。
#[test]
fn stale_panel_reports_are_ignored_without_touching_the_commit() {
    for defect in ["generation", "context", "revision"] {
        let (mut router, commits) = router();
        let mut identity = compose(&mut router, &commits);
        identity[defect] = json!(999);
        router.handle_linux(json!({"DisplayAcknowledged": {"session": 1, "identity": identity}}));
        key(&mut router, &commits, ' ');
        assert_eq!(*commits.lock().unwrap(), ["你好"], "{defect}");
    }
}

#[test]
fn poll_with_an_unchanged_frame_keeps_the_acknowledged_display() {
    // 插件组句期间定时 Poll 取重排结果；帧没变就不是新的展示，身份沿用
    let (mut router, commits) = router();
    let identity = compose(&mut router, &commits);
    let update = router
        .handle_linux(json!({"Poll": {"session": 1}}))
        .unwrap();
    assert_eq!(identity, update["Update"]["identity"]);
    key(&mut router, &commits, ' ');
    assert_eq!(*commits.lock().unwrap(), ["你好"]);
}

/// 失焦再回来、或开新会话，都不会顺手把别处的显示记录算到本会话头上。
#[test]
fn unfocused_or_resumed_session_requires_a_new_display_report() {
    let (mut router, commits) = router();
    let identity = compose(&mut router, &commits);
    router.handle_linux(
        json!({"LinuxEvent": {"session": 1, "event": {"Focus": {"focused": false}}}}),
    );
    router.handle_linux(json!({"DisplayAcknowledged": {"session": 1, "identity": identity}}));
    key(&mut router, &commits, ' ');
    assert_eq!(*commits.lock().unwrap(), ["你好"]);

    router
        .handle_linux(json!({"LinuxEvent": {"session": 1, "event": {"Focus": {"focused": true}}}}));
    let identity = compose(&mut router, &commits);
    router.handle_linux(json!({"DisplayAcknowledged": {"session": 1, "identity": identity}}));
    router.handle(ClientMessage::OpenSession {
        session: SessionId(2),
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    router.handle(ClientMessage::Poll {
        session: SessionId(2),
    });
    key(&mut router, &commits, ' ');
    assert_eq!(*commits.lock().unwrap(), ["你好", "你好"]);
}
