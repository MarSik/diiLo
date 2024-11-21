use std::cell::RefCell;

use crate::store::{cache::CountCacheSum, LocationId, PartId, Store};

use super::{
    caching_panel_data::{CachingPanelData, ParentPanel},
    model::{ActionDescriptor, EnterAction, PanelContent, PanelData, PanelItem},
};

#[derive(Debug)]
pub struct PanelPartSelection {
    parent: ParentPanel,
    cached: CachingPanelData,
}

impl PanelPartSelection {
    pub fn new(parent: Box<dyn PanelData>) -> Self {
        Self {
            parent: ParentPanel::new(parent, 0),
            cached: CachingPanelData::new(),
        }
    }

    fn load_cache(&self, store: &Store) -> Vec<PanelItem> {
        store
            .all_objects()
            .iter()
            .filter(|p| p.1.metadata.types.contains(&crate::store::ObjectType::Part))
            .map(|(p_id, p)| {
                let counts = store.count_by_part(p_id);
                let count = counts.sum();
                let count = count.added as isize - count.removed as isize;
                PanelItem::new(
                    &p.metadata.name,
                    &p.metadata.summary,
                    &count.to_string(),
                    Some(p_id),
                )
            })
            .collect()
    }
}

impl PanelData for PanelPartSelection {
    fn title(&self, store: &Store) -> String {
        "Nonfiltered part list".to_owned()
    }

    fn data_type(&self) -> super::model::PanelContent {
        PanelContent::Parts
    }

    fn enter(self: Box<Self>, idx: usize, store: &Store) -> EnterAction {
        let loader = || self.load_cache(store);

        if idx == 0 {
            return self.parent.enter();
        }

        if let Some(item_id) = self.cached.item_id(idx, loader) {
            return EnterAction(
                Box::new(PanelPartLocationsSelection::new(self, idx, item_id)),
                0,
            );
        } else {
            return EnterAction(self, idx);
        }
    }

    fn item_summary(&self, idx: usize, store: &Store) -> String {
        let loader = || self.load_cache(store);
        self.cached.item_summary(idx, loader)
    }

    fn len(&self, store: &Store) -> usize {
        let loader = || self.load_cache(store);
        self.cached.len(loader)
    }

    fn items(&self, store: &Store) -> Vec<PanelItem> {
        let loader = || self.load_cache(store);
        self.cached.items(loader)
    }

    fn actionable_objects(&self, idx: usize, store: &Store) -> Option<ActionDescriptor> {
        let loader = || self.load_cache(store);
        let part_id = self.cached.item_id(idx, loader).unwrap();
        Some(ActionDescriptor::new().add_part(part_id))
    }

    fn panel_title(&self, store: &Store) -> String {
        self.parent.panel_title_const(store, "parts")
    }

    fn reload(&mut self, store: &Store) {
        self.parent.reload(store);
        self.cached.reload();
    }

    fn item_actionable(&self, idx: usize) -> bool {
        self.cached.item_actionable(idx)
    }

    fn item_idx(&self, id: &str, store: &Store) -> Option<usize> {
        self.cached.item_idx(id, || self.load_cache(store))
    }

    fn item_name(&self, idx: usize, store: &Store) -> String {
        let loader = || self.load_cache(store);
        self.cached.item_name(idx, loader)
    }

    fn item(&self, idx: usize, store: &Store) -> PanelItem {
        let loader = || self.load_cache(store);
        self.cached.item(idx, loader)
    }
}

#[derive(Debug)]
pub struct PanelPartLocationsSelection {
    parent: ParentPanel,
    part_id: PartId,
    cached: CachingPanelData,
}

impl PanelPartLocationsSelection {
    pub fn new(parent: Box<dyn PanelData>, parent_idx: usize, part_id: PartId) -> Self {
        Self {
            parent: ParentPanel::new(parent, parent_idx),
            cached: CachingPanelData::new(),
            part_id,
        }
    }

    fn load_cache(&self, store: &Store) -> Vec<PanelItem> {
        store
            .locations_by_part(&self.part_id)
            .iter()
            .map(|(p, count)| {
                let count = count.count();
                PanelItem::new(
                    &p.metadata.name,
                    &p.metadata.summary,
                    &count.to_string(),
                    Some(&p.id),
                )
            })
            .collect()
    }
}

impl PanelData for PanelPartLocationsSelection {
    fn title(&self, store: &Store) -> String {
        let loc = self.cached.title(store, &self.part_id);
        format!("Locations of {}", loc).to_string()
    }

    fn panel_title(&self, store: &Store) -> String {
        self.parent.panel_title(store, &self.part_id)
    }

    fn data_type(&self) -> PanelContent {
        PanelContent::LocationOfParts
    }

    fn enter(self: Box<Self>, idx: usize, store: &Store) -> EnterAction {
        if idx == 0 {
            return self.parent.enter();
        }

        EnterAction(self, idx)
    }

    fn reload(&mut self, store: &Store) {
        self.cached.reload();
        self.parent.reload(store);
    }

    fn item_actionable(&self, idx: usize) -> bool {
        idx > 0
    }

    fn item_summary(&self, idx: usize, store: &Store) -> String {
        self.cached.item_summary(idx, || self.load_cache(store))
    }

    fn len(&self, store: &Store) -> usize {
        self.cached.len(|| self.load_cache(store))
    }

    fn items(&self, store: &Store) -> Vec<PanelItem> {
        self.cached.items(|| self.load_cache(store))
    }

    fn actionable_objects(&self, idx: usize, store: &Store) -> Option<ActionDescriptor> {
        // Even when no item is selected, the parent itself can be a target
        let mut ad = ActionDescriptor::new().add_part(self.part_id.clone());

        self.load_cache(store);
        if let Some(location_id) = self.cached.item_id(idx, || self.load_cache(store)) {
            ad = ad.add_location(location_id);
        }

        Some(ad)
    }

    fn item_idx(&self, name: &str, store: &Store) -> Option<usize> {
        self.cached.item_idx(name, || self.load_cache(store))
    }

    fn item_name(&self, idx: usize, store: &Store) -> String {
        self.cached.item_name(idx, || self.load_cache(store))
    }

    fn item(&self, idx: usize, store: &Store) -> PanelItem {
        self.cached.item(idx, || self.load_cache(store))
    }
}
