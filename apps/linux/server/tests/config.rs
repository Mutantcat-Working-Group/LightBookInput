//! 实际启动 Server 读资源目录：中→英释义表只在程序员模式开着时挂到候选上。
#![cfg(target_os = "linux")]
#[path = "support/server.rs"]
mod server;
use lightbookinput_platform::protocol::{PROTOCOL_VERSION, read_message, write_message};
use serde_json::{Value, json};
use server::Server;

/// 敲键时按下的修饰键：都不按。
fn modifiers() -> Value {
    json!({
        "ctrl": false,
        "shift": false,
        "alt": false,
        "win": false,
        "caps": false,
        "english_mode": false
    })
}

type Stream = std::os::unix::net::UnixStream;

/// 发一个 Linux 事件，收回服端的回复。
fn press(stream: &mut Stream, event: Value) -> Value {
    write_message(
        stream,
        &json!({"LinuxEvent": {"session": 1, "event": event}}),
    )
    .unwrap();
    read_message::<_, Value>(stream).unwrap().unwrap()
}

/// 敲一下键：Linux 前端的虚拟键码与字符直送。
fn key(virtual_key: u32, character: &str) -> Value {
    json!({"Key": {"release": false, "event": {"virtual_key": virtual_key, "character": character, "modifiers": modifiers()}}})
}

/// 敲一句拼音，取最后一帧。
fn compose(stream: &mut Stream, letters: &str) -> Value {
    let mut frame = Value::Null;
    for character in letters.chars() {
        frame = press(stream, key(character as u32, &character.to_string()))["KeyResult"]["frame"]
            .clone();
    }
    frame
}

fn items(frame: &Value) -> &[Value] {
    frame["candidates"]["items"].as_array().unwrap().as_slice()
}

#[test]
fn the_glossary_only_shows_up_while_the_tilde_is_held() {
    let directory =
        std::env::temp_dir().join(format!("lightbookinput-gloss-{}", std::process::id()));
    std::fs::create_dir_all(directory.join("resources/assets/glossary")).unwrap();
    std::fs::write(
        directory.join("resources/assets/glossary/glossary-en.tsv"),
        "你好\thello\n",
    )
    .unwrap();
    let mut server = Server::start(directory.clone());
    let mut stream = server.connect();
    write_message(
        &mut stream,
        &json!({"OpenSession": {"session": 1, "app": null, "protocol": PROTOCOL_VERSION}}),
    )
    .unwrap();
    read_message::<_, Value>(&mut stream).unwrap();
    write_message(
        &mut stream,
        &json!({"LinuxHello": {"version": 3, "session": 1, "generation": 1, "context": "gloss-test"}}),
    )
    .unwrap();
    read_message::<_, Value>(&mut stream).unwrap();
    press(
        &mut stream,
        json!({"Capabilities": {"sensitive": false, "password": false, "disabled": false}}),
    );

    // 平常打字：候选只有文字，没有那一行英文。
    let frame = compose(&mut stream, "nihao");
    assert_eq!(items(&frame)[0]["text"], "你好");
    assert_eq!(items(&frame)[0]["gloss"], Value::Null);
    assert_eq!(frame["notice"], Value::Null);

    // 按住 `~`：同一条组句的候选挂上英文，状态行换成模式提示。
    let held = press(&mut stream, key(0x60, "`"))["KeyResult"]["frame"].clone();
    assert_eq!(items(&held)[0]["text"], "你好");
    assert_eq!(items(&held)[0]["gloss"], "hello");
    assert_eq!(held["notice"], "程序员模式：数字键上屏英文，Esc 退出");

    // 数字键上屏那一行英文，组句结束。
    let committed = press(&mut stream, key(0x31, "1"))["KeyResult"].clone();
    assert_eq!(committed["commit"], "hello");
    assert!(committed["frame"]["preedit"].as_array().unwrap().is_empty());
    assert!(items(&committed["frame"]).is_empty());

    drop(server);
    std::fs::remove_dir_all(directory).unwrap();
}
