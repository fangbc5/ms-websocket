/// 呼叫请求 VO
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CallReqVO {
    /// 呼叫发起者 ID
    pub caller_uid: String,
    /// 目标用户 ID
    pub target_uid: String,
    /// 房间 ID
    pub room_id: String,
    /// 是否为视频通话 (true=视频, false=语音)
    pub is_video: bool,
}
