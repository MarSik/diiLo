use crate::store::{PartId, Store};

use super::{
    model::{ActionDescriptor, EnterAction, OpaqueId, PanelContent, PanelData, PanelItem},
    panel_labels::PanelLabelSelection,
    panel_locations::PanelLocationSelection,
    panel_parts::PanelPartSelection,
    panel_projects::PanelProjectSelection,
    panel_sources::PanelSourceSelection,
};

#[derive(Debug)]
pub struct PanelTypeSelection {
    name: String,
    data: Vec<PanelItem>,
}

impl PanelTypeSelection {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            data: vec![
                PanelItem::new("Parts", "show all parts", "", None),
                PanelItem::new("Projects", "show all projects", "", None),
                PanelItem::new("Labels", "filter by label", "", None),
                PanelItem::new("Locations", "show all storage locations", "", None),
                PanelItem::new("Sources", "part sources and orders", "", None),
            ],
        }
    }
}

impl PanelData for PanelTypeSelection {
    fn data_type(&self) -> PanelContent {
        PanelContent::TypeSelection
    }

    fn enter(self: Box<Self>, idx: usize, store: &Store) -> EnterAction {
        match idx {
            0 => EnterAction(Box::new(PanelPartSelection::new(self)), 0),
            1 => EnterAction(Box::new(PanelProjectSelection::new(self)), 0),
            2 => EnterAction(Box::new(PanelLabelSelection::new(self)), 0),
            3 => EnterAction(Box::new(PanelLocationSelection::new(self)), 0),
            4 => EnterAction(Box::new(PanelSourceSelection::new(self)), 0),
            _ => EnterAction(self, idx),
        }
    }

    fn title(&self, store: &Store) -> String {
        "Select the view you want to work with.".to_owned()
    }

    fn item_summary(&self, idx: usize, store: &Store) -> String {
        self.data[idx].summary.clone()
    }

    fn len(&self, store: &Store) -> usize {
        self.data.len()
    }

    fn items(&self, store: &Store) -> Vec<PanelItem> {
        self.data.clone()
    }

    fn actionable_objects(&self, idx: usize, store: &Store) -> Option<ActionDescriptor> {
        None
    }

    fn panel_title(&self, store: &Store) -> String {
        self.name.clone()
    }

    fn reload(&mut self, store: &Store) {
        // NOP
    }

    fn item_actionable(&self, idx: usize) -> bool {
        false
    }

    fn item_idx(&self, id: &str, store: &Store) -> Option<usize> {
        None
    }

    fn item_name(&self, idx: usize, store: &Store) -> String {
        self.data.get(idx).unwrap().name.clone()
    }

    fn item(&self, idx: usize, store: &Store) -> PanelItem {
        self.data.get(idx).unwrap().clone()
    }
}
