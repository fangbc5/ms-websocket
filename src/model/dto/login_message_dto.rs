/// 将扫码登录返回信息推送给所有横向扩展的服务
///
/// 用于在多个服务实例之间同步登录状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoginMessageDTO {
    /// 用户 ID
    pub uid: u64,
    /// 响应码
    pub code: i32,
}
