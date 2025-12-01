/// 会话恢复服务
///
/// 功能：
/// 1. 在用户重连时恢复其视频会话状态
/// 2. 重新加入用户之前所在的视频房间
/// 3. 确保用户状态一致性
///
/// 使用场景：
/// - 用户网络断开后重新连接
/// - 用户切换设备后重新登录
/// - 服务端重启后恢复用户状态
use crate::service::video_chat_service::VideoChatService;
use std::sync::Arc;

/// 会话恢复服务
pub struct SessionRecoveryService {
    video_service: Arc<VideoChatService>,
}

impl SessionRecoveryService {
    /// 创建新的会话恢复服务
    pub fn new(video_service: Arc<VideoChatService>) -> Self {
        Self { video_service }
    }

    /// 恢复用户会话
    pub async fn recover_user_sessions(&self, uid: i64) -> anyhow::Result<()> {
        // 获取用户所有房间
        let rooms = self.video_service.get_user_rooms(uid).await?;

        // 重新加入所有房间
        for room_id in rooms {
            if let Some(room) = self.video_service.get_room_metadata(room_id).await? {
                let _ = self.video_service.join_room(uid, room).await;
            }
        }

        Ok(())
    }
}
