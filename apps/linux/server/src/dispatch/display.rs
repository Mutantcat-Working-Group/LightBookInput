//! Linux 版本化显示初始化，Windows 协议保持不变。
use super::Router;
use crate::protocol::{DisplayIdentity, LINUX_UI_PROTOCOL, LinuxEvent, LinuxRequest};
use lightbookinput_platform::protocol::{ClientMessage, ServerMessage, SessionId};
use serde_json::{Value, json};

impl Router {
    pub fn display_settings(&self) -> Value {
        json!({"version": LINUX_UI_PROTOCOL, "preedit": self.config.preedit})
    }

    pub fn handle_linux(&mut self, value: Value) -> Option<Value> {
        if let Some(body) = value.get("DisplayReporting") {
            let session: SessionId = serde_json::from_value(body.get("session")?.clone()).ok()?;
            let identity: DisplayIdentity =
                serde_json::from_value(body.get("identity")?.clone()).ok()?;
            let info = self.sessions.get_mut(&session)?;
            info.display_identity = Some(identity);
            info.last_frame = None;
            return None;
        }
        let response = if let Some(body) = value.get("LinuxEvent") {
            let request: LinuxRequest = serde_json::from_value(body.clone()).ok()?;
            if let LinuxEvent::Candidate { identity, .. } | LinuxEvent::Page { identity, .. } =
                &request.event
                && !self.valid_panel_event(request.session, identity)
            {
                // 旧鼠标事件不能改变其他会话的焦点或显示身份。
                return Some(json!({"Ignored": {"session": request.session}}));
            }
            self.linux_event(request)?
        } else {
            let message: ClientMessage = serde_json::from_value(value).ok()?;
            self.handle(message)?
        };
        let mut value = serde_json::to_value(&response).ok()?;
        match &response {
            ServerMessage::KeyResult { session, frame, .. }
            | ServerMessage::Update { session, frame } => {
                let polled = matches!(response, ServerMessage::Update { .. });
                let info = self.sessions.get_mut(session)?;
                if let Some(identity) = &mut info.display_identity {
                    // 插件组句期间定时 Poll：帧没变就不是新的展示，身份沿用，已回报的曝光照算
                    if polled && info.last_frame.as_ref() == Some(frame) {
                        value.as_object_mut()?.values_mut().next()?["identity"] = json!(identity);
                        return Some(value);
                    }
                    self.display_revision += 1;
                    identity.revision = self.display_revision;
                    info.last_frame = Some(frame.clone());
                    value.as_object_mut()?.values_mut().next()?["identity"] = json!(identity);
                }
            }
            ServerMessage::Committed { session, .. } => {
                self.display_revision += 1;
                if let Some(info) = self.sessions.get_mut(session) {
                    info.last_frame = None;
                    if let Some(identity) = &mut info.display_identity {
                        identity.revision = self.display_revision;
                    }
                }
            }
            _ => {}
        }
        Some(value)
    }
}
