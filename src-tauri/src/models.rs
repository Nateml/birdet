use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Pack {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub bird_count: i64,
}

