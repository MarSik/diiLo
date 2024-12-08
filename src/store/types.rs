use std::{collections::HashSet, path::PathBuf, rc::Rc};

use chrono::{DateTime, FixedOffset};
use multimap::MultiMap;

use crate::store::serializer::{deserialize_labels, serialize_labels};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ObjectType {
    Part,
    Source,
    Project,
    Location,
}

// How should the system track a part type
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize,
)]
#[serde(rename_all = "lowercase")]
pub enum CountTracking {
    // Track as simple count, pieces are equivalent and can be taken
    // from a heap or placed to a heap of pieces as needed
    #[default]
    Count,
    // Track as pieces with specific length (volume, count)
    // Once broken to smaller pieces, the pieces cannot be joined back together
    Pieces,
    // Track as separate items, each item is unique and can be identified
    // Mostly used for pieces that have their own serial number
    Unique,
}

#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize,
)]
#[serde(rename_all = "lowercase")]
pub enum CountUnit {
    // Simple unit-less count
    #[default]
    Piece,
    // Length
    Centimeter,
    MilliMeter,
    Meter,
    // Volume
    Liter,
    DeciLiter,
    MilliLiter,
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct PartMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(default)]
    pub name: String,

    #[serde(alias = "mfgid")]
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub manufacturer_id: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub manufacturer: String,

    #[serde(default)]
    #[serde(serialize_with = "serialize_labels")]
    #[serde(deserialize_with = "deserialize_labels")]
    pub labels: MultiMap<String, String>,

    #[serde(default)]
    #[serde(alias = "attrs", alias = "attr")]
    #[serde(serialize_with = "serialize_labels")]
    #[serde(deserialize_with = "deserialize_labels")]
    pub attributes: MultiMap<String, String>,

    #[serde(default)]
    pub types: HashSet<ObjectType>,

    #[serde(default)]
    pub summary: String,

    #[serde(default)]
    pub track: CountTracking,

    // Can this part be released once used?
    // consumable: true means it is lost after use and
    // cannot be recovered
    #[serde(default)]
    pub consumable: bool,

    // The smallest counting unit, pieces, meters, cm, mm, liters, ..
    #[serde(default)]
    pub unit: CountUnit,
}

#[derive(Debug, Clone, Default)]
pub struct Part {
    pub id: PartId,
    pub filename: Option<PathBuf>,
    pub metadata: PartMetadata,
    pub content: String,
}

pub fn is_false(v: &bool) -> bool {
    !*v
}

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
pub(crate) struct LedgerEntryDto {
    #[serde(alias = "t")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) time: Option<String>,

    #[serde(alias = "tx")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) transaction: Option<String>,

    #[serde(alias = "n", alias = "c")]
    pub(super) count: usize,

    #[serde(rename = "part")]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(super) part_id: String,

    #[serde(
        rename = "location",
        alias = "destination",
        alias = "dst",
        alias = "to"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) location_id: Option<String>,

    #[serde(rename = "project", alias = "proj")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) project_id: Option<String>,

    #[serde(rename = "source", alias = "from", alias = "src", alias = "fr")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) source_id: Option<String>,

    #[serde(rename = "take", alias = "move", alias = "m", alias = "-")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    pub(super) cmd_take: bool,

    #[serde(rename = "store", alias = "receive", alias = "a", alias = "+")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    pub(super) cmd_store: bool,

    #[serde(rename = "require", alias = "req", alias = "?")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    pub(super) cmd_require: bool,

    #[serde(rename = "solder", alias = "s")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    pub(super) cmd_solder: bool,

    #[serde(rename = "unsolder", alias = "u")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    pub(super) cmd_unsolder: bool,

    #[serde(rename = "order", alias = "o")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    pub(super) cmd_order: bool,

    #[serde(rename = "cancel", alias = "co")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    pub(super) cmd_cancel_order: bool,

    #[serde(rename = "deliver", alias = "d")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    pub(super) cmd_deliver: bool,

    #[serde(rename = "return", alias = "ret", alias = "send")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    pub(super) cmd_return: bool,

    #[serde(rename = "correct", alias = "set", alias = "=")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    pub(super) cmd_set: bool,
}

pub type PartId = Rc<str>;
pub type LocationId = PartId;
pub type SourceId = Rc<str>;

#[derive(Debug)]
pub struct LedgerEntry {
    pub t: DateTime<FixedOffset>,
    pub count: usize,
    pub part: PartId,
    pub ev: LedgerEvent,
}

#[derive(Debug)]
pub enum LedgerEvent {
    TakeFrom(LocationId),
    StoreTo(LocationId),
    ForceCount(LocationId),
    RequireIn(LocationId),
    OrderFrom(SourceId),
    CancelOrderFrom(SourceId),
    DeliverFrom(SourceId),
    ReturnTo(SourceId),
    UnsolderFrom(LocationId),
    SolderTo(LocationId),
    RequireInProject(LocationId),
}
