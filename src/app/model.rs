use std::fmt::Display;

use crate::store::{filter::Query, LocationId, PartId, SourceId, Store};

use super::panel_typesel::PanelTypeSelection;

#[derive(Debug)]
pub(super) struct Model {
    pub(super) panel_a: Box<dyn PanelData>,
    pub(super) panel_b: Box<dyn PanelData>,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            panel_a: Box::new(PanelTypeSelection::new("[A]")),
            panel_b: Box::new(PanelTypeSelection::new("[B]")),
        }
    }
}

impl Model {}

// PanelData defines a virtual interface for the transfer of
// data between the real data store backing the implementation
// on one side and the user interface (panels, buttons) on the
// other side.
// It should be lightweight and store the minimal amount of data
// necessary. This can include the cached data loaded from the
// store (or it can query the store every time).
pub(super) trait PanelData: std::fmt::Debug {
    // Title, path or help to show
    fn title(&self, store: &Store) -> String;

    // Item summary
    fn panel_title(&self, store: &Store) -> String;

    // Data type, needed for action detection
    fn data_type(&self) -> PanelContent;

    // What happens when element is selected
    // This consumes the panel data and it must either return self
    // or as a subscreen it must store the parent data
    // so it can return it on exit
    fn enter(self: Box<Self>, idx: usize, store: &Store) -> EnterAction;

    // Refresh cached data from the store
    fn reload(&mut self, store: &Store);

    // Is the currently selected element Fx actionable
    fn item_actionable(&self, idx: usize) -> bool;

    // Item summary
    fn item_summary(&self, idx: usize, store: &Store) -> String;

    // Item name
    fn item_name(&self, idx: usize, store: &Store) -> String;

    // Item count
    fn len(&self, store: &Store) -> usize;

    // Selected item
    fn item(&self, idx: usize, store: &Store) -> PanelItem;

    // Items to show
    fn items(&self, store: &Store) -> Vec<PanelItem>;

    // Get action descriptor of the selected item
    fn actionable_objects(&self, idx: usize, store: &Store) -> Option<ActionDescriptor>;

    // Find the view index of part with name
    // When no such part exists, find the first name that is alphabetically
    // higher than provided parameter.
    // Warning: This can return an index after the last element = out of bounds of the possibly
    //          cached values.
    // Return None when the content is empty.
    fn item_idx(&self, name: &str, store: &Store) -> Option<usize>;

    // Return the filter status of this panel
    // It can signal that filter is not supported (and filter key should do nothing),
    // or that filter can be used, but it is not at the moment, or return
    // the current filter query.
    fn filter_status(&self) -> FilterStatus {
        FilterStatus::NotSupported
    }

    // Update the filter query the current panel uses
    fn filter(self: Box<Self>, query: Query, store: &Store) -> Result<EnterAction, FilterError>;
}

// the first element is the panel data source to activate
// the second element is the menu item to activate after move
pub struct EnterAction(pub(super) Box<dyn PanelData>, pub(super) usize);

pub enum FilterError {
    NotSupported(EnterAction),
}

impl Display for FilterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FilterError::NotSupported(_) => f.write_str("filter not supported"),
        }
    }
}

impl FilterError {
    pub fn return_to(self) -> EnterAction {
        match self {
            FilterError::NotSupported(enter_action) => enter_action,
        }
    }
}

pub enum FilterStatus {
    NotSupported,
    NotApplied,
    Query(String),
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum PanelContent {
    #[default]
    None,
    TypeSelection,
    Parts,
    Locations,
    PartsInLocation,
    LocationOfParts,
    LabelKeys,
    Labels,
    PartsWithLabels,
    Sources,
    PartsFromSources,
    PartsInOrders,
    Projects,
    PartsInProjects,
}

impl PanelContent {
    // Can a panel with specific type support the make operation?
    // This is a basic ruleset, some elements might override true back to false
    // based on specific conditions.
    pub fn can_make(&self) -> bool {
        match self {
            PanelContent::None => false,
            PanelContent::TypeSelection => false,
            PanelContent::Parts => true,
            PanelContent::Locations => true,
            PanelContent::PartsInLocation => true,
            PanelContent::LocationOfParts => true,
            PanelContent::Labels => true,
            PanelContent::LabelKeys => true,
            PanelContent::PartsWithLabels => true, // Serves as a shortcut to defining new part with the label
            PanelContent::Sources => true,
            PanelContent::PartsFromSources => true, // But serves as a shortcut for defining an order
            PanelContent::PartsInOrders => true, // Serves as a shortcut to defining new part and placing it to the order
            PanelContent::Projects => true,
            PanelContent::PartsInProjects => true, // Serves as a shortcut for defining requirements
        }
    }

    // Can a panel with specific type support the delete operation?
    // This is a basic ruleset, some elements might override true back to false
    // based on specific conditions.
    pub fn can_delete(&self) -> bool {
        match self {
            PanelContent::None => false,
            PanelContent::TypeSelection => false,
            PanelContent::Parts => true,     // When total count is zero
            PanelContent::Locations => true, // When total count is zero
            PanelContent::PartsInLocation => true,
            PanelContent::LocationOfParts => true,
            PanelContent::Labels => true,
            PanelContent::LabelKeys => true,
            PanelContent::PartsWithLabels => true, // Removes label
            PanelContent::Sources => true,         // Maybe? When no orders?
            PanelContent::PartsFromSources => true,
            PanelContent::PartsInOrders => true, // When not delivered
            PanelContent::Projects => true,      // When not soldered into
            PanelContent::PartsInProjects => true, // When count is zero,
        }
    }

    // A modifier that changes the type when the content points to
    // an inactive part.
    // An example:
    //  PartsInLocation normally act as target for both part operations
    //  and for location operations (copy-into).
    //  When no part is selected, the copy-into location can still
    //  be performed.
    pub fn on_part_inactive(&self, part_inactive: bool) -> Self {
        if part_inactive {
            // Part inactive, degrade to parent type
            match self {
                PanelContent::None => PanelContent::None,
                PanelContent::TypeSelection => PanelContent::None,
                PanelContent::Parts => PanelContent::None,
                PanelContent::Locations => PanelContent::None,
                PanelContent::PartsInLocation => PanelContent::Locations,
                PanelContent::LocationOfParts => PanelContent::None,
                PanelContent::LabelKeys => PanelContent::None,
                PanelContent::Labels => PanelContent::LabelKeys,
                PanelContent::PartsWithLabels => PanelContent::Labels,
                PanelContent::Sources => PanelContent::None,
                PanelContent::PartsFromSources => PanelContent::Sources,
                PanelContent::PartsInOrders => PanelContent::Sources,
                PanelContent::Projects => PanelContent::None,
                PanelContent::PartsInProjects => PanelContent::Projects,
            }
        } else {
            // Part active, just return the type as it was
            *self
        }
    }

    pub fn contains_parts(&self) -> bool {
        match self {
            PanelContent::None => false,
            PanelContent::TypeSelection => false,
            PanelContent::Parts => true,
            PanelContent::Locations => false,
            PanelContent::PartsInLocation => true,
            PanelContent::LocationOfParts => false,
            PanelContent::LabelKeys => false,
            PanelContent::Labels => false,
            PanelContent::PartsWithLabels => true,
            PanelContent::Sources => false,
            PanelContent::PartsFromSources => true,
            PanelContent::PartsInOrders => true,
            PanelContent::Projects => false,
            PanelContent::PartsInProjects => true,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct PanelItem {
    pub name: String,
    pub summary: String,
    pub data: String,
    pub id: Option<PartId>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct ActionDescriptor {
    part: Option<PartId>,
    location: Option<LocationId>,
    source: Option<SourceId>,
    project: Option<LocationId>,
    label_value: Option<String>,
    label_key: Option<String>,
}

impl ActionDescriptor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_part(self, part: PartId) -> Self {
        Self {
            part: Some(part),
            ..self
        }
    }

    pub fn add_location(self, location: LocationId) -> Self {
        Self {
            location: Some(location),
            ..self
        }
    }

    pub fn add_source(self, source: SourceId) -> Self {
        Self {
            source: Some(source),
            ..self
        }
    }

    pub fn add_project(self, project: LocationId) -> Self {
        Self {
            project: Some(project),
            ..self
        }
    }

    pub(crate) fn add_label_key(self, label_key: &str) -> ActionDescriptor {
        Self {
            label_key: Some(label_key.to_string()),
            ..self
        }
    }

    pub fn add_label(self, label_key: &str, label_value: &str) -> Self {
        Self {
            label_value: Some(label_value.to_string()),
            ..self.add_label_key(label_key)
        }
    }

    pub(super) fn part(&self) -> Option<&PartId> {
        self.part.as_ref()
    }

    pub(super) fn location(&self) -> Option<&LocationId> {
        self.location.as_ref()
    }

    pub(super) fn source(&self) -> Option<&SourceId> {
        self.source.as_ref()
    }

    pub(super) fn project(&self) -> Option<&PartId> {
        self.project.as_ref()
    }

    pub(super) fn label_key(&self) -> Option<&String> {
        self.label_key.as_ref()
    }

    pub(super) fn label(&self) -> Option<(String, String)> {
        self.label_key
            .as_ref()
            .and_then(|k| self.label_value.as_ref().map(|v| (k.clone(), v.clone())))
    }
}

impl PanelItem {
    pub fn new(name: &str, summary: &str, data: &str, id: Option<&PartId>) -> Self {
        Self {
            name: name.to_string(),
            summary: summary.to_string(),
            data: data.to_string(),
            id: id.cloned(),
        }
    }
}
