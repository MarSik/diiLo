use std::cell::RefCell;

use crate::store::{cache::CountCacheSum, PartId, Store};

use super::{
    caching_panel_data::{CachingPanelData, ParentPanel},
    model::{ActionDescriptor, EnterAction, OpaqueId, PanelContent, PanelData, PanelItem},
    panel_parts::PanelPartLocationsSelection,
};

#[derive(Debug)]
pub struct PanelLabelSelection {
    parent: ParentPanel,
    cached: CachingPanelData,
}

impl PanelLabelSelection {
    pub fn new(parent: Box<dyn PanelData>) -> Self {
        Self {
            parent: ParentPanel::new(parent, 0),
            cached: CachingPanelData::new(),
        }
    }

    fn load_cache(&self, store: &Store) -> Vec<PanelItem> {
        store
            .all_label_keys()
            .iter()
            .map(|(label_key, count)| {
                PanelItem::new(
                    &label_key,
                    "",
                    &count.to_string(),
                    Some(&label_key.as_str().into()),
                )
            })
            .collect()
    }
}

impl PanelData for PanelLabelSelection {
    fn title(&self, store: &Store) -> String {
        "Label list".to_owned()
    }

    fn data_type(&self) -> super::model::PanelContent {
        PanelContent::LabelKeys
    }

    fn enter(self: Box<Self>, idx: usize, store: &Store) -> EnterAction {
        let loader = || self.load_cache(store);

        if idx == 0 {
            return self.parent.enter();
        }

        if let Some(item_id) = self.cached.item_id(idx, loader) {
            return EnterAction(
                Box::new(PanelLabelValueSelection::new(
                    self,
                    item_id.to_string(),
                    idx,
                )),
                0,
            );
        } else {
            return EnterAction(self, idx);
        }
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
        let label_key = self.cached.item_name(idx, || self.load_cache(store));
        Some(ActionDescriptor::new().add_label_key(&label_key))
    }

    fn panel_title(&self, store: &Store) -> String {
        self.parent.panel_title_const(store, "labels")
    }

    fn reload(&mut self, store: &Store) {
        self.cached.reload();
        self.parent.reload(store);
    }

    fn item_actionable(&self, idx: usize) -> bool {
        self.cached.item_actionable(idx)
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

#[derive(Debug)]
pub struct PanelLabelValueSelection {
    parent: ParentPanel,
    cached: CachingPanelData,
    key: String,
}

impl PanelLabelValueSelection {
    pub fn new(parent: Box<dyn PanelData>, key: String, return_idx: usize) -> Self {
        Self {
            parent: ParentPanel::new(parent, return_idx),
            key,
            cached: CachingPanelData::new(),
        }
    }

    fn load_cache(&self, store: &Store) -> Vec<PanelItem> {
        store
            .all_label_values(&self.key)
            .iter()
            .map(|(label_value, count)| {
                PanelItem::new(
                    &label_value,
                    "",
                    &count.to_string(),
                    Some(&label_value.as_str().into()),
                )
            })
            .collect()
    }
}

impl PanelData for PanelLabelValueSelection {
    fn title(&self, store: &Store) -> String {
        format!("Label values for {}", self.key).to_owned()
    }

    fn data_type(&self) -> super::model::PanelContent {
        PanelContent::Labels
    }

    fn enter(self: Box<Self>, idx: usize, store: &Store) -> EnterAction {
        // Go up
        if idx == 0 {
            return self.parent.enter();
        }

        let label_key = self.key.clone();
        if let Some(item_id) = self.cached.item_id(idx, || self.load_cache(store)) {
            return EnterAction(
                Box::new(PanelPartByLabelSelection::new(
                    self, idx, &label_key, &item_id,
                )),
                0,
            );
        } else {
            return EnterAction(self, idx);
        }
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
        self.load_cache(store);
        let label_val = self.cached.item_id(idx, || self.load_cache(store));
        label_val
            .map(|label_val| ActionDescriptor::new().add_label(&self.key, &label_val))
            .or_else(|| Some(ActionDescriptor::new().add_label_key(&self.key)))
    }

    fn panel_title(&self, store: &Store) -> String {
        self.parent.panel_title_const(store, &self.key)
    }

    fn reload(&mut self, store: &Store) {
        self.cached.reload();
        self.parent.reload(store);
    }

    fn item_actionable(&self, idx: usize) -> bool {
        self.cached.item_actionable(idx)
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

#[derive(Debug)]
pub struct PanelPartByLabelSelection {
    parent: ParentPanel,
    label_key: String,
    label_value: String,
    cached: CachingPanelData,
}

impl PanelPartByLabelSelection {
    pub fn new(
        parent: Box<dyn PanelData>,
        parent_idx: usize,
        label_key: &str,
        label_value: &str,
    ) -> Self {
        Self {
            parent: ParentPanel::new(parent, parent_idx),
            cached: CachingPanelData::new(),
            label_key: label_key.to_string(),
            label_value: label_value.to_string(),
        }
    }

    fn load_cache(&self, store: &Store) -> Vec<PanelItem> {
        store
            .parts_by_label(&self.label_key, &self.label_value)
            .iter()
            .map(|p| {
                let c = store.count_by_part(&p.id).sum();
                PanelItem::new(
                    &p.metadata.name,
                    &p.metadata.summary,
                    &c.count().to_string(),
                    Some(&p.id),
                )
            })
            .collect()
    }
}

impl PanelData for PanelPartByLabelSelection {
    fn title(&self, store: &Store) -> String {
        format!("Parts marked as {}: {}", self.label_key, self.label_value).to_string()
    }

    fn panel_title(&self, store: &Store) -> String {
        self.parent.panel_title_const(store, &self.label_value)
    }

    fn data_type(&self) -> PanelContent {
        PanelContent::PartsWithLabels
    }

    fn enter(self: Box<Self>, idx: usize, store: &Store) -> EnterAction {
        // Go up
        if idx == 0 {
            return self.parent.enter();
        }

        if let Some(item_id) = self.cached.item_id(idx, || self.load_cache(store)) {
            return EnterAction(
                Box::new(PanelPartLocationsSelection::new(self, idx, item_id)),
                0,
            );
        } else {
            return EnterAction(self, idx);
        }
    }

    fn reload(&mut self, store: &Store) {
        self.cached.reload();
        self.parent.reload(store);
    }

    fn item_actionable(&self, idx: usize) -> bool {
        self.cached.item_actionable(idx)
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
        let mut ad = ActionDescriptor::new().add_label(&self.label_key, &self.label_value);

        self.load_cache(store);
        if let Some(part_id) = self.cached.item_id(idx, || self.load_cache(store)) {
            ad = ad.add_part(part_id);
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