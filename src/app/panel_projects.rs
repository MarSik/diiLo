use crate::{
    app::model::PanelItem,
    store::{cache::CountCacheSum, LocationId, Store},
};

use super::{
    caching_panel_data::{CachingPanelData, ParentPanel},
    model::{ActionDescriptor, EnterAction, PanelContent, PanelData},
};

#[derive(Debug)]
pub struct PanelProjectSelection {
    parent: ParentPanel,
    cached: CachingPanelData,
}

impl PanelProjectSelection {
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
            .filter(|p| {
                p.1.metadata
                    .types
                    .contains(&crate::store::ObjectType::Project)
            })
            .map(|(p_id, p)| {
                let counts = store.count_by_project(p_id);
                let count = counts.sum();
                let count = count.count();
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

impl PanelData for PanelProjectSelection {
    fn title(&self, _store: &Store) -> String {
        "Project list".to_owned()
    }

    fn data_type(&self) -> super::model::PanelContent {
        PanelContent::Projects
    }

    fn enter(self: Box<Self>, idx: usize, store: &Store) -> EnterAction {
        let loader = || self.load_cache(store);

        if idx == 0 {
            return self.parent.enter();
        }

        if let Some(item_id) = self.cached.item_id(idx, loader) {
            return EnterAction(
                Box::new(PanelProjectPartsSelection::new(self, idx, item_id)),
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
        self.cached
            .item_id(idx, || self.load_cache(store))
            .map(|loc_id| ActionDescriptor::new().add_project(loc_id))
    }

    fn panel_title(&self, store: &Store) -> String {
        self.parent.panel_title_const(store, "projects")
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
pub struct PanelProjectPartsSelection {
    parent: ParentPanel,
    project_id: LocationId,
    cached: CachingPanelData,
}

impl PanelProjectPartsSelection {
    pub fn new(parent: Box<dyn PanelData>, parent_idx: usize, project_id: LocationId) -> Self {
        Self {
            parent: ParentPanel::new(parent, parent_idx),
            cached: CachingPanelData::new(),
            project_id,
        }
    }

    fn load_cache(&self, store: &Store) -> Vec<PanelItem> {
        store
            .parts_by_project(&self.project_id)
            .iter()
            .map(|(p, count)| {
                let data = if count.required() > 0 {
                    format!("(= {}) {}", count.required(), count.count())
                } else {
                    count.count().to_string()
                };

                PanelItem::new(&p.metadata.name, &p.metadata.summary, &data, Some(&p.id))
            })
            .collect()
    }
}

impl PanelData for PanelProjectPartsSelection {
    fn title(&self, store: &Store) -> String {
        let loc = self.cached.title(store, &self.project_id);
        format!("Parts in {}", loc).to_string()
    }

    fn panel_title(&self, store: &Store) -> String {
        self.parent.panel_title(store, &self.project_id)
    }

    fn data_type(&self) -> PanelContent {
        PanelContent::PartsInProjects
    }

    fn enter(self: Box<Self>, idx: usize, _store: &Store) -> EnterAction {
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
        let mut ad = ActionDescriptor::new().add_project(self.project_id.clone());

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
