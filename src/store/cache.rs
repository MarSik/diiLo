use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    ops::Deref,
    rc::Rc,
};

use super::{LocationId, PartId};

#[derive(Debug)]
pub enum CountChange {
    NONE,
    SET(usize),
    ADD(usize),
    REMOVE(usize),
}

#[derive(Debug, Clone)]
pub struct CountCacheEntry {
    // These must stay constant once constructed, because of the hashing function
    part_id: PartId,
    location_id: LocationId,

    // Only counts can be modified
    added: usize, // total Negative values CAN happen when counting moves of real parts that were not recorded
    removed: usize,
    required: usize, // How many parts will be needed

    show_empty: bool, // Should show in the cache even when empty
}

impl CountCacheEntry {
    pub fn new(
        part_id: PartId,
        location_id: LocationId,
        added: usize,
        removed: usize,
        required: usize,
    ) -> Self {
        Self {
            part_id,
            location_id,
            added,
            removed,
            required,
            show_empty: false,
        }
    }

    pub fn part(&self) -> &PartId {
        &self.part_id
    }

    pub fn location(&self) -> &LocationId {
        &self.location_id
    }

    pub fn count(&self) -> isize {
        (self.added as isize).saturating_sub_unsigned(self.removed)
    }

    pub fn required(&self) -> usize {
        self.required
    }

    pub fn added(&self) -> usize {
        self.added
    }

    pub fn removed(&self) -> usize {
        self.removed
    }

    pub fn show_empty(&self) -> bool {
        self.show_empty
    }
}

impl Eq for CountCacheEntry {}

impl PartialEq for CountCacheEntry {
    fn eq(&self, other: &Self) -> bool {
        self.part_id == other.part_id && self.location_id == other.location_id
    }
}

impl Hash for CountCacheEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.part_id.hash(state);
        self.location_id.hash(state);
    }
}

pub struct CountCache {
    by_location: HashMap<LocationId, HashSet<Rc<CountCacheEntry>>>,
    by_part: HashMap<PartId, HashSet<Rc<CountCacheEntry>>>,
    all: HashSet<Rc<CountCacheEntry>>,
}

impl Default for CountCache {
    fn default() -> Self {
        Self::new()
    }
}

impl CountCache {
    pub fn new() -> Self {
        Self {
            by_location: HashMap::new(),
            by_part: HashMap::new(),
            all: HashSet::new(),
        }
    }

    pub fn update_count(
        &mut self,
        part_id: &PartId,
        location_id: &LocationId,
        added: CountChange,
        removed: CountChange,
        required: CountChange,
    ) -> CountCacheEntry {
        let mut key = CountCacheEntry::new(Rc::clone(part_id), Rc::clone(location_id), 0, 0, 0);
        if let Some(v) = self.all.get(&key) {
            key.added = v.added;
            key.removed = v.removed;
            key.required = v.required;
            key.show_empty = v.show_empty;
        }

        match added {
            CountChange::NONE => (),
            CountChange::SET(v) => key.added = v,
            CountChange::ADD(d) => key.added = key.added.saturating_add(d),
            CountChange::REMOVE(d) => key.added = key.added.saturating_sub(d),
        };

        match removed {
            CountChange::NONE => (),
            CountChange::SET(v) => key.removed = v,
            CountChange::ADD(d) => key.removed = key.removed.saturating_add(d),
            CountChange::REMOVE(d) => key.removed = key.removed.saturating_sub(d),
        };

        match required {
            CountChange::NONE => (),
            CountChange::SET(v) => key.required = v,
            CountChange::ADD(d) => key.required = key.required.saturating_add(d),
            CountChange::REMOVE(d) => key.required = key.required.saturating_sub(d),
        };

        self.set_count(key)
    }

    pub fn get_count(&self, part_id: &PartId, location_id: &LocationId) -> CountCacheEntry {
        let mut key = CountCacheEntry::new(Rc::clone(part_id), Rc::clone(location_id), 0, 0, 0);
        if let Some(v) = self.all.get(&key) {
            key.added = v.added;
            key.removed = v.removed;
            key.required = v.required;
            key.show_empty = v.show_empty;
        }

        key
    }

    pub fn show_empty(
        &mut self,
        part_id: &PartId,
        location_id: &LocationId,
        show_empty: bool,
    ) -> CountCacheEntry {
        let mut key = CountCacheEntry::new(Rc::clone(part_id), Rc::clone(location_id), 0, 0, 0);
        if let Some(v) = self.all.get(&key) {
            key.added = v.added;
            key.removed = v.removed;
            key.required = v.required;
        }

        key.show_empty = show_empty;
        self.set_count(key)
    }

    pub fn set_count(&mut self, key: CountCacheEntry) -> CountCacheEntry {
        let key = Rc::new(key);

        if !self.by_location.contains_key(&key.location_id) {
            self.by_location
                .insert(Rc::clone(&key.location_id), HashSet::new());
        }
        self.by_location
            .get_mut(&key.location_id)
            .unwrap()
            .replace(Rc::clone(&key));

        if !self.by_part.contains_key(&key.part_id) {
            self.by_part.insert(Rc::clone(&key.part_id), HashSet::new());
        }
        self.by_part
            .get_mut(&key.part_id)
            .unwrap()
            .replace(Rc::clone(&key));

        self.all.replace(Rc::clone(&key));

        key.deref().clone()
    }

    pub fn by_location(&self, location_id: &str) -> Vec<CountCacheEntry> {
        let mut content = Vec::new();
        if let Some(c) = self.by_location.get(location_id) {
            for v in c {
                if v.count() == 0 && v.required == 0 && !v.show_empty {
                    continue;
                }
                content.push(v.deref().clone());
            }
        }
        content
    }

    pub fn by_part(&self, part_id: &str) -> Vec<CountCacheEntry> {
        let mut content = Vec::new();
        if let Some(c) = self.by_part.get(part_id) {
            for v in c {
                if v.count() == 0 && v.required == 0 && !v.show_empty {
                    continue;
                }
                content.push(v.deref().clone());
            }
        }
        content
    }

    pub(crate) fn clear(&mut self) {
        self.all.clear();
        self.by_location.clear();
        self.by_part.clear();
    }

    pub(crate) fn remove(&mut self, object_id: &str) {
        if let Some(cs) = self.by_location.get(object_id) {
            for e in cs {
                self.by_part.get_mut(&e.part_id).unwrap().remove(e);
                self.all.remove(e);
            }
        }

        if let Some(cs) = self.by_part.get(object_id) {
            for e in cs {
                self.by_location.get_mut(&e.location_id).unwrap().remove(e);
                self.all.remove(e);
            }
        }
    }
}

pub trait CountCacheSum {
    // Sum all cached values in the collection
    fn sum(&self) -> CountCacheSumResult;
}

#[derive(Debug, Clone, Copy)]
pub struct CountCacheSumResult {
    pub added: usize,
    pub removed: usize,
    pub required: usize,
}

impl CountCacheSumResult {
    pub fn count(&self) -> isize {
        (self.added as isize).saturating_sub_unsigned(self.removed)
    }
}

impl CountCacheSum for Vec<CountCacheEntry> {
    fn sum(&self) -> CountCacheSumResult {
        let mut cce = CountCacheSumResult {
            added: 0,
            removed: 0,
            required: 0,
        };

        for e in self.iter() {
            cce.added = cce.added.saturating_add(e.added);
            cce.removed = cce.removed.saturating_add(e.removed);
            cce.required = cce.required.saturating_add(e.required);
        }

        cce
    }
}
