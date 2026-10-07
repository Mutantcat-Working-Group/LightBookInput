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

/// 松一下键：用来验证松键不退出程序员模式。
fn release(stream: &mut Stream, virtual_key: u32, character: &str) -> Value {
    press(
        stream,
        json!({"Key": {"release": true, "event": {"virtual_key": virtual_key, "character": character, "modifiers": modifiers()}}}),
    )
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

/// 起一个带着中英释义表的 Server（`你好` → `hello`）并握好手，资源目录由调用方清理。
/// 会话挂在连接上：握手用的连接必须一起交出去，断开就等于服务端 CloseSession，之后按键只会收到 Ignored。
fn gloss_server(directory: &std::path::Path) -> (Server, Stream) {
    std::fs::create_dir_all(directory.join("resources/assets/glossary")).unwrap();
    std::fs::write(
        directory.join("resources/assets/glossary/glossary-en.tsv"),
        "你好\thello\n",
    )
    .unwrap();
    let mut server = Server::start(directory.to_path_buf());
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
    (server, stream)
}

#[test]
fn the_glossary_shows_up_once_the_tilde_is_tapped() {
    let directory =
        std::env::temp_dir().join(format!("lightbookinput-gloss-{}", std::process::id()));
    let (mut server, mut stream) = gloss_server(&directory);

    // 平常打字：候选只有文字，没有那一行英文。
    let frame = compose(&mut stream, "nihao");
    assert_eq!(items(&frame)[0]["text"], "你好");
    assert_eq!(items(&frame)[0]["gloss"], Value::Null);
    assert_eq!(frame["notice"], Value::Null);

    // 点按 `~`：同一条组句的候选挂上英文，状态行换成模式提示。
    let toggled = press(&mut stream, key(0x60, "`"))["KeyResult"]["frame"].clone();
    assert_eq!(items(&toggled)[0]["text"], "你好");
    assert_eq!(items(&toggled)[0]["gloss"], "hello");
    assert_eq!(
        toggled["notice"],
        "程序员模式：方向键选释义，数字 / 空格上屏，Esc 退出"
    );

    // 数字键上屏那一行英文，组句结束。
    let committed = press(&mut stream, key(0x31, "1"))["KeyResult"].clone();
    assert_eq!(committed["commit"], "hello");
    assert!(committed["frame"]["preedit"].as_array().unwrap().is_empty());
    assert!(items(&committed["frame"]).is_empty());

    drop(server);
    std::fs::remove_dir_all(directory).unwrap();
}

/// 松键只收自动重复标记，不退出模式；退出要再按一次 `~`。
/// 自动重复的连发按下也在这里被吞掉，否则长按会把开关来回切。
#[test]
fn releasing_the_tilde_does_not_leave_programmer_mode() {
    let directory = std::env::temp_dir().join(format!(
        "lightbookinput-gloss-release-{}-{}",
        std::process::id(),
        line!()
    ));
    let (mut server, mut stream) = gloss_server(&directory);

    compose(&mut stream, "nihao");
    let entered = press(&mut stream, key(0x60, "`"))["KeyResult"]["frame"].clone();
    assert_eq!(items(&entered)[0]["gloss"], "hello");

    // 松键：模式还开着，释义与拼音都留着。
    let released = release(&mut stream, 0x60, "`")["KeyResult"]["frame"].clone();
    assert_eq!(items(&released)[0]["gloss"], "hello");
    assert_eq!(
        released["notice"],
        "程序员模式：方向键选释义，数字 / 空格上屏，Esc 退出"
    );
    assert!(!released["preedit"].as_array().unwrap().is_empty());

    // 再按一次 `~`：退出，组句原样留着，之后数字照常选中文候选。
    let left = press(&mut stream, key(0x60, "`"))["KeyResult"]["frame"].clone();
    assert_eq!(items(&left)[0]["gloss"], Value::Null);
    assert_eq!(left["notice"], Value::Null);
    assert!(!left["preedit"].as_array().unwrap().is_empty());

    drop(server);
    std::fs::remove_dir_all(directory).unwrap();
}
