use crate::types::{RoomId, UserId};

/// 呼叫响应 VO
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CallResponseVO {
    /// 呼叫发起者 ID
    pub caller_uid: UserId,
    /// 房间 ID
    pub room_id: RoomId,
    /// 接受状态: -1=超时, 0=拒绝, 1=接通, 2=挂断
    pub accepted: i32,
}
