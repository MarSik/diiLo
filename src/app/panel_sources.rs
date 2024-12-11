use crate::store::{cache::CountCacheSum, filter::Query, PartId, SourceId, Store};

use super::{
    caching_panel_data::{self, CachingPanelData, ParentPanel},
    model::{
        ActionDescriptor, EnterAction, FilterError, FilterStatus, PanelContent, PanelData,
        PanelItem,
    },
};

#[derive(Debug)]
pub struct PanelSourceSelection {
    parent: ParentPanel,
    cached: CachingPanelData,
    query: Option<Query>,
}

impl PanelSourceSelection {
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
            .filter(|p| {
                p.1.metadata
                    .types
                    .contains(&crate::store::ObjectType::Source)
            })
            .filter(|p| self.query.as_ref().map_or(true, |q| q.matches(p.1)))
            .map(|(p_id, p)| {
                let counts = store.count_by_source(p_id);
                let count = counts.sum();

                let ordered = (count.required as isize).saturating_sub_unsigned(count.added);
                let count = count.count();
                let data = if ordered > 0 {
                    format!("(+ {}) {}", ordered, count)
                } else {
                    count.to_string()
                };

                PanelItem::new(
                    &p.metadata.name,
                    None,
                    &p.metadata.summary,
                    &data,
                    Some(&p_id.into()),
                    None,
                )
            })
            .collect()
    }
}

impl PanelData for PanelSourceSelection {
    fn title(&self, _store: &Store) -> String {
        "Source list".to_owned()
    }

    fn data_type(&self) -> super::model::PanelContent {
        PanelContent::Sources
    }

    fn enter(self: Box<Self>, idx: usize, store: &Store) -> EnterAction {
        let loader = || self.load_cache(store);

        if idx == 0 {
            return self.parent.enter();
        }

        if let Some(item_id) = self.cached.item_id(idx, loader) {
            EnterAction(
                Box::new(PanelSourcesMenu::new(self, idx, item_id.part_type())),
                0,
            )
        } else {
            EnterAction(self, idx)
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
            .map(|source_id| {
                ActionDescriptor::new().add_source(SourceId::clone(source_id.part_type()))
            })
    }

    fn panel_title(&self, _store: &Store) -> String {
        "Sources".to_owned()
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
pub struct PanelSourcesMenu {
    parent: Box<dyn PanelData>,
    data: Vec<PanelItem>,
    parent_idx: usize,
    source_id: SourceId,
}

impl PanelSourcesMenu {
    pub fn new(parent: Box<dyn PanelData>, parent_idx: usize, source_id: &SourceId) -> Self {
        Self {
            parent,
            data: vec![
                PanelItem::new("<Back>", None, "", "", None, None),
                PanelItem::new(
                    "Parts",
                    None,
                    "show parts delivered from source",
                    "",
                    None,
                    None,
                ),
                PanelItem::new("Orders", None, "show orders", "", None, None),
            ],
            parent_idx,
            source_id: source_id.clone(),
        }
    }
}

impl PanelData for PanelSourcesMenu {
    fn data_type(&self) -> PanelContent {
        PanelContent::Sources
    }

    fn enter(self: Box<Self>, idx: usize, _store: &Store) -> EnterAction {
        match idx {
            0 => EnterAction(self.parent, self.parent_idx),
            1 => {
                let source_id = self.source_id.clone();
                EnterAction(
                    Box::new(PanelPartFromSourcesSelection::new(
                        self, idx, source_id, None,
                    )),
                    0,
                )
            }
            2 => {
                let source_id = self.source_id.clone();
                EnterAction(
                    Box::new(PanelOrderedFromSourcesSelection::new(
                        self, idx, source_id, None,
                    )),
                    0,
                )
            }
            _ => EnterAction(self.parent, self.parent_idx),
        }
    }

    fn title(&self, _store: &Store) -> String {
        "Select the view for the source.".to_owned()
    }

    fn item_summary(&self, idx: usize, _store: &Store) -> String {
        self.data[idx].name.to_owned()
    }

    fn len(&self, _store: &Store) -> usize {
        self.data.len()
    }

    fn items(&self, _store: &Store) -> Vec<PanelItem> {
        self.data.clone()
    }

    fn actionable_objects(&self, _idx: usize, _store: &Store) -> Option<ActionDescriptor> {
        Some(ActionDescriptor::new().add_source(self.source_id.clone()))
    }

    fn panel_title(&self, store: &Store) -> String {
        let loc = store
            .part_by_id(&self.source_id)
            .map(|p| p.metadata.name.clone())
            .unwrap_or("<unknown>".to_string());
        [self.parent.panel_title(store), loc].join(" / ")
    }

    fn reload(&mut self, store: &Store) {
        self.parent_idx =
            caching_panel_data::panel_reload(&mut self.parent, self.parent_idx, store);
    }

    fn item_actionable(&self, _idx: usize) -> bool {
        false
    }

    fn item_idx(&self, name: &str, _store: &Store) -> Option<usize> {
        match self.data[1..].binary_search_by_key(&name.to_string(), |v| v.name.to_lowercase()) {
            Ok(idx) => Some(idx + 1),
            Err(idx) => Some(idx + 2),
        }
    }

    fn item_name(&self, idx: usize, _store: &Store) -> String {
        self.data[idx].name.clone()
    }

    fn item(&self, idx: usize, _store: &Store) -> PanelItem {
        self.data[idx].clone()
    }

    fn filter(
        self: Box<Self>,
        _query: Query,
        _store: &Store,
    ) -> Result<EnterAction, super::model::FilterError> {
        Err(FilterError::NotSupported(EnterAction(self, 0)))
    }
}

#[derive(Debug)]
pub struct PanelPartFromSourcesSelection {
    parent: ParentPanel,
    source_id: SourceId,
    cached: CachingPanelData,
    query: Option<Query>,
}

impl PanelPartFromSourcesSelection {
    pub fn new(
        parent: Box<dyn PanelData>,
        parent_idx: usize,
        source_id: SourceId,
        query: Option<Query>,
    ) -> Self {
        Self {
            parent: ParentPanel::new(parent, parent_idx),
            cached: CachingPanelData::new(),
            source_id,
            query,
        }
    }

    fn load_cache(&self, store: &Store) -> Vec<PanelItem> {
        store
            .parts_by_source(&self.source_id)
            .iter()
            .filter(|p| self.query.as_ref().map_or(true, |q| q.matches(p.0)))
            .map(|(p, count)| {
                let data = if count.required() > count.added() {
                    format!(
                        "(+ {}) {}",
                        count.required().saturating_sub(count.added()),
                        count.count()
                    )
                } else {
                    count.count().to_string()
                };

                let subname = match count.part() {
                    PartId::Simple(_) => None,
                    PartId::Piece(_, _) => count
                        .part()
                        .subname()
                        .map(|s| format!("{}{}", s, p.metadata.unit)),
                    PartId::Unique(_, _) => count.part().subname(),
                };

                PanelItem::new(
                    &p.metadata.name,
                    subname,
                    &p.metadata.summary,
                    &data,
                    Some(&p.id.as_ref().into()),
                    Some(&self.source_id.as_ref().into()),
                )
            })
            .collect()
    }
}

impl PanelData for PanelPartFromSourcesSelection {
    fn title(&self, store: &Store) -> String {
        let loc = self.cached.title(store, &self.source_id.as_ref().into());
        match &self.query {
            Some(q) => format!("Parts from {}: query: {}", loc, q.current_query()).to_string(),
            None => format!("Parts from {}", loc).to_string(),
        }
    }

    fn panel_title(&self, store: &Store) -> String {
        self.parent.panel_title_const(store, "parts")
    }

    fn data_type(&self) -> PanelContent {
        PanelContent::PartsFromSources
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
        // Even when no item is selected, the parent itself can be a target
        let mut ad = ActionDescriptor::new().add_source(self.source_id.clone());

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
                Box::new(Self::new(parent.0, parent.1, self.source_id, None)),
                0,
            ))
        } else {
            Ok(EnterAction(
                Box::new(Self::new(parent.0, parent.1, self.source_id, Some(query))),
                0,
            ))
        }
    }
}

#[derive(Debug)]
pub struct PanelOrderedFromSourcesSelection {
    parent: ParentPanel,
    source_id: SourceId,
    cached: CachingPanelData,
    query: Option<Query>,
}

impl PanelOrderedFromSourcesSelection {
    pub fn new(
        parent: Box<dyn PanelData>,
        parent_idx: usize,
        source_id: SourceId,
        query: Option<Query>,
    ) -> Self {
        Self {
            parent: ParentPanel::new(parent, parent_idx),
            cached: CachingPanelData::new(),
            source_id,
            query,
        }
    }

    fn load_cache(&self, store: &Store) -> Vec<PanelItem> {
        store
            .parts_by_source(&self.source_id)
            .iter()
            .filter(|(_, count)| count.show_empty() || (count.required() > count.added()))
            .filter(|p| self.query.as_ref().map_or(true, |q| q.matches(p.0)))
            .map(|(p, count)| {
                let data = count.required().saturating_sub(count.added()).to_string();

                let subname = match count.part() {
                    PartId::Simple(_) => None,
                    PartId::Piece(_, _) => count
                        .part()
                        .subname()
                        .map(|s| format!("{}{}", s, p.metadata.unit)),
                    PartId::Unique(_, _) => count.part().subname(),
                };

                PanelItem::new(
                    &p.metadata.name,
                    subname,
                    &p.metadata.summary,
                    &data,
                    Some(&p.id.as_ref().into()),
                    Some(&self.source_id.as_ref().into()),
                )
            })
            .collect()
    }
}

impl PanelData for PanelOrderedFromSourcesSelection {
    fn title(&self, store: &Store) -> String {
        let loc = self.cached.title(store, &self.source_id.as_ref().into());
        match &self.query {
            Some(q) => {
                format!("Parts ordered from {}: query: {}", loc, q.current_query()).to_string()
            }
            None => format!("Parts ordered from {}", loc).to_string(),
        }
    }

    fn panel_title(&self, store: &Store) -> String {
        self.parent.panel_title_const(store, "orders")
    }

    fn data_type(&self) -> PanelContent {
        PanelContent::PartsInOrders
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
        // Even when no item is selected, the parent itself can be a target
        let mut ad = ActionDescriptor::new().add_source(self.source_id.clone());

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
                Box::new(Self::new(parent.0, parent.1, self.source_id, None)),
                0,
            ))
        } else {
            Ok(EnterAction(
                Box::new(Self::new(parent.0, parent.1, self.source_id, Some(query))),
                0,
            ))
        }
    }
}
