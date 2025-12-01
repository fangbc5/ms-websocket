/// 呼叫超时 VO
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CallTimeoutVO {
    /// 未接听的用户 ID
    pub target_uid: String,
}
