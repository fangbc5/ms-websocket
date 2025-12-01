/// 呼叫请求 VO
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CallRequestVO {
    /// 目标用户 ID
    pub target_uid: u64,
    /// 房间 ID
    pub room_id: u64,
    /// 是否为视频通话 (true=视频, false=语音)
    pub is_video: bool,
}
