use std::{cmp::Ordering, path::PathBuf};

use diilo::store::{cache::CountCacheEntry, Store};

pub fn populate_store(store: &mut Store) -> anyhow::Result<()> {
    let part = Store::load_part_from_file(
        [
            env!("CARGO_MANIFEST_DIR"),
            "tests",
            "resources",
            "objects",
            "test-part.md",
        ]
        .iter()
        .collect::<PathBuf>(),
    )?;
    store.insert_part_to_cache(part);

    let part = Store::load_part_from_file(
        [
            env!("CARGO_MANIFEST_DIR"),
            "tests",
            "resources",
            "objects",
            "test-pieces.md",
        ]
        .iter()
        .collect::<PathBuf>(),
    )?;
    store.insert_part_to_cache(part);

    let location = Store::load_part_from_file(
        [
            env!("CARGO_MANIFEST_DIR"),
            "tests",
            "resources",
            "objects",
            "location-a.md",
        ]
        .iter()
        .collect::<PathBuf>(),
    )?;
    store.insert_part_to_cache(location);

    let location = Store::load_part_from_file(
        [
            env!("CARGO_MANIFEST_DIR"),
            "tests",
            "resources",
            "objects",
            "location-b.md",
        ]
        .iter()
        .collect::<PathBuf>(),
    )?;
    store.insert_part_to_cache(location);
    Ok(())
}

pub fn sort_count_predictably(count: &mut [CountCacheEntry]) {
    // Test predictability needs the count vector sorted by type id and piece size
    count.sort_by(|a, b| {
        // First sort by part ID
        let ord = a.part().part_type().cmp(b.part().part_type());
        match ord {
            std::cmp::Ordering::Less => return ord,
            std::cmp::Ordering::Greater => return ord,
            std::cmp::Ordering::Equal => (),
        }

        // Then sort by piece size
        let piece_ord = a
            .part()
            .piece_size_option()
            .cmp(&b.part().piece_size_option());
        if piece_ord != Ordering::Equal {
            return piece_ord;
        }

        // Then sort by Location ID
        let ord = a.location().part_type().cmp(b.location().part_type());
        match ord {
            std::cmp::Ordering::Less => return ord,
            std::cmp::Ordering::Greater => return ord,
            std::cmp::Ordering::Equal => (),
        }

        a.part().piece_size().cmp(&b.part().piece_size())
    });
}
