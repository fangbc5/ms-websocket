use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: i64,
    pub room_type: i32, // 1: 群聊 2: 单聊
    pub hot_flag: i32, // 是否全员展示 0否 1是
    pub active_time: DateTime<Utc>,
    pub last_msg_id: i64,
    pub ext_json: Option<String>,
    pub create_time: DateTime<Utc>,
    pub create_by: i64,
    pub update_time: DateTime<Utc>,
    pub update_by: i64,
    pub is_del: i32,
    pub tenant_id: i64,
}

impl Room {
    /// 获取房间类型（兼容 Java 的 getType()）
    pub fn get_type(&self) -> i32 {
        self.room_type
    }

    /// 获取房间 ID（兼容 Java 的 getId()）
    pub fn get_id(&self) -> i64 {
        self.id
    }

    /// 获取租户 ID（兼容 Java 的 getTenantId()）
    pub fn get_tenant_id(&self) -> i64 {
        self.tenant_id
    }
}