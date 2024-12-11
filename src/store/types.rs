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
    #[serde(rename = "pc")]
    Piece,
    // Length
    #[serde(rename = "cm")]
    Centimeter,
    #[serde(rename = "mm")]
    MilliMeter,
    #[serde(rename = "m")]
    Meter,
    // Volume
    #[serde(rename = "l")]
    Liter,
    #[serde(rename = "dl")]
    DeciLiter,
    #[serde(rename = "ml")]
    MilliLiter,
}

impl std::fmt::Display for CountUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CountUnit::Piece => f.write_str("pc"),
            CountUnit::Centimeter => f.write_str("cm"),
            CountUnit::MilliMeter => f.write_str("mm"),
            CountUnit::Meter => f.write_str("m"),
            CountUnit::Liter => f.write_str("l"),
            CountUnit::DeciLiter => f.write_str("dl"),
            CountUnit::MilliLiter => f.write_str("mm"),
        }
    }
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

#[derive(Default, Debug, Clone)]
pub struct Part {
    pub id: PartTypeId,
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

    #[serde(rename = "size", alias = "len", alias = "l")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub(super) piece_size: Option<usize>,
}

pub type PartTypeId = Rc<str>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum PartId {
    Simple(PartTypeId),
    Piece(PartTypeId, usize), // .1 is piece counter and must represent the size of the piece at least per .0 type
    Unique(PartTypeId, Rc<str>), // .1 is a serial number
}

impl PartId {
    pub fn part_type(&self) -> &PartTypeId {
        match self {
            PartId::Simple(rc) => rc,
            PartId::Piece(rc, _) => rc,
            PartId::Unique(rc, _) => rc,
        }
    }

    pub fn to_simple(&self) -> Self {
        Self::Simple(self.part_type().clone())
    }

    pub fn piece(&self, l: usize) -> Self {
        Self::Piece(self.part_type().clone(), l)
    }

    pub fn piece_size(&self) -> usize {
        match self {
            PartId::Simple(_) => 1,
            PartId::Piece(_, s) => *s,
            PartId::Unique(_, _) => 1,
        }
    }

    pub fn piece_size_option(&self) -> Option<usize> {
        match self {
            PartId::Simple(_) => None,
            PartId::Piece(_, s) => Some(*s),
            PartId::Unique(_, _) => None,
        }
    }

    pub fn subname(&self) -> Option<String> {
        match self {
            PartId::Simple(_) => None,
            PartId::Piece(_, s) => Some(s.to_string()),
            PartId::Unique(_, s) => Some(s.to_string()),
        }
    }

    // Create ID with sizing information if the current ID supports sizing
    // This updates Piece sizing, but keeps other ID types intact
    pub fn maybe_sized(&self, l: usize) -> Self {
        match self {
            PartId::Simple(_) => self.clone(),
            PartId::Piece(_, _) => self.piece(l),
            PartId::Unique(_, _) => self.clone(),
        }
    }

    pub fn conditional_piece(&self, is_piece: bool, l: usize) -> Self {
        if is_piece {
            self.piece(l)
        } else {
            self.clone()
        }
    }

    pub(crate) fn to_piece(&self, default: usize) -> PartId {
        match self {
            PartId::Simple(rc) => PartId::Piece(rc.clone(), default),
            PartId::Piece(rc, 0) => PartId::Piece(rc.clone(), default),
            PartId::Piece(_, _) => self.clone(),
            PartId::Unique(rc, _) => PartId::Piece(rc.clone(), default),
        }
    }

    pub(crate) fn to_unique(&self) -> PartId {
        match self {
            PartId::Simple(_) => {
                todo!("No way to generate proper serial number for new unique part")
            }
            PartId::Piece(_, _) => {
                todo!("No way to generate proper serial number for new unique part")
            }
            PartId::Unique(_, _) => self.clone(),
        }
    }
}

impl From<&str> for PartId {
    fn from(value: &str) -> Self {
        PartId::Simple(value.into())
    }
}

impl From<&String> for PartId {
    fn from(value: &String) -> Self {
        PartId::Simple(value.as_str().into())
    }
}

impl From<String> for PartId {
    fn from(value: String) -> Self {
        PartId::Simple(value.as_str().into())
    }
}

impl From<PartTypeId> for PartId {
    fn from(value: PartTypeId) -> Self {
        PartId::Simple(PartTypeId::clone(&value))
    }
}

impl From<&PartTypeId> for PartId {
    fn from(value: &PartTypeId) -> Self {
        PartId::Simple(PartTypeId::clone(value))
    }
}

impl std::fmt::Display for PartId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PartId::Simple(rc) => rc.fmt(f),
            PartId::Piece(rc, _) => rc.fmt(f),
            PartId::Unique(rc, serial) => f.write_fmt(format_args!("{} [{}]", rc, serial)),
        }
    }
}

pub type LocationId = PartId;
pub type ProjectId = PartId;
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
