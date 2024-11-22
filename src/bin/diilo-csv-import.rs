use std::{collections::HashSet, fs::File, path::PathBuf};

use diilo::store::{LedgerEntry, LedgerEvent, Part, PartMetadata, Store};
use multimap::MultiMap;

#[derive(Debug, serde::Deserialize)]
struct CsvPartDto {
    name: String,
    manufacturer: String,
    footprint: String,
    category: String,
    summary: String,
    description: String,
}

#[derive(Debug, serde::Deserialize)]
struct CsvLedgerDto {
    location: String,
    source: String,
    project: String,
    part: String,
    added: Option<usize>,
    removed: Option<usize>,
    t: String, // DateTime Local
}

fn main() -> anyhow::Result<()> {
    let mut store = Store::new(PathBuf::from("output"))?;
    store.open_ledger(None)?;

    let csv_ledger = File::open("ledger.csv")?;
    let csv_parts = File::open("parts.csv")?;

    let mut ledger_reader = csv::Reader::from_reader(csv_ledger);
    let mut part_reader = csv::Reader::from_reader(csv_parts);

    let mut locations = HashSet::new();
    let mut sources = HashSet::new();
    let mut projects = HashSet::new();

    let mut events = vec![];
    for csv_ledger in ledger_reader.deserialize() {
        print!("L");
        let csv_ledger: CsvLedgerDto = csv_ledger?;

        if csv_ledger.added.unwrap_or(0) > 0 && !csv_ledger.location.is_empty() {
            let ledger = LedgerEntry {
                t: parse_datetime::parse_datetime(&csv_ledger.t)?,
                count: csv_ledger.added.unwrap(),
                ev: LedgerEvent::StoreTo(
                    store
                        .name_to_id(csv_ledger.location.as_str())
                        .as_str()
                        .into(),
                ),
                part: store.name_to_id(&csv_ledger.part).as_str().into(),
            };
            locations.insert(csv_ledger.location.clone());
            events.push(ledger);
        }

        if csv_ledger.removed.unwrap_or(0) > 0 && !csv_ledger.location.is_empty() {
            let ledger = LedgerEntry {
                t: parse_datetime::parse_datetime(&csv_ledger.t)?,
                count: csv_ledger.removed.unwrap(),
                ev: LedgerEvent::TakeFrom(
                    store
                        .name_to_id(csv_ledger.location.as_str())
                        .as_str()
                        .into(),
                ),
                part: store.name_to_id(&csv_ledger.part).as_str().into(),
            };
            locations.insert(csv_ledger.location.clone());
            events.push(ledger);
        }

        if !csv_ledger.project.is_empty() && csv_ledger.added.unwrap_or(0) > 0 {
            let ledger = LedgerEntry {
                t: parse_datetime::parse_datetime(&csv_ledger.t)?,
                count: csv_ledger.added.unwrap(),
                ev: LedgerEvent::UnsolderFrom(
                    store
                        .name_to_id(csv_ledger.project.as_str())
                        .as_str()
                        .into(),
                ),
                part: store.name_to_id(&csv_ledger.part).as_str().into(),
            };
            events.push(ledger);
            projects.insert(csv_ledger.project.clone());
        }

        if !csv_ledger.project.is_empty() && csv_ledger.removed.unwrap_or(0) > 0 {
            let ledger = LedgerEntry {
                t: parse_datetime::parse_datetime(&csv_ledger.t)?,
                count: csv_ledger.removed.unwrap(),
                ev: LedgerEvent::SolderTo(
                    store
                        .name_to_id(csv_ledger.project.as_str())
                        .as_str()
                        .into(),
                ),
                part: store.name_to_id(&csv_ledger.part).as_str().into(),
            };
            events.push(ledger);
            projects.insert(csv_ledger.project.clone());
        }

        if !csv_ledger.source.is_empty() && csv_ledger.added.unwrap_or(0) > 0 {
            let ledger = LedgerEntry {
                t: parse_datetime::parse_datetime(&csv_ledger.t)?,
                count: csv_ledger.added.unwrap(),
                ev: LedgerEvent::DeliverFrom(csv_ledger.source.as_str().into()),
                part: store.name_to_id(&csv_ledger.part).as_str().into(),
            };
            sources.insert(csv_ledger.source.clone());
            events.push(ledger);
        }
    }

    let mut recorded_parts = HashSet::new();

    for csv_part in part_reader.deserialize() {
        print!("P");
        let csv_part: CsvPartDto = csv_part?;

        if csv_part.name.is_empty() {
            continue;
        }

        recorded_parts.insert(csv_part.name.clone());

        let mut part = Part {
            id: store.name_to_id(&csv_part.name).as_str().into(),
            filename: None,
            metadata: PartMetadata {
                id: Some(store.name_to_id(&csv_part.name)),
                name: csv_part.name.trim().to_string(),
                manufacturer_id: csv_part.name.trim().to_string(),
                manufacturer: csv_part.manufacturer.trim().to_string(),
                labels: MultiMap::new(),
                attributes: MultiMap::new(),
                summary: csv_part.summary.trim().to_string(),
                types: HashSet::new(),
            },
            content: csv_part.description,
        };

        part.metadata.types.insert(diilo::store::ObjectType::Part);

        if locations.contains(&csv_part.name) {
            part.metadata
                .types
                .insert(diilo::store::ObjectType::Project);
        }

        if projects.contains(&csv_part.name) {
            part.metadata
                .types
                .insert(diilo::store::ObjectType::Project);
        }

        if !csv_part.footprint.trim().is_empty() {
            part.metadata.labels.insert(
                "footprint".to_owned(),
                csv_part.footprint.trim().to_lowercase(),
            );
        }

        if !csv_part.category.trim().is_empty() {
            part.metadata.labels.insert(
                "category".to_owned(),
                csv_part.category.trim().to_lowercase(),
            );
        }

        store.store_part(&mut part)?;
    }

    println!(".");

    events.sort_by_key(|e| e.t);
    for e in events {
        store.record_event(&e)?;
    }

    for l in &projects {
        if l.trim().is_empty() {
            continue;
        }

        if recorded_parts.contains(l) {
            continue;
        }

        let mut p = Part {
            id: store.name_to_id(l).as_str().into(),
            filename: None,
            metadata: PartMetadata {
                id: Some(store.name_to_id(l)),
                name: l.clone(),
                types: HashSet::new(),
                ..Default::default()
            },
            ..Default::default()
        };
        p.metadata.types.insert(diilo::store::ObjectType::Project);
        p.metadata.types.insert(diilo::store::ObjectType::Part);
        store.store_part(&mut p)?;
    }

    for l in locations {
        if l.trim().is_empty() {
            continue;
        }

        if recorded_parts.contains(&l) {
            continue;
        }

        if projects.contains(&l) {
            continue;
        }

        let mut p = Part {
            id: store.name_to_id(&l).as_str().into(),
            filename: None,
            metadata: PartMetadata {
                id: Some(store.name_to_id(&l)),
                name: l,
                types: HashSet::new(),
                ..Default::default()
            },
            ..Default::default()
        };
        p.metadata.types.insert(diilo::store::ObjectType::Location);
        store.store_part(&mut p)?;
    }

    for l in sources {
        if l.trim().is_empty() {
            continue;
        }

        if recorded_parts.contains(&l) {
            continue;
        }

        if projects.contains(&l) {
            continue;
        }

        let mut p = Part {
            id: store.name_to_id(&l).as_str().into(),
            filename: None,
            metadata: PartMetadata {
                id: Some(store.name_to_id(&l)),
                name: l,
                types: HashSet::new(),
                ..Default::default()
            },
            ..Default::default()
        };
        p.metadata.types.insert(diilo::store::ObjectType::Source);
        store.store_part(&mut p)?;
    }

    Ok(())
}
