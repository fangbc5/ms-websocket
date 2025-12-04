use crate::types::UserId;

/// 拒绝呼叫 VO
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CallRejectedVO {
    /// 拒绝呼叫的用户 ID
    pub rejected_by: UserId,
}
