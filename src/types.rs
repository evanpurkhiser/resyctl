use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotId {
    pub config_id: String,
    pub day: String,
    pub party_size: u8,
    pub venue_id: i64,
    pub start: Option<String>,
    pub slot_type: Option<String>,
}
