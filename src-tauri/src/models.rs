use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Pack {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub bird_count: i64,
}

/// On-disk shape of an exported pack. Carries only species identity (no audio);
/// import re-downloads any missing recordings from Xeno-Canto.
#[derive(Serialize, Deserialize, Debug)]
pub struct PackFile {
    pub format: String, // always "birdet-pack"
    pub version: u32,   // schema version (1)
    pub name: String,
    pub birds: Vec<PackFileBird>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PackFileBird {
    pub ebird_code: Option<String>,
    pub common_name: String,
    pub scientific_name: String,
}

/// Outcome of importing a pack file.
#[derive(Serialize, Debug)]
pub struct PackImportResult {
    pub pack_id: String,
    pub name: String,
    pub linked_existing: i64, // birds already in the library, just linked
    pub downloaded_new: i64,  // birds newly fetched from Xeno-Canto
    pub skipped: i64,         // couldn't be resolved or downloaded
    pub recordings_added: i64,
}

