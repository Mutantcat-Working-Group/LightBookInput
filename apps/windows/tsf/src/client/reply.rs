use super::KeyResponse;

/// 一次 [`super::EngineClient::key`] 的应答。
pub enum KeyReply {
    /// 常规处理结果。
    Result(KeyResponse),
}
