use std::io::BufReader;

use diilo::store::{cache::CountCacheSum, Store};
use tempdir::TempDir;
use testutils::{populate_store, sort_count_predictably};

mod testutils;

#[test]
fn test_explicit_pieces_forcecount() -> anyhow::Result<()> {
    let store_path = TempDir::new("test")?;
    let mut store = Store::new(store_path.into_path())?;
    populate_store(&mut store)?;

    let f = BufReader::new(
        "2024-12-10T10:00:00Z,count=10,size=10,part=test-pieces,correct,location=location-a\n"
            .as_bytes(),
    );
    store
        .load_events_from_buf(f)?
        .iter()
        .for_each(|ev| store.update_count_cache(ev));

    let mut count = store.count_by_part_type(&"test-pieces".into());
    let sum = count.sum();
    sort_count_predictably(&mut count);

    assert_eq!(sum.added, 10, "should have added ten items to cache");
    assert_eq!(sum.removed, 0, "should have empty remove count");
    assert_eq!(sum.required, 0, "should have empty required count");

    assert_eq!(count.len(), 1, "part should be in one location only");
    assert_eq!(
        count.first().unwrap().part().part_type(),
        &"test-pieces".into(),
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
    let store_path = TempDir::new("test")?;
    let mut store = Store::new(store_path.into_path())?;
    populate_store(&mut store)?;

    let f = BufReader::new(
        "2024-12-10T10:00:00Z,count=10,part=test-pieces,correct,location=location-a\n".as_bytes(),
    );
    store
        .load_events_from_buf(f)?
        .iter()
        .for_each(|ev| store.update_count_cache(ev));

    let mut count = store.count_by_part_type(&"test-pieces".into());
    let sum = count.sum();
    sort_count_predictably(&mut count);

    assert_eq!(sum.added, 10, "should have added ten items to cache");
    assert_eq!(sum.removed, 0, "should have empty remove count");
    assert_eq!(sum.required, 0, "should have empty required count");

    assert_eq!(count.len(), 1, "part should be in one location only");
    assert_eq!(
        count.first().unwrap().part().part_type(),
        &"test-pieces".into(),
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
