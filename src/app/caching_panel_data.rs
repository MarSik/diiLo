use std::cell::RefCell;

use crate::store::{PartId, Store};

use super::model::{EnterAction, PanelData, PanelItem};

#[derive(Debug)]
pub struct ParentPanel {
    parent: Box<dyn PanelData>,
    parent_idx: usize,
}

impl ParentPanel {
    pub fn new(parent: Box<dyn PanelData>, parent_idx: usize) -> Self {
        Self { parent, parent_idx }
    }

    pub fn enter(self) -> EnterAction {
        EnterAction(self.parent, self.parent_idx)
    }

    pub fn reload(&mut self, store: &Store) {
        // Make sure that the parent index stays valid after cache reload
        // For example when new elements are added to the list (or some are removed)
        self.parent_idx = panel_reload(&mut self.parent, self.parent_idx, store);
    }

    pub fn panel_title(&self, store: &Store, object_id: &PartId) -> String {
        let loc = store
            .part_by_id(object_id.part_type())
            .map(|p| p.metadata.name.clone())
            .unwrap_or("<unknown>".to_string());
        [self.parent.panel_title(store), loc].join(" / ")
    }

    pub fn panel_title_const(&self, store: &Store, name: &str) -> String {
        [self.parent.panel_title(store), name.to_string()].join(" / ")
    }
}

// Reload panel data and return the new index that is equivalent to the item_idx before
// the reload.
pub fn panel_reload(panel: &mut Box<dyn PanelData>, item_idx: usize, store: &Store) -> usize {
    let item_name = panel.item_name(item_idx, store);
    panel.reload(store);
    let new_idx = panel.item_idx(&item_name, store).unwrap_or(0);

    // The item_idx can return an id after the last element.
    // Make sure we catch that.
    let max = panel.len(store).saturating_sub(1);
    if new_idx > max {
        max
    } else {
        new_idx
    }
}

#[derive(Debug)]
pub struct CachingPanelData {
    cached: RefCell<Option<Vec<PanelItem>>>,
}

impl CachingPanelData {
    pub fn new() -> Self {
        Self {
            cached: RefCell::new(None),
        }
    }

    fn load_cache<L: Fn() -> Vec<PanelItem>>(&self, loader: L) {
        if let Ok(cache) = self.cached.try_borrow() {
            if cache.is_some() {
                return;
            }
        }

        let mut parts: Vec<PanelItem> = loader();

        parts.sort_by_key(|f| f.name.to_lowercase());
        let mut out = vec![PanelItem::new("<Back>", None, "", "", None, None)];
        out.extend(parts);

        self.cached.replace(Some(out));
    }

    pub fn reload(&mut self) {
        self.cached.replace(None);
    }

    pub fn title(&self, store: &Store, object_id: &PartId) -> String {
        store
            .part_by_id(object_id.part_type())
            .map(|p| p.metadata.name.clone())
            .unwrap_or("<unknown>".to_string())
    }

    pub fn item_actionable(&self, idx: usize) -> bool {
        idx > 0
    }

    pub fn item_summary<L: Fn() -> Vec<PanelItem>>(&self, idx: usize, loader: L) -> String {
        self.load_cache(loader);
        if idx == 0 {
            return "Back to type selection".to_owned();
        }

        return self.cached.borrow().as_ref().unwrap()[idx].summary.clone();
    }

    pub fn len<L: Fn() -> Vec<PanelItem>>(&self, loader: L) -> usize {
        self.load_cache(loader);
        self.cached.borrow().as_ref().unwrap().len()
    }

    pub fn items<L: Fn() -> Vec<PanelItem>>(&self, loader: L) -> Vec<PanelItem> {
        self.load_cache(loader);
        self.cached.borrow().clone().unwrap()
    }

    pub fn item_id<L: Fn() -> Vec<PanelItem>>(&self, idx: usize, loader: L) -> Option<PartId> {
        self.load_cache(loader);
        self.cached.borrow().as_ref().unwrap()[idx].id.clone()
    }

    pub fn item_parent_id<L: Fn() -> Vec<PanelItem>>(
        &self,
        idx: usize,
        loader: L,
    ) -> Option<PartId> {
        self.load_cache(loader);
        self.cached.borrow().as_ref().unwrap()[idx]
            .parent_id
            .clone()
    }

    pub fn item<L: Fn() -> Vec<PanelItem>>(&self, idx: usize, loader: L) -> PanelItem {
        self.load_cache(loader);
        self.cached.borrow().as_ref().unwrap()[idx].clone()
    }

    pub fn item_idx<L: Fn() -> Vec<PanelItem>>(&self, name: &str, loader: L) -> Option<usize> {
        self.load_cache(loader);

        let cache = self.cached.borrow();
        if cache.is_none() {
            return None;
        }
        let cache = cache.as_ref().unwrap();
        match cache.as_slice()[1..]
            .binary_search_by_key(&name.to_lowercase(), |v| v.name.to_lowercase())
        {
            Ok(idx) => Some(idx + 1),
            Err(idx) => Some(idx + 1),
        }
    }

    pub fn item_name<L: Fn() -> Vec<PanelItem>>(&self, idx: usize, loader: L) -> String {
        self.load_cache(loader);
        return self.cached.borrow().as_ref().unwrap()[idx].name.clone();
    }
}
