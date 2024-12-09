use super::*;
use tempdir::TempDir;

#[test]
fn test_make_part_id() -> anyhow::Result<()> {
    let store_path = TempDir::new("test")?;

    let store = Store::new(store_path.into_path())?;
    let mut app = App::new(store)?;

    let part_id = app.make_new_type_id("test");
    assert_eq!(part_id, "test".into());

    let part = Part {
        id: part_id,
        ..Default::default()
    };
    app.store.insert_part_to_cache(part);

    let part_id = app.make_new_type_id("test");
    assert_eq!(part_id, "test--1".into());

    let part = Part {
        id: part_id,
        ..Default::default()
    };
    app.store.insert_part_to_cache(part);

    let part_id = app.make_new_type_id("test");
    assert_eq!(part_id, "test--2".into());

    Ok(())
}
