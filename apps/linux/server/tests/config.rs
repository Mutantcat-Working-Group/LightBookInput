//! 实际启动 Server 读资源目录：程序员模式的中→英释义表要跟着候选一起出来。
#![cfg(target_os = "linux")]
#[path = "support/server.rs"]
mod server;
use lightbookinput_platform::protocol::{PROTOCOL_VERSION, read_message, write_message};
use serde_json::{Value, json};
use server::Server;

#[test]
fn english_glossary_rides_along_with_the_candidates() {
    let directory = std::env::temp_dir().join(format!("lightbookinput-gloss-{}", std::process::id()));
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
    write_message(&mut stream, &json!({"LinuxEvent": {"session": 1, "event": {"Capabilities": {"sensitive": false, "password": false, "disabled": false}}}})).unwrap();
    read_message::<_, Value>(&mut stream).unwrap();
    let mut result = Value::Null;
    for character in "nihao".chars() {
        write_message(&mut stream, &json!({"LinuxEvent": {"session": 1, "event": {"Key": {"release": false, "event": {"virtual_key": character as u32, "character": character, "modifiers": {"ctrl": false, "shift": false, "alt": false, "win": false, "caps": false, "english_mode": false}}}}}})).unwrap();
        result = read_message::<_, Value>(&mut stream).unwrap().unwrap();
    }
    let items = result["KeyResult"]["frame"]["candidates"]["items"]
        .as_array()
        .unwrap();
    assert_eq!(items[0]["text"], "你好");
    assert_eq!(items[0]["gloss"], "hello");
    drop(server);
    std::fs::remove_dir_all(directory).unwrap();
}
