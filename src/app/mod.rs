use std::{mem::replace, rc::Rc};

use errs::AppError;
use log::{debug, error, info};
use model::{
    ActionDescriptor, EnterAction, Model, PanelContent, PanelData, PanelItem, PanelItemDisplayId,
};
use tui_input::Input;
use view::{ActivePanel, DialogState, View};

use crate::store::{
    Part, PartId, PartTypeId, SourceId, Store, filter::Query, types::CountTracking,
};

mod action_create;
mod action_delete;
mod action_label;
mod action_local;
mod action_move;
mod action_orders;
mod action_solder;
mod caching_panel_data;
pub mod errs;
mod kbd;
mod model;
mod panel_labels;
mod panel_locations;
mod panel_parts;
mod panel_projects;
mod panel_sources;
mod panel_typesel;
mod render;
mod view;

#[cfg(test)]
mod tests;

pub struct App {
    // State of visual elements, active panel, dialogs etc.
    // This is used to switch HOW the model content is displayed.
    view: View,
    // The app data and synchronization engine
    store: Store,
    // The interface between different data sources and the UI,
    // that holds no data. This is used to switch WHAT content is displayed.
    model: Model,
}

#[derive(Debug, PartialEq)]
pub enum AppEvents {
    Nop,
    // Redraw UI
    Redraw,
    // Clear terminal and redraw
    FullRedraw,
    // Reload data model
    ReloadData,
    // Reload data model and then select item on active panel
    // by id and when not found by name
    ReloadDataSelectByDisplayId(PanelItemDisplayId, String),
    // Reload data model and then select item on active panel
    // by part id and when not found by name
    ReloadDataSelectByPartId(PartId, String),
    // Reload data model and then select item on active panel by name
    ReloadDataSelectByName(String),
    // Select
    // by id and when not found by name
    SelectByDisplayId(PanelItemDisplayId, String),
    // Select
    // by id and when not found by name
    SelectByPartId(PartId, String),
    // Select by name
    SelectByName(String),
    // Start editor and reload after edit is complete
    Edit(PartId),
    // Quit application
    Quit,
}

impl AppEvents {
    pub fn or(self, other: AppEvents) -> AppEvents {
        match self {
            AppEvents::Nop => other,
            other => other,
        }
    }

    pub fn select_by_name(self, name: &str) -> AppEvents {
        match self {
            AppEvents::ReloadData | AppEvents::ReloadDataSelectByDisplayId(_, _) => {
                AppEvents::ReloadDataSelectByName(name.to_string())
            }
            _ => AppEvents::SelectByName(name.to_string()),
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum ActionVariant {
    #[default]
    None,
    Error,
    AddLabelToPart,
    AddPartToLabel,
    RemoveLabelFromPart,
    RemovePartFromLabel,
    CreatePart,
    ClonePart,
    RequirePart,
    OrderPart,
    MovePart,
    DeliverPart,
    SolderPart,
    UnsolderPart,
    OrderPartLocal,
    RequirePartInLocationLocal,
    RequirePartInProjectLocal,
    ForceCount,
    ForceCountLocal,
    ForceCountProject,
    ForceCountProjectLocal,
    Delete,
    SplitLocal,
}

impl ActionVariant {
    pub fn name(self) -> &'static str {
        match self {
            ActionVariant::None => "",
            ActionVariant::Error => "",
            ActionVariant::AddLabelToPart => "label",
            ActionVariant::RemoveLabelFromPart => "unlabel",
            ActionVariant::AddPartToLabel => "label",
            ActionVariant::RemovePartFromLabel => "unlabel",
            ActionVariant::CreatePart => "make",
            ActionVariant::ClonePart => "clone",
            ActionVariant::RequirePart => "require",
            ActionVariant::OrderPart => "order",
            ActionVariant::MovePart => "move",
            ActionVariant::DeliverPart => "deliver",
            ActionVariant::SolderPart => "solder",
            ActionVariant::UnsolderPart => "unsolder",
            ActionVariant::OrderPartLocal => "order",
            ActionVariant::RequirePartInLocationLocal => "require",
            ActionVariant::RequirePartInProjectLocal => "require",
            ActionVariant::ForceCount => "force count",
            ActionVariant::ForceCountLocal => "force count",
            ActionVariant::ForceCountProject => "force count",
            ActionVariant::ForceCountProjectLocal => "force count",
            ActionVariant::Delete => "delete",
            ActionVariant::SplitLocal => "split",
        }
    }

    pub fn dual_panel(self) -> bool {
        !matches!(
            self,
            ActionVariant::OrderPartLocal
                | ActionVariant::RequirePartInLocationLocal
                | ActionVariant::RequirePartInProjectLocal
                | ActionVariant::Delete
                | ActionVariant::ClonePart
                | ActionVariant::CreatePart
                | ActionVariant::ForceCountLocal
                | ActionVariant::ForceCountProject
                | ActionVariant::ForceCountProjectLocal
                | ActionVariant::SplitLocal
        )
    }

    pub fn description(self) -> &'static str {
        match self {
            ActionVariant::None => "",
            ActionVariant::Error => "",
            ActionVariant::AddLabelToPart => "Add label",
            ActionVariant::RemoveLabelFromPart => "Remove label",
            ActionVariant::AddPartToLabel => "Add label",
            ActionVariant::RemovePartFromLabel => "Remove label",
            ActionVariant::CreatePart => "Create new part",
            ActionVariant::ClonePart => "Clone part",
            ActionVariant::RequirePart => "Request part",
            ActionVariant::OrderPart => "Order part",
            ActionVariant::MovePart => "Move part",
            ActionVariant::DeliverPart => "Deliver part",
            ActionVariant::SolderPart => "Solder part",
            ActionVariant::UnsolderPart => "Unsolder part",
            ActionVariant::OrderPartLocal => "Order part",
            ActionVariant::RequirePartInLocationLocal => "Require part",
            ActionVariant::RequirePartInProjectLocal => "Require part",
            ActionVariant::Delete => "Delete part",
            ActionVariant::ForceCount => "Force count",
            ActionVariant::ForceCountLocal => "Force count",
            ActionVariant::ForceCountProject => "Force count",
            ActionVariant::ForceCountProjectLocal => "Force count",
            ActionVariant::SplitLocal => "Split piece",
        }
    }

    pub fn countable(self) -> bool {
        match self {
            ActionVariant::None => false,
            ActionVariant::Error => false,
            ActionVariant::AddLabelToPart => false,
            ActionVariant::RemoveLabelFromPart => false,
            ActionVariant::AddPartToLabel => false,
            ActionVariant::RemovePartFromLabel => false,
            ActionVariant::CreatePart => false,
            ActionVariant::ClonePart => false,
            ActionVariant::RequirePart => true,
            ActionVariant::OrderPart => true,
            ActionVariant::MovePart => true,
            ActionVariant::DeliverPart => true,
            ActionVariant::SolderPart => true,
            ActionVariant::UnsolderPart => true,
            ActionVariant::OrderPartLocal => true,
            ActionVariant::RequirePartInLocationLocal => true,
            ActionVariant::RequirePartInProjectLocal => true,
            ActionVariant::Delete => false,
            ActionVariant::ForceCount => true,
            ActionVariant::ForceCountLocal => true,
            ActionVariant::ForceCountProject => true,
            ActionVariant::ForceCountProjectLocal => true,
            ActionVariant::SplitLocal => true,
        }
    }
}

impl App {
    pub fn new(store: Store) -> anyhow::Result<Self> {
        Ok(Self {
            store,
            view: View::default(),
            model: Model::default(),
        })
    }

    // Return an action pair (active part type -> inactive part type) that can then
    // be used to figure out what operations are available.
    // Take into account that a non-item selection might be active ("back" item) and
    // degrade to the proper parent types.
    fn get_action_direction(&self) -> (PanelContent, PanelContent) {
        match self.view.active {
            view::ActivePanel::PanelA => (
                self.model.panel_a.data_type().on_part_inactive(
                    !self
                        .model
                        .panel_a
                        .item_actionable(self.view.panel_a.selected),
                ),
                self.model.panel_b.data_type().on_part_inactive(
                    !self
                        .model
                        .panel_b
                        .item_actionable(self.view.panel_b.selected),
                ),
            ),
            view::ActivePanel::PanelB => (
                self.model.panel_b.data_type().on_part_inactive(
                    !self
                        .model
                        .panel_b
                        .item_actionable(self.view.panel_b.selected),
                ),
                self.model.panel_a.data_type().on_part_inactive(
                    !self
                        .model
                        .panel_a
                        .item_actionable(self.view.panel_a.selected),
                ),
            ),
        }
    }

    fn get_active_panel_data(&self) -> &dyn PanelData {
        match self.view.active {
            view::ActivePanel::PanelA => self.model.panel_a.as_ref(),
            view::ActivePanel::PanelB => self.model.panel_b.as_ref(),
        }
    }

    fn get_inactive_panel_data(&self) -> &dyn PanelData {
        match self.view.active {
            view::ActivePanel::PanelA => self.model.panel_b.as_ref(),
            view::ActivePanel::PanelB => self.model.panel_a.as_ref(),
        }
    }

    pub fn f9_action(&self) -> ActionVariant {
        match self.get_action_direction() {
            (PanelContent::PartsFromSources, _) => ActionVariant::OrderPartLocal,
            (PanelContent::PartsInOrders, _) => ActionVariant::OrderPartLocal,
            (PanelContent::PartsInLocation, _) => ActionVariant::RequirePartInLocationLocal,
            (PanelContent::LocationOfParts, _) => ActionVariant::RequirePartInLocationLocal,
            (PanelContent::PartsInProjects, _) => ActionVariant::RequirePartInProjectLocal,
            (_, _) => ActionVariant::None,
        }
    }

    pub fn ctrl_f9_action(&self) -> ActionVariant {
        match self.get_action_direction() {
            (PanelContent::PartsInLocation, _) => ActionVariant::ForceCountLocal,
            (PanelContent::LocationOfParts, _) => ActionVariant::ForceCountLocal,
            (PanelContent::PartsInProjects, _) => ActionVariant::ForceCountProjectLocal,
            (_, _) => ActionVariant::None,
        }
    }

    pub fn ctrl_f6_action(&self) -> ActionVariant {
        match self.get_action_direction() {
            (PanelContent::PartsInLocation, _) => ActionVariant::SplitLocal,
            (PanelContent::LocationOfParts, _) => ActionVariant::SplitLocal,
            (_, _) => ActionVariant::None,
        }
    }

    pub fn f5_action(&self) -> ActionVariant {
        match self.get_action_direction() {
            (PanelContent::TypeSelection, _) => ActionVariant::None,
            (_, PanelContent::TypeSelection) => ActionVariant::None,

            (p, PanelContent::Locations) if p.contains_parts() => ActionVariant::RequirePart,
            (p, PanelContent::PartsInLocation) if p.contains_parts() => ActionVariant::RequirePart,

            (p, PanelContent::Labels) if p.contains_parts() => ActionVariant::AddPartToLabel,
            (p, PanelContent::PartsWithLabels) if p.contains_parts() => {
                ActionVariant::AddPartToLabel
            }

            (p, PanelContent::Sources) if p.contains_parts() => ActionVariant::OrderPart,
            (p, PanelContent::PartsInOrders) if p.contains_parts() => ActionVariant::OrderPart,
            (p, PanelContent::PartsFromSources) if p.contains_parts() => ActionVariant::OrderPart,

            (p, PanelContent::PartsInProjects) if p.contains_parts() => ActionVariant::RequirePart,
            (p, PanelContent::Projects) if p.contains_parts() => ActionVariant::RequirePart,
            (PanelContent::LocationOfParts, PanelContent::Projects) => ActionVariant::RequirePart,

            (PanelContent::Parts, _) => ActionVariant::ClonePart,
            (PanelContent::Projects, _) => ActionVariant::ClonePart,

            (PanelContent::Locations, _) => ActionVariant::None,

            (PanelContent::Labels, p) if p.contains_parts() => ActionVariant::AddLabelToPart,
            (PanelContent::Labels, _) => ActionVariant::None,

            (PanelContent::Sources, _) => ActionVariant::None,

            (_, _) => ActionVariant::None,
        }
    }

    pub fn f6_action(&self) -> ActionVariant {
        match self.get_action_direction() {
            (PanelContent::TypeSelection, _) => ActionVariant::None,
            (_, PanelContent::TypeSelection) => ActionVariant::None,

            (PanelContent::PartsInLocation, PanelContent::Locations) => ActionVariant::MovePart,
            (PanelContent::PartsInLocation, PanelContent::LocationOfParts) => {
                ActionVariant::MovePart
            }
            (PanelContent::PartsInLocation, PanelContent::PartsInLocation) => {
                ActionVariant::MovePart
            }
            (PanelContent::LocationOfParts, PanelContent::PartsInLocation) => {
                ActionVariant::MovePart
            }
            (PanelContent::LocationOfParts, PanelContent::Locations) => ActionVariant::MovePart,
            (PanelContent::LocationOfParts, PanelContent::LocationOfParts) => {
                ActionVariant::MovePart
            }

            (p, PanelContent::Labels) if p.contains_parts() => ActionVariant::RemovePartFromLabel,
            (p, PanelContent::PartsWithLabels) if p.contains_parts() => {
                ActionVariant::RemovePartFromLabel
            }

            (PanelContent::PartsInLocation, PanelContent::Projects) => ActionVariant::SolderPart,
            (PanelContent::PartsInLocation, PanelContent::PartsInProjects) => {
                ActionVariant::SolderPart
            }
            (PanelContent::LocationOfParts, PanelContent::Projects) => ActionVariant::SolderPart,
            (PanelContent::LocationOfParts, PanelContent::PartsInProjects) => {
                ActionVariant::SolderPart
            }

            (PanelContent::Parts, PanelContent::Locations) => ActionVariant::ForceCount,
            (PanelContent::Parts, PanelContent::PartsInLocation) => ActionVariant::ForceCount,
            (PanelContent::Parts, PanelContent::PartsInProjects) => {
                ActionVariant::ForceCountProject
            }

            (PanelContent::PartsWithLabels, PanelContent::Locations) => ActionVariant::ForceCount,
            (PanelContent::PartsWithLabels, PanelContent::PartsInLocation) => {
                ActionVariant::ForceCount
            }

            (PanelContent::Parts, _) => ActionVariant::None,
            (PanelContent::Locations, _) => ActionVariant::None,

            (PanelContent::Labels, p) if p.contains_parts() => ActionVariant::RemoveLabelFromPart,
            (PanelContent::Labels, _) => ActionVariant::None,

            (PanelContent::PartsFromSources, PanelContent::Locations) => ActionVariant::DeliverPart,
            (PanelContent::PartsInOrders, PanelContent::Locations) => ActionVariant::DeliverPart,
            (PanelContent::PartsFromSources, PanelContent::PartsInLocation) => {
                ActionVariant::DeliverPart
            }
            (PanelContent::PartsInOrders, PanelContent::PartsInLocation) => {
                ActionVariant::DeliverPart
            }

            (PanelContent::PartsInProjects, PanelContent::Locations) => ActionVariant::UnsolderPart,
            (PanelContent::PartsInProjects, PanelContent::PartsInLocation) => {
                ActionVariant::UnsolderPart
            }

            (PanelContent::Sources, _) => ActionVariant::None,

            (PanelContent::Projects, _) => ActionVariant::None,
            (_, _) => ActionVariant::None,
        }
    }

    fn f8_action(&self) -> ActionVariant {
        if self.get_active_panel_data().data_type().can_delete() {
            ActionVariant::Delete
        } else {
            ActionVariant::None
        }
    }

    pub fn press_enter(&mut self) -> AppEvents {
        match self.view.hot() {
            view::Hot::PanelA => {
                // Replacing a non-copy structure member in a mutable self requires a workaround
                // using the std::memory::replace and a temporary "empty" value
                let old = replace(&mut self.model.panel_a, Box::new(TemporaryEmptyPanel()));
                let next = old.enter(self.view.panel_a.selected, &self.store);
                self.model.panel_a = next.0;
                self.view.panel_a.selected = next.1;
                AppEvents::Redraw
            }
            view::Hot::PanelB => {
                // Replacing a non-copy structure member in a mutable self requires a workaround
                // using the std::memory::replace and a temporary "empty" value
                let old = replace(&mut self.model.panel_b, Box::new(TemporaryEmptyPanel()));
                let next = old.enter(self.view.panel_b.selected, &self.store);
                self.model.panel_b = next.0;
                self.view.panel_b.selected = next.1;
                AppEvents::Redraw
            }
            _ => AppEvents::Redraw,
        }
    }

    pub fn finish_action(&mut self) -> anyhow::Result<AppEvents> {
        match self.view.hot() {
            view::Hot::ActionCountDialog => {
                self.view.hide_action_dialog();

                let source_idx = self.view.get_active_panel_selection();
                let source = self
                    .get_active_panel_data()
                    .actionable_objects(source_idx, &self.store);

                let destination_idx = self.view.get_inactive_panel_selection();
                let destination = self
                    .get_inactive_panel_data()
                    .actionable_objects(destination_idx, &self.store);

                match self.view.action_count_dialog_action {
                    ActionVariant::AddLabelToPart => {
                        self.finish_action_add_label_to_part(&source, &destination)
                    }
                    ActionVariant::RemoveLabelFromPart => {
                        self.finish_action_remove_label_from_part(&source, &destination)
                    }
                    // Use the same as AddLabelToPart, but revese the order of arguments
                    ActionVariant::AddPartToLabel => {
                        self.finish_action_add_label_to_part(&destination, &source)
                    }
                    ActionVariant::RemovePartFromLabel => {
                        // Use the same as RemoveLabelFromPart, but revese the order of arguments
                        self.finish_action_remove_label_from_part(&destination, &source)
                    }
                    ActionVariant::RequirePart => self.finish_action_require(&source, &destination),
                    ActionVariant::OrderPart => self.finish_action_order(&source, &destination),
                    ActionVariant::OrderPartLocal => self.finish_action_order(&source, &source),
                    ActionVariant::MovePart => self.finish_action_move(&source, &destination),
                    ActionVariant::DeliverPart => self.finish_action_deliver(&source, &destination),
                    ActionVariant::SolderPart => self.finish_action_solder(&source, &destination),
                    ActionVariant::UnsolderPart => self.finish_action_unsolder(source, destination),
                    ActionVariant::RequirePartInLocationLocal
                    | ActionVariant::RequirePartInProjectLocal => {
                        self.finish_action_require_local(source.as_ref())
                    }
                    ActionVariant::Error => Err(AppError::BadOperationContext.into()),

                    ActionVariant::ForceCount => {
                        self.finish_action_force_count(source, destination)
                    }
                    ActionVariant::ForceCountLocal => {
                        self.finish_action_force_count_local(source.as_ref())
                    }
                    ActionVariant::ForceCountProject => {
                        self.finish_action_force_count_project(source, destination)
                    }
                    ActionVariant::ForceCountProjectLocal => {
                        self.finish_action_force_count_project_local(source.as_ref())
                    }
                    ActionVariant::SplitLocal => self.finish_action_split_local(source.as_ref()),

                    // These are called in different way, keep the todo here to catch errors
                    ActionVariant::CreatePart => todo!(),
                    ActionVariant::ClonePart => todo!(),
                    ActionVariant::None => todo!(),
                    ActionVariant::Delete => todo!(),
                }
            }
            view::Hot::CreatePartDialog => Ok(AppEvents::Redraw),
            _ => Ok(AppEvents::Redraw),
        }
    }

    pub fn press_f9(&mut self) -> Result<AppEvents, AppError> {
        let action = self.f9_action();
        self.interpret_action(action)
    }

    pub fn press_f5(&mut self) -> Result<AppEvents, AppError> {
        let action = self.f5_action();
        self.interpret_action(action)
    }

    pub fn press_f6(&mut self) -> Result<AppEvents, AppError> {
        let action = self.f6_action();
        self.interpret_action(action)
    }

    pub fn press_ctrl_f9(&mut self) -> Result<AppEvents, AppError> {
        let action = self.ctrl_f9_action();

        if !self
            .get_active_panel_data()
            .item_actionable(self.view.get_active_panel_selection())
        {
            return Ok(AppEvents::Nop);
        }

        self.interpret_action(action)
    }

    pub fn press_ctrl_f6(&mut self) -> Result<AppEvents, AppError> {
        let action = self.ctrl_f6_action();

        if !self
            .get_active_panel_data()
            .item_actionable(self.view.get_active_panel_selection())
        {
            return Ok(AppEvents::Nop);
        }

        self.interpret_action(action)
    }

    fn panel_item_from_id(&self, p_id: &PartId) -> Result<PanelItem, AppError> {
        let obj = self
            .store
            .part_by_id(p_id.part_type())
            .ok_or(AppError::NoSuchObject(p_id.to_string()))?;
        Ok(PanelItem {
            name: obj.metadata.name.clone(),
            subname: p_id.subname(),
            summary: obj.metadata.summary.clone(),
            data: String::with_capacity(0),
            id: Some(PartId::clone(p_id)),
            parent_id: None,
        })
    }

    fn interpret_action(&mut self, action: ActionVariant) -> Result<AppEvents, AppError> {
        // Dual panel actions are ignored when both sides are not visible
        if action.dual_panel() && !self.view.layout.is_dual_panel() {
            return Ok(AppEvents::Nop);
        }

        // Source panel must return something actionable
        let src = self
            .get_active_panel_data()
            .actionable_objects(self.view.get_active_panel_selection(), &self.store)
            .ok_or(AppError::BadOperationContext)?;

        match action {
            ActionVariant::None => return Ok(AppEvents::Nop),
            ActionVariant::Error => return Err(AppError::BadOperationContext),
            ActionVariant::MovePart => {
                let dst = self
                    .get_inactive_panel_data()
                    .actionable_objects(self.view.get_inactive_panel_selection(), &self.store)
                    .and_then(|ad| ad.location().cloned())
                    .ok_or(AppError::BadOperationContext);
                if let Ok(dst) = dst {
                    self.action_dialog_common_move(
                        action,
                        Some(self.panel_item_from_id(&dst)?),
                        src.part().map_or(1, PartId::piece_size),
                    );
                } else {
                    return Err(dst.unwrap_err());
                }
            }
            ActionVariant::AddLabelToPart | ActionVariant::RemoveLabelFromPart => {
                let dst = self
                    .get_inactive_panel_data()
                    .actionable_objects(self.view.get_inactive_panel_selection(), &self.store)
                    .ok_or(AppError::BadOperationContext)?;
                let src = self
                    .get_active_panel_data()
                    .actionable_objects(self.view.get_active_panel_selection(), &self.store)
                    .ok_or(AppError::BadOperationContext)?;

                self.prepare_add_remove_label(src, dst, action)?;
            }
            ActionVariant::AddPartToLabel | ActionVariant::RemovePartFromLabel => {
                let dst = self
                    .get_inactive_panel_data()
                    .actionable_objects(self.view.get_inactive_panel_selection(), &self.store)
                    .ok_or(AppError::BadOperationContext)?;
                let src = self
                    .get_active_panel_data()
                    .actionable_objects(self.view.get_active_panel_selection(), &self.store)
                    .ok_or(AppError::BadOperationContext)?;

                // Use the same as AddLabelToPart, but reverse arguments
                self.prepare_add_remove_label(dst, src, action)?;
            }
            ActionVariant::CreatePart => todo!(),
            ActionVariant::ClonePart => {
                return self.action_clone_part();
            }
            ActionVariant::OrderPart => {
                let dst = self
                    .get_inactive_panel_data()
                    .actionable_objects(self.view.get_inactive_panel_selection(), &self.store)
                    .and_then(|ad| ad.source().map(SourceId::clone))
                    .ok_or(AppError::BadOperationContext);
                if let Ok(dst) = dst {
                    self.action_dialog_common_move(
                        action,
                        Some(self.panel_item_from_id(&dst.into())?),
                        src.part().map_or(1, PartId::piece_size),
                    );
                } else {
                    return Err(dst.unwrap_err());
                }
            }
            ActionVariant::RequirePart => {
                let dst = self
                    .get_inactive_panel_data()
                    .actionable_objects(self.view.get_inactive_panel_selection(), &self.store)
                    .and_then(|ad| ad.location().or_else(|| ad.project()).cloned())
                    .ok_or(AppError::BadOperationContext);
                if let Ok(dst) = dst {
                    self.action_dialog_common_move(
                        action,
                        Some(self.panel_item_from_id(&dst)?),
                        src.part().map_or(1, PartId::piece_size),
                    );
                } else {
                    return Err(dst.unwrap_err());
                }
            }
            ActionVariant::DeliverPart | ActionVariant::UnsolderPart => {
                let dst = self
                    .get_inactive_panel_data()
                    .actionable_objects(self.view.get_inactive_panel_selection(), &self.store)
                    .and_then(|ad| ad.location().cloned())
                    .ok_or(AppError::BadOperationContext);
                if let Ok(dst) = dst {
                    self.action_dialog_common_move(
                        action,
                        Some(self.panel_item_from_id(&dst)?),
                        src.part().map_or(1, PartId::piece_size),
                    );
                } else {
                    return Err(dst.unwrap_err());
                }
            }
            ActionVariant::SolderPart => {
                let dst = self
                    .get_inactive_panel_data()
                    .actionable_objects(self.view.get_inactive_panel_selection(), &self.store)
                    .and_then(|ad| ad.project().cloned())
                    .ok_or(AppError::BadOperationContext);
                if let Ok(dst) = dst {
                    self.action_dialog_common_move(
                        action,
                        Some(self.panel_item_from_id(&dst)?),
                        src.part().map_or(1, PartId::piece_size),
                    );
                } else {
                    return Err(dst.unwrap_err());
                }
            }
            ActionVariant::OrderPartLocal => {
                let dst = self
                    .get_active_panel_data()
                    .actionable_objects(self.view.get_active_panel_selection(), &self.store)
                    .and_then(|ad| ad.source().map(Rc::clone))
                    .ok_or(AppError::BadOperationContext);
                if let Ok(dst) = dst {
                    self.action_dialog_common_move(
                        action,
                        Some(self.panel_item_from_id(&dst.into())?),
                        src.part().map_or(1, PartId::piece_size),
                    );
                } else {
                    return Err(dst.unwrap_err());
                }
            }
            ActionVariant::RequirePartInLocationLocal
            | ActionVariant::RequirePartInProjectLocal => {
                return self.prepare_require_part_local(action);
            }
            ActionVariant::Delete => {
                self.prepare_delete();
            }
            ActionVariant::ForceCount => {
                self.prepare_force_count()?;
            }
            ActionVariant::ForceCountLocal => {
                self.prepare_force_count_local()?;
            }
            ActionVariant::ForceCountProject => {
                self.prepare_force_count_project()?;
            }
            ActionVariant::ForceCountProjectLocal => {
                self.prepare_force_count_project_local()?;
            }
            ActionVariant::SplitLocal => {
                self.prepare_split_local()?;
            }
        };

        // The code above just opens dialogs and does not manipulate data
        // Redraw screen
        Ok(AppEvents::Redraw)
    }

    fn action_dialog_common_move(
        &mut self,
        action: ActionVariant,
        destination: Option<PanelItem>,
        step: usize,
    ) {
        let source = self
            .get_active_panel_data()
            .item(self.view.get_active_panel_selection(), &self.store);
        // Step 1 - move operation can cut pieces
        self.view
            .show_action_dialog(action, Some(source), destination, 0, step);
    }

    pub fn full_reload(&mut self) -> anyhow::Result<()> {
        self.store.load_parts()?;
        self.store.load_events()?;

        self.reload();
        Ok(())
    }

    pub fn reload(&mut self) {
        // Make sure that the selected item is kept selected even though its index might have changed
        self.view.panel_a.selected = caching_panel_data::panel_reload(
            &mut self.model.panel_a,
            self.view.panel_a.selected,
            &self.store,
        );
        self.view.panel_b.selected = caching_panel_data::panel_reload(
            &mut self.model.panel_b,
            self.view.panel_b.selected,
            &self.store,
        );
    }

    fn press_f7(&mut self) -> Result<AppEvents, AppError> {
        if self.get_active_panel_data().data_type().can_make() {
            self.view.create_name.reset();
            self.view.create_summary.reset();
            self.view.create_idx = Default::default();
            self.view.create_hints = vec![];
            self.view.create_dialog = DialogState::Visible;
            self.view.create_save_into = None;
        }
        Ok(AppEvents::Redraw)
    }

    fn press_f2(&mut self) -> Result<AppEvents, AppError> {
        let active = self.get_active_panel_data();
        if active.data_type().can_make() {
            let selection = self.view.get_active_panel_selection();
            let item = active.item(selection, &self.store);
            if item.id.is_none() {
                return Ok(AppEvents::Nop);
            }

            self.view.create_name = Input::new(item.name);
            self.view.create_summary = Input::new(item.summary);
            self.view.create_idx = Default::default();
            self.view.create_dialog = DialogState::Visible;
            self.view.create_save_into = item.id;
            self.update_create_dialog_hints();
        }
        Ok(AppEvents::Redraw)
    }

    fn update_create_dialog_hints(&mut self) {
        // Do not show hints during part edit
        if self.view.create_save_into.is_some() {
            return;
        }

        let query = self.view.create_name.value().trim().to_lowercase();
        if query.is_empty() {
            self.view.create_hints = vec![];
            return;
        }

        // Special case for labels
        if self.get_active_panel_data().data_type() == PanelContent::LabelKeys {
            self.view.create_hints = self
                .store
                .all_label_keys()
                .iter()
                .filter(|(k, _)| k.to_lowercase().starts_with(&query))
                .map(|(k, _)| PanelItem::new(k, None, "", "", Some(&k.into()), None))
                .collect();
            return;
        }

        if self.get_active_panel_data().data_type() == PanelContent::Labels {
            if let Some(ad) = self
                .get_active_panel_data()
                .actionable_objects(self.view.get_active_panel_selection(), &self.store)
                && let Some(label_key) = ad.label_key()
            {
                self.view.create_hints = self
                    .store
                    .all_label_values(label_key)
                    .iter()
                    .filter(|(v, _)| v.to_lowercase().starts_with(&query))
                    .map(|(v, _)| PanelItem::new(v, None, "", "", Some(&v.into()), None))
                    .collect();
                return;
            }
            self.view.create_hints = vec![];
            return;
        }

        // Parts
        self.view.create_hints = self
            .store
            .all_objects()
            .iter()
            .filter(|(_, p)| match self.get_active_panel_data().data_type() {
                PanelContent::Parts
                | PanelContent::PartsFromSources
                | PanelContent::PartsInLocation
                | PanelContent::PartsInOrders
                | PanelContent::PartsInProjects
                | PanelContent::PartsWithLabels => {
                    p.metadata.types.contains(&crate::store::ObjectType::Part)
                }
                PanelContent::Locations | PanelContent::LocationOfParts => p
                    .metadata
                    .types
                    .contains(&crate::store::ObjectType::Location),
                PanelContent::Sources => {
                    p.metadata.types.contains(&crate::store::ObjectType::Source)
                }
                PanelContent::Projects => p
                    .metadata
                    .types
                    .contains(&crate::store::ObjectType::Project),
                PanelContent::None
                | PanelContent::TypeSelection
                | PanelContent::LabelKeys
                | PanelContent::Labels => false,
            })
            .filter(|(_, p)| p.metadata.name.to_lowercase().starts_with(&query))
            .map(|(_, p)| {
                PanelItem::new(
                    &p.metadata.name,
                    None,
                    &p.metadata.summary,
                    "",
                    // Provide ID with pieces type as a hint to the renderer and cache when needed
                    Some(
                        &Into::<PartId>::into(p.id.as_ref())
                            .conditional_piece(p.metadata.track == CountTracking::Pieces, 0),
                    ),
                    None,
                )
            })
            .take(20)
            .collect();
    }

    fn press_f8(&mut self) -> Result<AppEvents, AppError> {
        let action = self.f8_action();
        self.interpret_action(action)
    }

    pub fn select_item(&mut self, name: &str) {
        if let Some(idx) = self.get_active_panel_data().item_idx(name, &self.store) {
            let idx = idx.min(self.get_active_panel_data().len(&self.store));
            let len = self
                .get_active_panel_data()
                .len(&self.store)
                .saturating_sub(1);
            self.view.update_active_panel(|s| s.selected = idx.min(len));
        }
    }

    pub fn select_item_by_display_id(&mut self, display_id: PanelItemDisplayId, name: &str) {
        let len = self
            .get_active_panel_data()
            .len(&self.store)
            .saturating_sub(1);

        if let Some(idx) = self
            .get_active_panel_data()
            .item_idx_by_display_id(display_id, &self.store)
        {
            let idx = idx.min(self.get_active_panel_data().len(&self.store));
            self.view.update_active_panel(|s| s.selected = idx.min(len));
            return;
        }

        if let Some(idx) = self.get_active_panel_data().item_idx(name, &self.store) {
            let idx = idx.min(self.get_active_panel_data().len(&self.store));
            self.view.update_active_panel(|s| s.selected = idx.min(len));
        }
    }

    pub fn select_item_by_part_id(&mut self, part_id: &PartId, name: &str) {
        let len = self
            .get_active_panel_data()
            .len(&self.store)
            .saturating_sub(1);

        if let Some(idx) = self
            .get_active_panel_data()
            .item_idx_by_part_id(part_id, &self.store)
        {
            let idx = idx.min(self.get_active_panel_data().len(&self.store));
            self.view.update_active_panel(|s| s.selected = idx.min(len));
            return;
        }

        if let Some(idx) = self.get_active_panel_data().item_idx(name, &self.store) {
            let idx = idx.min(self.get_active_panel_data().len(&self.store));
            self.view.update_active_panel(|s| s.selected = idx.min(len));
        }
    }

    fn make_new_type_id(&self, name: &str) -> PartTypeId {
        let mut candidate = self.store.name_to_id(name).into();
        loop {
            if let Some(_part) = self.store.part_by_id(&candidate) {
                // conflict! generate new id
                if let Some((prefix, suffix)) = candidate.rsplit_once("--") {
                    if let Some(suffix_no) = num::BigUint::parse_bytes(suffix.as_bytes(), 36) {
                        let next_suffix = suffix_no + 1_u32;
                        candidate =
                            Rc::from([prefix, next_suffix.to_str_radix(36).as_str()].join("--"));
                    } else {
                        candidate = Rc::from([prefix, "1"].join("--"));
                    }
                } else {
                    candidate = Rc::from([&candidate, "1"].join("--"));
                }
            } else {
                debug!("Allocated new ID {:?}", candidate);
                return candidate;
            }
        }
    }

    pub fn update_status(&mut self, msg: &str) {
        info!("status: {}", msg);
        self.view.status = msg.to_owned();
    }

    fn press_f4(&self) -> Result<AppEvents, AppError> {
        let item = self
            .get_active_panel_data()
            .item(self.view.get_active_panel_selection(), &self.store);
        let part_id = item.id.ok_or(AppError::PartHasNoId)?;
        Ok(AppEvents::Edit(part_id))
    }

    pub fn get_part(&self, p_id: &PartTypeId) -> Option<&Part> {
        self.store.part_by_id(p_id)
    }

    pub fn reload_part(&mut self, part: &Part) {
        self.store.insert_part_to_cache(part.clone());
    }

    pub fn show_alert(&mut self, title: &str, alert: &str) {
        self.view.alert_dialog = DialogState::Visible;
        self.view.alert_title = title.to_string();
        self.view.alert_text = alert.to_string();
        error!("{}: {}", title, alert);
    }

    fn open_filter_dialog(&mut self) {
        match self.get_active_panel_data().filter_status() {
            model::FilterStatus::NotSupported => (),
            model::FilterStatus::NotApplied => {
                self.view.filter_query.reset();
                self.view.filter_dialog = DialogState::Visible;
            }
            model::FilterStatus::Query(q) => {
                self.view.filter_query = Input::new(q);
                self.view.filter_dialog = DialogState::Visible;
            }
        }
    }

    fn perform_filter(&mut self) -> AppEvents {
        let query = Query::new(self.view.filter_query.value());
        if let Err(_e) = query {
            // TODO handle errors once the parsing gets complex
            return AppEvents::Redraw;
        }
        let query = query.unwrap();

        let selected = self.view.filter_selected.unwrap_or(
            self.get_active_panel_data()
                .item(self.view.get_active_panel_selection(), &self.store)
                .display_id(),
        );

        match self.view.active {
            ActivePanel::PanelA => {
                // Replacing a non-copy structure member in a mutable self requires a workaround
                // using the std::memory::replace and a temporary "empty" value
                let old = replace(&mut self.model.panel_a, Box::new(TemporaryEmptyPanel()));
                match old.filter(query, &self.store) {
                    Ok(next) => {
                        self.model.panel_a = next.0;
                        self.view.panel_a.selected = next.1;
                        self.view.filter_dialog = DialogState::Hidden;
                    }
                    Err(e) => {
                        self.model.panel_a = e.return_to().0;
                        // TODO report error back
                    }
                }
                let name = self
                    .model
                    .panel_a
                    .item(self.view.panel_a.selected, &self.store)
                    .name
                    .clone();
                AppEvents::SelectByDisplayId(selected, name)
            }
            ActivePanel::PanelB => {
                // Replacing a non-copy structure member in a mutable self requires a workaround
                // using the std::memory::replace and a temporary "empty" value
                let old = replace(&mut self.model.panel_b, Box::new(TemporaryEmptyPanel()));
                match old.filter(query, &self.store) {
                    Ok(next) => {
                        self.model.panel_b = next.0;
                        self.view.panel_b.selected = next.1;
                        self.view.filter_dialog = DialogState::Hidden;
                    }
                    Err(e) => {
                        self.model.panel_b = e.return_to().0;
                        // TODO report error back
                    }
                }
                let name = self
                    .model
                    .panel_b
                    .item(self.view.panel_b.selected, &self.store)
                    .name
                    .clone();
                AppEvents::SelectByDisplayId(selected, name)
            }
        }
    }
}

// This is a NO-OP panel data structure that is used ONLY INTERNALLY
// during the switch from one panel to the next.
#[derive(Debug)]
struct TemporaryEmptyPanel();
impl PanelData for TemporaryEmptyPanel {
    fn title(&self, _store: &Store) -> String {
        todo!()
    }

    fn panel_title(&self, _store: &Store) -> String {
        todo!()
    }

    fn data_type(&self) -> PanelContent {
        todo!()
    }

    fn enter(self: Box<Self>, _idx: usize, _store: &Store) -> model::EnterAction {
        todo!()
    }

    fn reload(&mut self, _store: &Store) {
        todo!()
    }

    fn item_actionable(&self, _idx: usize) -> bool {
        todo!()
    }

    fn item_summary(&self, _idx: usize, _store: &Store) -> String {
        todo!()
    }

    fn len(&self, _store: &Store) -> usize {
        todo!()
    }

    fn items(&self, _store: &Store) -> Vec<PanelItem> {
        todo!()
    }

    fn actionable_objects(&self, _idx: usize, _store: &Store) -> Option<ActionDescriptor> {
        todo!()
    }

    fn item_idx(&self, _id: &str, _store: &Store) -> Option<usize> {
        todo!()
    }

    fn item(&self, _idx: usize, _store: &Store) -> PanelItem {
        todo!()
    }

    fn filter(
        self: Box<Self>,
        _query: Query,
        _store: &Store,
    ) -> Result<EnterAction, model::FilterError> {
        todo!()
    }
}
