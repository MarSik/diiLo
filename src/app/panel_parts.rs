use crate::store::{
    cache::CountCacheSum, filter::Query, types::CountUnit, PartId, PartTypeId, Store,
};

use super::{
    caching_panel_data::{CachingPanelData, ParentPanel},
    model::{
        ActionDescriptor, EnterAction, FilterError, FilterStatus, PanelContent, PanelData,
        PanelItem,
    },
};

#[derive(Debug)]
pub struct PanelPartSelection {
    parent: ParentPanel,
    cached: CachingPanelData,
    query: Option<Query>,
}

impl PanelPartSelection {
    pub fn new(parent: Box<dyn PanelData>, parent_idx: usize, query: Option<Query>) -> Self {
        Self {
            parent: ParentPanel::new(parent, parent_idx),
            cached: CachingPanelData::new(),
            query,
        }
    }

    fn load_cache(&self, store: &Store) -> Vec<PanelItem> {
        store
            .all_objects()
            .iter()
            .filter(|p| p.1.metadata.types.contains(&crate::store::ObjectType::Part))
            .filter(|p| self.query.as_ref().map_or(true, |q| q.matches(p.1)))
            .map(|(p_id, p)| {
                let counts = store.count_by_part_type(p_id);
                let count = counts.sum();
                let count = count.added as isize - count.removed as isize;
                PanelItem::new(
                    &p.metadata.name,
                    None,
                    &p.metadata.summary,
                    &count.to_string(),
                    Some(&p_id.into()),
                    None,
                )
            })
            .collect()
    }
}

impl PanelData for PanelPartSelection {
    fn title(&self, _store: &Store) -> String {
        match &self.query {
            Some(q) => format!("Part list: {}", q).to_string(),
            None => "Nonfiltered part list".to_owned(),
        }
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
            EnterAction(
                Box::new(PanelPartLocationsSelection::new(
                    self,
                    idx,
                    PartTypeId::clone(item_id.part_type()),
                )),
                0,
            )
        } else {
            EnterAction(self, idx)
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

    fn item(&self, idx: usize, store: &Store) -> PanelItem {
        let loader = || self.load_cache(store);
        self.cached.item(idx, loader)
    }

    fn filter_status(&self) -> super::model::FilterStatus {
        match &self.query {
            Some(q) => FilterStatus::Query(q.current_query()),
            None => FilterStatus::NotApplied,
        }
    }

    fn filter(
        self: Box<Self>,
        query: Query,
        _store: &Store,
    ) -> Result<EnterAction, super::model::FilterError> {
        let parent = self.parent.enter();

        if query.is_empty() {
            Ok(EnterAction(
                Box::new(Self::new(parent.0, parent.1, None)),
                0,
            ))
        } else {
            Ok(EnterAction(
                Box::new(Self::new(parent.0, parent.1, Some(query))),
                0,
            ))
        }
    }
}

#[derive(Debug)]
pub struct PanelPartLocationsSelection {
    parent: ParentPanel,
    part_type_id: PartTypeId,
    cached: CachingPanelData,
}

impl PanelPartLocationsSelection {
    pub fn new(parent: Box<dyn PanelData>, parent_idx: usize, part_type_id: PartTypeId) -> Self {
        Self {
            parent: ParentPanel::new(parent, parent_idx),
            cached: CachingPanelData::new(),
            part_type_id,
        }
    }

    fn load_cache(&self, store: &Store) -> Vec<PanelItem> {
        store
            .locations_by_part_type(&self.part_type_id)
            .iter()
            .map(|(p, count)| {
                let data = if count.required() > 0 {
                    format!("(> {}) {}", count.required(), count.count())
                } else {
                    count.count().to_string()
                };

                let part = store.part_by_id(&self.part_type_id);

                let subname = match count.part() {
                    PartId::Simple(_) => None,
                    PartId::Piece(_, _) => count.part().subname().map(|s| {
                        format!(
                            "{}{}",
                            s,
                            part.map(|part| part.metadata.unit)
                                .unwrap_or(CountUnit::Piece)
                        )
                    }),
                    PartId::Unique(_, _) => count.part().subname(),
                };

                PanelItem::new(
                    &p.metadata.name,
                    subname,
                    &p.metadata.summary,
                    &data,
                    Some(count.location()),
                    Some(count.part()),
                )
            })
            .collect()
    }
}

impl PanelData for PanelPartLocationsSelection {
    fn title(&self, store: &Store) -> String {
        let loc = self.cached.title(store, &self.part_type_id.as_ref().into());
        format!("Locations of {}", loc).to_string()
    }

    fn panel_title(&self, store: &Store) -> String {
        self.parent
            .panel_title(store, &self.part_type_id.as_ref().into())
    }

    fn data_type(&self) -> PanelContent {
        PanelContent::LocationOfParts
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
        let mut ad = ActionDescriptor::new();

        self.load_cache(store);
        if let Some(location_id) = self.cached.item_id(idx, || self.load_cache(store)) {
            ad = ad.add_location(location_id);
        }
        if let Some(part_id) = self.cached.item_parent_id(idx, || self.load_cache(store)) {
            ad = ad.add_part(part_id);
        }

        Some(ad)
    }

    fn item_idx(&self, name: &str, store: &Store) -> Option<usize> {
        self.cached.item_idx(name, || self.load_cache(store))
    }

    fn item(&self, idx: usize, store: &Store) -> PanelItem {
        self.cached.item(idx, || self.load_cache(store))
    }

    fn filter(
        self: Box<Self>,
        _query: Query,
        _store: &Store,
    ) -> Result<EnterAction, super::model::FilterError> {
        Err(FilterError::NotSupported(EnterAction(self, 0)))
    }
}
