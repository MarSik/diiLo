// This file contains tests of some internal logic and emulates loading and processing
// events as stored on disk without instantiating the UI.
// The app uses the examples from here internally.

use std::path::PathBuf;

use chrono::Local;
use diilo::store::{LedgerEntry, LedgerEvent, PartId, Store, cache::CountCacheSum};
use tempfile::TempDir;
use testutils::{populate_store, sort_count_predictably};

mod testutils;

#[test]
fn test_basic_delivery() -> anyhow::Result<()> {
    let store_path = TempDir::new()?;
    let mut store = Store::new(store_path.path().to_path_buf())?;
    populate_store(&mut store)?;

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 1,
        part: PartId::Simple("test-part".into()),
        ev: LedgerEvent::StoreTo(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let count = store.count_by_part(&ev.part);
    let sum = count.sum();

    assert_eq!(sum.added, 1, "should have added one item to cache");
    assert_eq!(sum.removed, 0, "should have empty remove count");
    assert_eq!(sum.required, 0, "should have empty required count");

    assert_eq!(count.len(), 1, "part should be in one location only");
    assert_eq!(
        count.first().unwrap().part(),
        &ev.part,
        "should be test part"
    );
    assert_eq!(
        count.first().unwrap().location().part_type().to_string(),
        "location-a".to_string(),
        "should be stored in location-a"
    );

    Ok(())
}

#[test]
fn test_basic_delivery_dual() -> anyhow::Result<()> {
    let store_path = TempDir::new()?;
    let mut store = Store::new(store_path.path().to_path_buf())?;
    populate_store(&mut store)?;

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 1,
        part: PartId::Simple("test-part".into()),
        ev: LedgerEvent::StoreTo(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 2,
        part: PartId::Simple("test-part".into()),
        ev: LedgerEvent::StoreTo(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let count = store.count_by_part(&ev.part);
    let sum = count.sum();

    assert_eq!(sum.added, 3, "should have added one item to cache");
    assert_eq!(sum.removed, 0, "should have empty remove count");
    assert_eq!(sum.required, 0, "should have empty required count");

    assert_eq!(count.len(), 1, "part should be in one location only");
    assert_eq!(
        count.first().unwrap().part(),
        &ev.part,
        "should be test part"
    );
    assert_eq!(
        count.first().unwrap().location().part_type().to_string(),
        "location-a".to_string(),
        "should be stored in location-a"
    );

    Ok(())
}

#[test]
fn test_explicit_pieces_delivery() -> anyhow::Result<()> {
    let store_path = TempDir::new()?;
    let mut store = Store::new(store_path.path().to_path_buf())?;
    populate_store(&mut store)?;

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 1,
        part: PartId::Piece("test-pieces".into(), 1),
        ev: LedgerEvent::StoreTo(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let count = store.count_by_part(&ev.part);
    let sum = count.sum();

    assert_eq!(sum.added, 1, "should have added one item to cache");
    assert_eq!(sum.removed, 0, "should have empty remove count");
    assert_eq!(sum.required, 0, "should have empty required count");

    assert_eq!(count.len(), 1, "part should be in one location only");
    assert_eq!(
        count.first().unwrap().part(),
        &ev.part,
        "should be test pieces"
    );
    assert_eq!(
        count.first().unwrap().part().piece_size_option(),
        Some(1),
        "should contain piece size data"
    );
    assert_eq!(
        count.first().unwrap().location().part_type().to_string(),
        "location-a".to_string(),
        "should be stored in location-a"
    );

    Ok(())
}

#[test]
fn test_explicit_pieces_forcecount() -> anyhow::Result<()> {
    let store_path = TempDir::new()?;
    let mut store = Store::new(store_path.path().to_path_buf())?;
    populate_store(&mut store)?;

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 10,
        part: PartId::Piece("test-pieces".into(), 10),
        ev: LedgerEvent::ForceCount(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let count = store.count_by_part_type(ev.part.part_type());
    let sum = count.sum();

    assert_eq!(sum.added, 10, "should have added ten items to cache");
    assert_eq!(sum.removed, 0, "should have empty remove count");
    assert_eq!(sum.required, 0, "should have empty required count");

    assert_eq!(count.len(), 1, "part should be in one location only");
    assert_eq!(
        count.first().unwrap().part(),
        &ev.part,
        "should be test pieces"
    );
    assert_eq!(
        count.first().unwrap().part().piece_size_option(),
        Some(10),
        "should contain piece size data"
    );
    assert_eq!(
        count.first().unwrap().location().part_type().to_string(),
        "location-a".to_string(),
        "should be stored in location-a"
    );

    Ok(())
}

#[test]
fn test_explicit_pieces_forcecount_w_conversion() -> anyhow::Result<()> {
    let store_path = TempDir::new()?;
    let mut store = Store::new(store_path.path().to_path_buf())?;
    populate_store(&mut store)?;

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 10,
        part: PartId::Simple("test-pieces".into()),
        ev: LedgerEvent::ForceCount(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let count = store.count_by_part_type(ev.part.part_type());
    let sum = count.sum();

    assert_eq!(sum.added, 10, "should have added ten items to cache");
    assert_eq!(sum.removed, 0, "should have empty remove count");
    assert_eq!(sum.required, 0, "should have empty required count");

    assert_eq!(count.len(), 1, "part should be in one location only");
    assert_eq!(
        count.first().unwrap().part().part_type(),
        ev.part.part_type(),
        "should be test pieces"
    );
    assert_eq!(
        count.first().unwrap().part().piece_size_option(),
        Some(10),
        "should contain piece size data"
    );
    assert_eq!(
        count.first().unwrap().location().part_type().to_string(),
        "location-a".to_string(),
        "should be stored in location-a"
    );

    Ok(())
}

#[test]
fn test_differently_sized_pieces_delivery() -> anyhow::Result<()> {
    let store_path = TempDir::new()?;
    let mut store = Store::new(store_path.path().to_path_buf())?;
    populate_store(&mut store)?;

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 1,
        part: PartId::Piece("test-pieces".into(), 1),
        ev: LedgerEvent::StoreTo(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 2,
        part: PartId::Piece("test-pieces".into(), 2),
        ev: LedgerEvent::StoreTo(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let mut count = store.count_by_part_type(ev.part.part_type());
    let sum = count.sum();

    // Test predictability needs the count vector sorted by type id and piece size
    sort_count_predictably(&mut count);

    assert_eq!(sum.added, 3, "should have added one item to cache");
    assert_eq!(sum.removed, 0, "should have empty remove count");
    assert_eq!(sum.required, 0, "should have empty required count");

    assert_eq!(count.len(), 2, "part should be two elements");

    assert_eq!(
        count.first().unwrap().part().part_type(),
        ev.part.part_type(),
        "should be test pieces"
    );
    assert_eq!(
        count.first().unwrap().part().piece_size_option(),
        Some(1),
        "should contain piece size data = 1"
    );
    assert_eq!(
        count.first().unwrap().location().part_type().to_string(),
        "location-a".to_string(),
        "should be stored in location-a"
    );

    assert_eq!(
        count.get(1).unwrap().part().part_type(),
        ev.part.part_type(),
        "should be test pieces"
    );
    assert_eq!(
        count.get(1).unwrap().part().piece_size_option(),
        Some(2),
        "should contain piece size data = 2"
    );
    assert_eq!(
        count.get(1).unwrap().location().part_type().to_string(),
        "location-a".to_string(),
        "should be stored in location-a"
    );

    Ok(())
}

#[test]
fn test_partial_piece_move() -> anyhow::Result<()> {
    let store_path = TempDir::new()?;
    let mut store = Store::new(store_path.path().to_path_buf())?;
    populate_store(&mut store)?;

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 10,
        part: PartId::Piece("test-pieces".into(), 10),
        ev: LedgerEvent::StoreTo(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 3,
        part: PartId::Piece("test-pieces".into(), 10),
        ev: LedgerEvent::TakeFrom(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 3,
        part: PartId::Piece("test-pieces".into(), 10),
        ev: LedgerEvent::StoreTo(PartId::Simple("location-b".into())),
    };

    store.update_count_cache(&ev);

    let mut count = store.count_by_part_type(ev.part.part_type());
    let sum = count.sum();

    // Test predictability needs the count vector sorted by type id and piece size
    sort_count_predictably(&mut count);

    assert_eq!(
        sum.added, 10,
        "should have 10 in total - delivery not shown, because the cache item is empty"
    );
    assert_eq!(
        sum.removed, 0,
        "no removals - the original cache item is empty and supressed"
    );
    assert_eq!(sum.required, 0, "should have empty required count");

    assert_eq!(count.len(), 2, "part should be two elements");

    assert_eq!(
        count.first().unwrap().part().part_type(),
        ev.part.part_type(),
        "should be test pieces"
    );
    assert_eq!(
        count.first().unwrap().part().piece_size_option(),
        Some(3),
        "should contain piece size data = 3"
    );
    assert_eq!(
        count.first().unwrap().location().part_type().to_string(),
        "location-b".to_string(),
        "should be stored in location-b"
    );

    assert_eq!(
        count.get(1).unwrap().part().part_type(),
        ev.part.part_type(),
        "should be test pieces"
    );
    assert_eq!(
        count.get(1).unwrap().part().piece_size_option(),
        Some(7),
        "should contain piece size data = 7"
    );
    assert_eq!(
        count.get(1).unwrap().location().part_type().to_string(),
        "location-a".to_string(),
        "should be stored in location-a"
    );

    Ok(())
}

#[test]
fn test_partial_piece_move_double() -> anyhow::Result<()> {
    let store_path = TempDir::new()?;
    let mut store = Store::new(store_path.path().to_path_buf())?;
    populate_store(&mut store)?;

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 10,
        part: PartId::Piece("test-pieces".into(), 10),
        ev: LedgerEvent::StoreTo(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 3,
        part: PartId::Piece("test-pieces".into(), 10),
        ev: LedgerEvent::TakeFrom(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 3,
        part: PartId::Piece("test-pieces".into(), 10),
        ev: LedgerEvent::StoreTo(PartId::Simple("location-b".into())),
    };

    store.update_count_cache(&ev);

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 3,
        part: PartId::Piece("test-pieces".into(), 7),
        ev: LedgerEvent::TakeFrom(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 3,
        part: PartId::Piece("test-pieces".into(), 7),
        ev: LedgerEvent::StoreTo(PartId::Simple("location-b".into())),
    };

    store.update_count_cache(&ev);

    let mut count = store.count_by_part_type(ev.part.part_type());
    let sum = count.sum();

    // Test predictability needs the count vector sorted by type id and piece size
    sort_count_predictably(&mut count);

    assert_eq!(
        sum.added, 10,
        "should still have 10 in total - delivery not shown, because the cache item is empty"
    );
    assert_eq!(
        sum.removed, 0,
        "no removals - the original cache item is empty and supressed"
    );
    assert_eq!(sum.required, 0, "should have empty required count");

    assert_eq!(count.len(), 2, "part should be two elements");

    assert_eq!(count.first().unwrap().count(), 6, "should be 6 test pieces");
    assert_eq!(
        count.first().unwrap().part().part_type(),
        ev.part.part_type(),
        "should be test pieces"
    );
    assert_eq!(
        count.first().unwrap().part().piece_size_option(),
        Some(3),
        "should contain piece size data = 3"
    );
    assert_eq!(
        count.first().unwrap().location().part_type().to_string(),
        "location-b".to_string(),
        "should be stored in location-b"
    );

    assert_eq!(count.get(1).unwrap().count(), 4, "should be 4 test pieces");
    assert_eq!(
        count.get(1).unwrap().part().part_type(),
        ev.part.part_type(),
        "should be test pieces"
    );
    assert_eq!(
        count.get(1).unwrap().part().piece_size_option(),
        Some(4),
        "should contain piece size data = 4"
    );
    assert_eq!(
        count.get(1).unwrap().location().part_type().to_string(),
        "location-a".to_string(),
        "should be stored in location-a"
    );

    Ok(())
}

#[test]
fn test_partial_piece_solder() -> anyhow::Result<()> {
    let store_path = TempDir::new()?;
    let mut store = Store::new(store_path.path().to_path_buf())?;
    populate_store(&mut store)?;

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 10,
        part: PartId::Piece("test-pieces".into(), 10),
        ev: LedgerEvent::StoreTo(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 3,
        part: PartId::Piece("test-pieces".into(), 10),
        ev: LedgerEvent::TakeFrom(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 3,
        part: PartId::Piece("test-pieces".into(), 10),
        ev: LedgerEvent::SolderTo(PartId::Simple("project-x".into())),
    };

    store.update_count_cache(&ev);

    let mut count = store.count_by_part_type(ev.part.part_type());
    let sum = count.sum();

    // Test predictability needs the count vector sorted by type id and piece size
    sort_count_predictably(&mut count);

    assert_eq!(
        sum.added, 7,
        "should have 7 in total - delivery not shown, because the cache item is empty"
    );
    assert_eq!(
        sum.removed, 0,
        "no removals - the original cache item is empty and supressed"
    );
    assert_eq!(sum.required, 0, "should have empty required count");

    assert_eq!(count.len(), 1, "part should be one elements");

    assert_eq!(
        count.first().unwrap().part().part_type(),
        ev.part.part_type(),
        "should be test pieces"
    );
    assert_eq!(
        count.first().unwrap().part().piece_size_option(),
        Some(7),
        "should contain piece size data = 7"
    );
    assert_eq!(
        count.first().unwrap().location().part_type().to_string(),
        "location-a".to_string(),
        "should be stored in location-a"
    );

    let count = store.count_by_project(&PartId::Simple("project-x".into()));
    let sum = count.sum();

    assert_eq!(sum.added, 3, "should have 3 in total");
    assert_eq!(sum.removed, 0, "no removals");
    assert_eq!(sum.required, 0, "should have empty required count");

    assert_eq!(count.len(), 1, "project should be one element");

    assert_eq!(
        count.first().unwrap().part().part_type(),
        ev.part.part_type(),
        "should be test pieces"
    );
    assert_eq!(
        count.first().unwrap().part().piece_size_option(),
        Some(3),
        "should contain piece size data = 3"
    );
    assert_eq!(
        count.first().unwrap().location().part_type().to_string(),
        "project-x".to_string(),
        "should be stored in project-x"
    );

    Ok(())
}

#[test]
fn test_partial_piece_solder_unsolder() -> anyhow::Result<()> {
    let store_path = TempDir::new()?;
    let mut store = Store::new(store_path.path().to_path_buf())?;
    populate_store(&mut store)?;

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 10,
        part: PartId::Piece("test-pieces".into(), 10),
        ev: LedgerEvent::StoreTo(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 3,
        part: PartId::Piece("test-pieces".into(), 10),
        ev: LedgerEvent::TakeFrom(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 3,
        part: PartId::Piece("test-pieces".into(), 10),
        ev: LedgerEvent::SolderTo(PartId::Simple("project-x".into())),
    };

    store.update_count_cache(&ev);

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 2,
        part: PartId::Piece("test-pieces".into(), 3),
        ev: LedgerEvent::UnsolderFrom(PartId::Simple("project-x".into())),
    };

    store.update_count_cache(&ev);

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 2,
        part: PartId::Piece("test-pieces".into(), 3),
        ev: LedgerEvent::StoreTo(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let mut count = store.count_by_part_type(ev.part.part_type());
    let sum = count.sum();

    // Test predictability needs the count vector sorted by type id and piece size
    sort_count_predictably(&mut count);

    assert_eq!(
        sum.added, 9,
        "should have 7 in total - delivery not shown, because the cache item is empty"
    );
    assert_eq!(
        sum.removed, 0,
        "no removals - the original cache item is empty and supressed"
    );
    assert_eq!(sum.required, 0, "should have empty required count");

    assert_eq!(count.len(), 2, "part should be two elements");

    assert_eq!(
        count.first().unwrap().part().part_type(),
        ev.part.part_type(),
        "should be test pieces"
    );
    assert_eq!(
        count.first().unwrap().part().piece_size_option(),
        Some(2),
        "should contain piece size data = 2"
    );
    assert_eq!(
        count.first().unwrap().location().part_type().to_string(),
        "location-a".to_string(),
        "should be stored in location-a"
    );

    assert_eq!(
        count.get(1).unwrap().part().part_type(),
        ev.part.part_type(),
        "should be test pieces"
    );
    assert_eq!(
        count.get(1).unwrap().part().piece_size_option(),
        Some(7),
        "should contain piece size data = 7"
    );
    assert_eq!(
        count.get(1).unwrap().location().part_type().to_string(),
        "location-a".to_string(),
        "should be stored in location-a"
    );

    let count = store.count_by_project(&PartId::Simple("project-x".into()));
    let sum = count.sum();

    assert_eq!(sum.added, 1, "should have 1 in total");
    assert_eq!(sum.removed, 0, "no removals");
    assert_eq!(sum.required, 0, "should have empty required count");

    assert_eq!(count.len(), 1, "project should be one element");

    assert_eq!(
        count.first().unwrap().part().part_type(),
        ev.part.part_type(),
        "should be test pieces"
    );
    assert_eq!(
        count.first().unwrap().part().piece_size_option(),
        Some(1),
        "should contain piece size data = 1"
    );
    assert_eq!(
        count.first().unwrap().location().part_type().to_string(),
        "project-x".to_string(),
        "should be stored in project-x"
    );

    Ok(())
}

#[test]
fn test_partial_part_solder_unsolder() -> anyhow::Result<()> {
    let store_path = TempDir::new()?;
    let mut store = Store::new(store_path.path().to_path_buf())?;
    populate_store(&mut store)?;

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 10,
        part: PartId::Simple("test-part".into()),
        ev: LedgerEvent::StoreTo(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 3,
        part: PartId::Simple("test-part".into()),
        ev: LedgerEvent::TakeFrom(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 3,
        part: PartId::Simple("test-part".into()),
        ev: LedgerEvent::SolderTo(PartId::Simple("project-x".into())),
    };

    store.update_count_cache(&ev);

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 2,
        part: PartId::Simple("test-part".into()),
        ev: LedgerEvent::UnsolderFrom(PartId::Simple("project-x".into())),
    };

    store.update_count_cache(&ev);

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 2,
        part: PartId::Simple("test-part".into()),
        ev: LedgerEvent::StoreTo(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let mut count = store.count_by_part_type(ev.part.part_type());
    let sum = count.sum();

    // Test predictability needs the count vector sorted by type id and piece size
    sort_count_predictably(&mut count);

    assert_eq!(
        sum.added, 12,
        "should have 12 in total - delivery + unsolder"
    );
    assert_eq!(sum.removed, 3, "3 were soldered");
    assert_eq!(sum.required, 0, "should have empty required count");

    assert_eq!(count.len(), 1, "part should be one element");

    assert_eq!(
        count.first().unwrap().part().part_type(),
        ev.part.part_type(),
        "should be test part"
    );
    assert_eq!(
        count.first().unwrap().part().piece_size_option(),
        None,
        "should not contain piece size data"
    );
    assert_eq!(
        count.first().unwrap().location().part_type().to_string(),
        "location-a".to_string(),
        "should be stored in location-a"
    );

    let count = store.count_by_project(&PartId::Simple("project-x".into()));
    let sum = count.sum();

    assert_eq!(sum.added, 3, "should have 3 soldered in total");
    assert_eq!(sum.removed, 2, "2 unsoldered");
    assert_eq!(sum.required, 0, "should have empty required count");

    assert_eq!(count.len(), 1, "project should be one element");

    assert_eq!(
        count.first().unwrap().part().part_type(),
        ev.part.part_type(),
        "should be test part"
    );
    assert_eq!(
        count.first().unwrap().part().piece_size_option(),
        None,
        "should contain no piece size data"
    );
    assert_eq!(
        count.first().unwrap().location().part_type().to_string(),
        "project-x".to_string(),
        "should be stored in project-x"
    );

    Ok(())
}

#[test]
fn test_partial_piece_split_move() -> anyhow::Result<()> {
    let store_path = TempDir::new()?;
    let mut store = Store::new(store_path.path().to_path_buf())?;
    populate_store(&mut store)?;

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 10,
        part: PartId::Piece("test-pieces".into(), 10),
        ev: LedgerEvent::StoreTo(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 3,
        part: PartId::Piece("test-pieces".into(), 10),
        ev: LedgerEvent::TakeFrom(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 3,
        part: PartId::Piece("test-pieces".into(), 10),
        ev: LedgerEvent::StoreTo(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let mut count = store.count_by_part_type(ev.part.part_type());
    let sum = count.sum();

    // Test predictability needs the count vector sorted by type id and piece size
    sort_count_predictably(&mut count);

    assert_eq!(
        sum.added, 10,
        "should have 10 in total - delivery not shown, because the cache item is empty"
    );
    assert_eq!(
        sum.removed, 0,
        "no removals - the original cache item is empty and supressed"
    );
    assert_eq!(sum.required, 0, "should have empty required count");

    assert_eq!(count.len(), 2, "part should be two elements");

    assert_eq!(
        count.first().unwrap().part().part_type(),
        ev.part.part_type(),
        "should be test pieces"
    );
    assert_eq!(
        count.first().unwrap().part().piece_size_option(),
        Some(3),
        "should contain piece size data = 3"
    );
    assert_eq!(
        count.first().unwrap().location().part_type().to_string(),
        "location-a".to_string(),
        "should be stored in location-a"
    );

    assert_eq!(
        count.get(1).unwrap().part().part_type(),
        ev.part.part_type(),
        "should be test pieces"
    );
    assert_eq!(
        count.get(1).unwrap().part().piece_size_option(),
        Some(7),
        "should contain piece size data = 7"
    );
    assert_eq!(
        count.get(1).unwrap().location().part_type().to_string(),
        "location-a".to_string(),
        "should be stored in location-a"
    );

    Ok(())
}

#[test]
fn test_piece_delivery_w_id_conversion() -> anyhow::Result<()> {
    let store_path = TempDir::new()?;
    let mut store = Store::new(store_path.path().to_path_buf())?;
    populate_store(&mut store)?;

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 10,
        part: PartId::Simple("test-pieces".into()),
        ev: LedgerEvent::StoreTo(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let count = store.count_by_part_type(ev.part.part_type());
    let sum = count.sum();

    assert_eq!(sum.added, 10, "should have 10 in total");
    assert_eq!(sum.removed, 0, "no removals");
    assert_eq!(sum.required, 0, "should have empty required count");

    assert_eq!(count.len(), 1, "part should be in one cache element");

    assert_eq!(
        count.first().unwrap().part().part_type(),
        ev.part.part_type(),
        "should be test pieces"
    );
    assert_eq!(
        count.first().unwrap().part().piece_size_option(),
        Some(10),
        "should contain piece size data = 10"
    );
    assert_eq!(
        count.first().unwrap().location().part_type().to_string(),
        "location-a".to_string(),
        "should be stored in location-a"
    );

    Ok(())
}

#[test]
fn test_piece_move_w_id_conversion() -> anyhow::Result<()> {
    let store_path = TempDir::new()?;
    let mut store = Store::new(store_path.path().to_path_buf())?;
    populate_store(&mut store)?;

    // Simulate a change of part tracking from simple to pieces
    // First override test-pieces to use count tracking
    let part = Store::load_part_from_file(
        [
            env!("CARGO_MANIFEST_DIR"),
            "tests",
            "resources",
            "objects",
            "test-pieces-force-simple.md",
        ]
        .iter()
        .collect::<PathBuf>(),
    )?;
    store.insert_part_to_cache(part);

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 10,
        part: PartId::Simple("test-pieces".into()),
        ev: LedgerEvent::StoreTo(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    // Reload test-pieces with pieces tracking enabled
    populate_store(&mut store)?;

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 3,
        part: PartId::Simple("test-pieces".into()),
        ev: LedgerEvent::TakeFrom(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 3,
        part: PartId::Simple("test-pieces".into()),
        ev: LedgerEvent::StoreTo(PartId::Simple("location-b".into())),
    };

    store.update_count_cache(&ev);

    let mut count = store.count_by_part_type(ev.part.part_type());
    let sum = count.sum();

    // Test predictability needs the count vector sorted by type id and piece size
    sort_count_predictably(&mut count);

    assert_eq!(
        sum.added, 13,
        "should have 10 in total due to 10 delivery and 3 split"
    );
    assert_eq!(sum.removed, 3, "3 were removed from location-a");
    assert_eq!(sum.required, 0, "should have empty required count");

    assert_eq!(count.len(), 2, "part should be in two locations");

    assert_eq!(
        count.first().unwrap().part().part_type(),
        ev.part.part_type(),
        "should be test pieces"
    );
    assert_eq!(
        count.first().unwrap().part().piece_size_option(),
        None,
        "should not contain piece size data as it comes from count tracking times"
    );
    assert_eq!(
        count.first().unwrap().location().part_type().to_string(),
        "location-a".to_string(),
        "should be stored in location-a"
    );
    assert_eq!(count.first().unwrap().count(), 7, "should have 7 remaining");

    assert_eq!(
        count.get(1).unwrap().part().part_type(),
        ev.part.part_type(),
        "should be test pieces"
    );
    assert_eq!(
        count.get(1).unwrap().part().piece_size_option(),
        Some(3),
        "should contain piece size data = 3"
    );
    assert_eq!(
        count.get(1).unwrap().location().part_type().to_string(),
        "location-b".to_string(),
        "should be stored in location-b"
    );
    assert_eq!(count.get(1).unwrap().count(), 3, "should have 3 items");

    Ok(())
}

#[test]
fn test_piece_move_w_id_conversion_to_simple() -> anyhow::Result<()> {
    let store_path = TempDir::new()?;
    let mut store = Store::new(store_path.path().to_path_buf())?;
    populate_store(&mut store)?;

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 10,
        part: PartId::Simple("test-pieces".into()),
        ev: LedgerEvent::StoreTo(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let count = store.count_by_part_type(ev.part.part_type());
    let sum = count.sum();

    assert_eq!(
        sum.added, 10,
        "should have 10 in total due to 7 as pieces and 3 in second location"
    );
    assert_eq!(
        sum.removed, 0,
        "no removals, because a split piece was created"
    );
    assert_eq!(sum.required, 0, "should have empty required count");

    assert_eq!(count.len(), 1, "part should be in one location");

    assert_eq!(
        count.first().unwrap().part().part_type(),
        ev.part.part_type(),
        "should be test pieces"
    );
    assert_eq!(
        count.first().unwrap().part().piece_size_option(),
        Some(10),
        "should contain piece size 10 in the source"
    );
    assert_eq!(
        count.first().unwrap().location().part_type().to_string(),
        "location-a".to_string(),
        "should be stored in location-a"
    );
    assert_eq!(
        count.first().unwrap().count(),
        10,
        "should have 10 remaining"
    );

    // Simulate a change of part tracking from pieces to simple
    let part = Store::load_part_from_file(
        [
            env!("CARGO_MANIFEST_DIR"),
            "tests",
            "resources",
            "objects",
            "test-pieces-force-simple.md",
        ]
        .iter()
        .collect::<PathBuf>(),
    )?;
    store.insert_part_to_cache(part);

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 3,
        part: PartId::Piece("test-pieces".into(), 10),
        ev: LedgerEvent::TakeFrom(PartId::Simple("location-a".into())),
    };

    store.update_count_cache(&ev);

    let ev = LedgerEntry {
        t: Local::now().fixed_offset(),
        count: 3,
        part: PartId::Piece("test-pieces".into(), 10),
        ev: LedgerEvent::StoreTo(PartId::Simple("location-b".into())),
    };

    store.update_count_cache(&ev);

    let mut count = store.count_by_part_type(ev.part.part_type());
    let sum = count.sum();

    // Test predictability needs the count vector sorted by type id and piece size
    sort_count_predictably(&mut count);

    assert_eq!(
        sum.added, 10,
        "should have 10 in total due to 7 as pieces and 3 in second location"
    );
    assert_eq!(
        sum.removed, 0,
        "no removals, because a split piece was created"
    );
    assert_eq!(sum.required, 0, "should have empty required count");

    assert_eq!(count.len(), 2, "part should be in two locations");

    assert_eq!(
        count.get(1).unwrap().part().part_type(),
        ev.part.part_type(),
        "should be test pieces"
    );
    assert_eq!(
        count.get(1).unwrap().part().piece_size_option(),
        None,
        "should not contain piece size data"
    );
    assert_eq!(
        count.get(1).unwrap().location().part_type().to_string(),
        "location-b".to_string(),
        "should be stored in location-b"
    );
    assert_eq!(count.get(1).unwrap().count(), 3, "should have 3 items");

    assert_eq!(
        count.first().unwrap().part().part_type(),
        ev.part.part_type(),
        "should be test pieces"
    );
    assert_eq!(
        count.first().unwrap().part().piece_size_option(),
        None,
        "should not contain piece sizes in the source"
    );
    assert_eq!(
        count.first().unwrap().location().part_type().to_string(),
        "location-a".to_string(),
        "should be stored in location-a"
    );
    assert_eq!(count.first().unwrap().count(), 7, "should have 7 remaining");

    Ok(())
}
