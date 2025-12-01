/// 扫码成功对象，推送给用户的消息对象
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScanSuccessMessageDTO {
    /// 推送的响应码
    pub code: i32,
}
