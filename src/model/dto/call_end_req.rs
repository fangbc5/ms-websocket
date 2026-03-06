use crate::types::{RoomId, UserId};
use serde::{Deserialize, Serialize};

/// 音视频消息元数据 DTO
///
/// 对应 Java CallEndReq，用于通过 Kafka 发送到 IM 服务处理通话结束/开始消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallEndReq {
    /// 是否为开始消息（群聊开始通话时为 true）
    pub begin: bool,
    /// 操作人 ID
    pub uid: Option<UserId>,
    /// 房间 ID
    pub room_id: RoomId,
    /// 租户 ID
    pub tenant_id: Option<i64>,
    /// 是否为群聊
    pub is_group: Option<bool>,
    /// 媒体类型（true=视频, false=语音）
    pub medium_type: Option<bool>,
    /// 房间创建者 ID
    pub creator: Option<UserId>,
    /// 通话开始时间（毫秒时间戳）
    pub start_time: Option<i64>,
    /// 通话结束时间（毫秒时间戳）
    pub end_time: Option<i64>,
    /// 通话状态
    /// @see CallStatusEnum: ONGOING, COMPLETED, REJECTED, DROPPED, CANCEL, TIMEOUT, FAILED, RINGING, MANAGER_CLOSE
    pub state: String,
}

impl CallEndReq {
    /// 创建通话结束请求（房间关闭时）
    ///
    /// 对应 Java: new CallEndReq(uid, roomId, tenantId, type.equals(1), mediumType, creator, startTime, endTime, reason)
    pub fn new_end(
        uid: Option<UserId>,
        room_id: RoomId,
        tenant_id: Option<i64>,
        is_group: bool,
        medium_type: Option<bool>,
        creator: Option<UserId>,
        start_time: Option<i64>,
        state: String,
    ) -> Self {
        Self {
            begin: false,
            uid,
            room_id,
            tenant_id,
            is_group: Some(is_group),
            medium_type,
            creator,
            start_time,
            end_time: Some(chrono::Utc::now().timestamp_millis()),
            state,
        }
    }

    /// 创建群聊通话开始消息
    ///
    /// 对应 Java: new CallEndReq(uid, creator, roomId, tenantId, startTime, reason)
    pub fn new_start(
        uid: UserId,
        creator: Option<UserId>,
        room_id: RoomId,
        tenant_id: Option<i64>,
        state: String,
    ) -> Self {
        Self {
            begin: true,
            uid: Some(uid),
            room_id,
            tenant_id,
            is_group: None,
            medium_type: Some(false),
            creator,
            start_time: Some(chrono::Utc::now().timestamp_millis()),
            end_time: None,
            state,
        }
    }
}
