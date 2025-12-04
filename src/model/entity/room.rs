use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::RoomId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: RoomId,
    pub room_type: u8, // 1: 群聊 2: 单聊
    pub hot_flag: u8, // 是否全员展示 0否 1是
    pub active_time: DateTime<Utc>,
    pub last_msg_id: u64,
    pub ext_json: Option<String>,
    pub create_time: DateTime<Utc>,
    pub create_by: u64,
    pub update_time: DateTime<Utc>,
    pub update_by: u64,
    pub is_del: u8,
    pub tenant_id: u64,
}

impl Room {
    /// 获取房间类型（兼容 Java 的 getType()）
    pub fn get_type(&self) -> u8 {
        self.room_type
    }

    /// 获取房间 ID（兼容 Java 的 getId()）
    pub fn get_id(&self) -> RoomId {
        self.id
    }

    /// 获取租户 ID（兼容 Java 的 getTenantId()）
    pub fn get_tenant_id(&self) -> u64 {
        self.tenant_id
    }
}