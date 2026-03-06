/// 通话响应状态枚举
///
/// 对应 Java CallResponseStatus
/// 用于视频呼叫响应中的接受/拒绝/超时/挂断状态

/// 通话响应状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallResponseStatus {
    /// 超时未接听
    Timeout = -1,
    /// 已拒绝
    Rejected = 0,
    /// 已接听
    Accepted = 1,
    /// 已挂断
    Hangup = 2,
}

impl CallResponseStatus {
    /// 从 i32 值转换
    pub fn of(value: i32) -> Option<Self> {
        match value {
            -1 => Some(CallResponseStatus::Timeout),
            0 => Some(CallResponseStatus::Rejected),
            1 => Some(CallResponseStatus::Accepted),
            2 => Some(CallResponseStatus::Hangup),
            _ => None,
        }
    }
}
