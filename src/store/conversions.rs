use chrono::{DateTime, Local};

use super::{LedgerEntry, LedgerEntryDto, LedgerEvent};

impl From<&LedgerEntryDto> for LedgerEntry {
    fn from(val: &LedgerEntryDto) -> LedgerEntry {
        //The parse_datetime call takes 5 ms and is extremely slow!
        //let t = self.time.clone().map(parse_datetime).unwrap().unwrap();
        let t = val
            .time
            .as_deref()
            .map(DateTime::parse_from_rfc3339)
            .unwrap_or_else(|| Ok(Local::now().fixed_offset()))
            .unwrap();

        if val.cmd_store {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::StoreTo(val.location_id.clone().unwrap().into()),
            }
        } else if val.cmd_take {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::TakeFrom(val.location_id.clone().unwrap().into()),
            }
        } else if val.cmd_deliver {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::DeliverFrom(
                    val.source_id
                        .as_ref()
                        .or(val.location_id.as_ref())
                        .unwrap()
                        .as_str()
                        .into(),
                ),
            }
        } else if val.cmd_return {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::ReturnTo(
                    val.source_id
                        .as_ref()
                        .or(val.location_id.as_ref())
                        .unwrap()
                        .as_str()
                        .into(),
                ),
            }
        } else if val.cmd_order {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::OrderFrom(
                    val.source_id
                        .as_ref()
                        .or(val.location_id.as_ref())
                        .unwrap()
                        .as_str()
                        .into(),
                ),
            }
        } else if val.cmd_cancel_order {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::CancelOrderFrom(
                    val.source_id
                        .as_ref()
                        .or(val.location_id.as_ref())
                        .unwrap()
                        .as_str()
                        .into(),
                ),
            }
        } else if val.cmd_require && val.location_id.is_some() {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::RequireIn(val.location_id.clone().unwrap().into()),
            }
        } else if val.cmd_require && val.project_id.is_some() {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::RequireInProject(val.project_id.clone().unwrap().into()),
            }
        } else if val.cmd_require && val.source_id.is_some() {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::OrderFrom(val.source_id.clone().unwrap().into()),
            }
        } else if val.cmd_solder {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::SolderTo(
                    val.project_id
                        .as_ref()
                        .or(val.location_id.as_ref())
                        .unwrap()
                        .as_str()
                        .into(),
                ),
            }
        } else if val.cmd_unsolder {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::UnsolderFrom(
                    val.project_id
                        .as_ref()
                        .or(val.location_id.as_ref())
                        .unwrap()
                        .as_str()
                        .into(),
                ),
            }
        } else if val.cmd_set {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::ForceCount(val.location_id.clone().unwrap().into()),
            }
        } else {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::TakeFrom(val.location_id.clone().unwrap().into()),
            }
        }
    }
}

impl From<&LedgerEntry> for LedgerEntryDto {
    fn from(entry: &LedgerEntry) -> Self {
        match &entry.ev {
            LedgerEvent::TakeFrom(location) => LedgerEntryDto {
                time: Some(entry.t.to_rfc3339()),
                transaction: None,
                count: entry.count,
                part_id: entry.part.part_type().to_string(),
                location_id: Some(location.part_type().to_string()),
                cmd_take: true, // TODO check if location is a project -> unsolder
                ..Default::default()
            },
            LedgerEvent::StoreTo(location) => LedgerEntryDto {
                time: Some(entry.t.to_rfc3339()),
                transaction: None,
                count: entry.count,
                part_id: entry.part.part_type().to_string(),
                location_id: Some(location.part_type().to_string()),
                cmd_store: true, // TODO check if location is a project -> solder
                ..Default::default()
            },
            LedgerEvent::UnsolderFrom(location) => LedgerEntryDto {
                time: Some(entry.t.to_rfc3339()),
                transaction: None,
                count: entry.count,
                part_id: entry.part.part_type().to_string(),
                location_id: Some(location.part_type().to_string()),
                cmd_unsolder: true,
                ..Default::default()
            },
            LedgerEvent::SolderTo(location) => LedgerEntryDto {
                time: Some(entry.t.to_rfc3339()),
                transaction: None,
                count: entry.count,
                part_id: entry.part.part_type().to_string(),
                location_id: Some(location.part_type().to_string()),
                cmd_solder: true,
                ..Default::default()
            },
            LedgerEvent::ForceCount(location) => LedgerEntryDto {
                time: Some(entry.t.to_rfc3339()),
                transaction: None,
                count: entry.count,
                part_id: entry.part.part_type().to_string(),
                location_id: Some(location.part_type().to_string()),
                cmd_set: true,
                ..Default::default()
            },
            LedgerEvent::RequireIn(location) => LedgerEntryDto {
                time: Some(entry.t.to_rfc3339()),
                transaction: None,
                count: entry.count,
                part_id: entry.part.part_type().to_string(),
                location_id: Some(location.part_type().to_string()),
                cmd_require: true,
                ..Default::default()
            },
            LedgerEvent::RequireInProject(project_id) => LedgerEntryDto {
                time: Some(entry.t.to_rfc3339()),
                transaction: None,
                count: entry.count,
                part_id: entry.part.part_type().to_string(),
                project_id: Some(project_id.part_type().to_string()),
                cmd_require: true,
                ..Default::default()
            },
            LedgerEvent::ReturnTo(source_id) => LedgerEntryDto {
                time: Some(entry.t.to_rfc3339()),
                transaction: None,
                count: entry.count,
                part_id: entry.part.part_type().to_string(),
                source_id: Some(source_id.to_string()),
                cmd_return: true,
                ..Default::default()
            },
            LedgerEvent::DeliverFrom(source) => LedgerEntryDto {
                time: Some(entry.t.to_rfc3339()),
                transaction: None,
                count: entry.count,
                part_id: entry.part.part_type().to_string(),
                source_id: Some(source.to_string()),
                cmd_deliver: true,
                ..Default::default()
            },
            LedgerEvent::OrderFrom(source) => LedgerEntryDto {
                time: Some(entry.t.to_rfc3339()),
                transaction: None,
                count: entry.count,
                part_id: entry.part.part_type().to_string(),
                source_id: Some(source.to_string()),
                cmd_order: true,
                ..Default::default()
            },
            LedgerEvent::CancelOrderFrom(source) => LedgerEntryDto {
                time: Some(entry.t.to_rfc3339()),
                transaction: None,
                count: entry.count,
                part_id: entry.part.part_type().to_string(),
                source_id: Some(source.to_string()),
                cmd_cancel_order: true,
                ..Default::default()
            },
        }
    }
}
