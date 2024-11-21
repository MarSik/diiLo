use std::{mem::replace, rc::Rc};

use chrono::Local;
use errs::AppError;
use log::{debug, error, info};
use model::{ActionDescriptor, Model, PanelContent, PanelData, PanelItem};
use tui_input::Input;
use view::{CreateMode, DialogState, View};

use crate::store::{LedgerEntry, LedgerEvent, Part, PartId, PartMetadata, Store};

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
    // Reload data model
    ReloadData,
    // Reload data model and then select item on active panel
    ReloadDataSelect(String),
    // Select
    Select(String),
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
            AppEvents::ReloadData | AppEvents::ReloadDataSelect(_) => {
                AppEvents::ReloadDataSelect(name.to_string())
            }
            _ => AppEvents::Select(name.to_string()),
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum ActionVariant {
    #[default]
    None,
    Error,
    AddLabel,
    RemoveLabel,
    CreatePart,
    ClonePart,
    RequirePart,
    OrderPart,
    MovePart,
    DeliverPart,
    SolderPart,
    UnsolderPart,
    OrderPartLocal,
    RequirePartLocal,
    Delete,
}

impl ActionVariant {
    pub fn name(self) -> &'static str {
        match self {
            ActionVariant::None => "",
            ActionVariant::Error => "",
            ActionVariant::AddLabel => "label",
            ActionVariant::RemoveLabel => "unlabel",
            ActionVariant::CreatePart => "make",
            ActionVariant::ClonePart => "clone",
            ActionVariant::RequirePart => "require",
            ActionVariant::OrderPart => "order",
            ActionVariant::MovePart => "move",
            ActionVariant::DeliverPart => "deliver",
            ActionVariant::SolderPart => "solder",
            ActionVariant::UnsolderPart => "unsolder",
            ActionVariant::OrderPartLocal => "order",
            ActionVariant::RequirePartLocal => "require",
            ActionVariant::Delete => "delete",
        }
    }

    pub fn dual_panel(self) -> bool {
        match self {
            ActionVariant::OrderPartLocal => false,
            ActionVariant::RequirePartLocal => false,
            ActionVariant::Delete => false,
            ActionVariant::ClonePart => false,
            ActionVariant::CreatePart => false,
            _ => true,
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            ActionVariant::None => "",
            ActionVariant::Error => "",
            ActionVariant::AddLabel => "Add label",
            ActionVariant::RemoveLabel => "Remove label",
            ActionVariant::CreatePart => "Create new part",
            ActionVariant::ClonePart => "Clone part",
            ActionVariant::RequirePart => "Request part",
            ActionVariant::OrderPart => "Order part",
            ActionVariant::MovePart => "Move part",
            ActionVariant::DeliverPart => "Deliver part",
            ActionVariant::SolderPart => "Solder part",
            ActionVariant::UnsolderPart => "Unsolder part",
            ActionVariant::OrderPartLocal => "Order part",
            ActionVariant::RequirePartLocal => "Require part",
            ActionVariant::Delete => "Delete part",
        }
    }

    pub fn countable(self) -> bool {
        match self {
            ActionVariant::None => false,
            ActionVariant::Error => false,
            ActionVariant::AddLabel => false,
            ActionVariant::RemoveLabel => false,
            ActionVariant::CreatePart => false,
            ActionVariant::ClonePart => false,
            ActionVariant::RequirePart => true,
            ActionVariant::OrderPart => true,
            ActionVariant::MovePart => true,
            ActionVariant::DeliverPart => true,
            ActionVariant::SolderPart => true,
            ActionVariant::UnsolderPart => true,
            ActionVariant::OrderPartLocal => true,
            ActionVariant::RequirePartLocal => true,
            ActionVariant::Delete => false,
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
            (PanelContent::PartsInLocation, _) => ActionVariant::RequirePartLocal,
            (PanelContent::LocationOfParts, _) => ActionVariant::RequirePartLocal,
            (PanelContent::PartsInProjects, _) => ActionVariant::RequirePartLocal,
            (_, _) => ActionVariant::None,
        }
    }

    pub fn f5_action(&self) -> ActionVariant {
        match self.get_action_direction() {
            (PanelContent::TypeSelection, _) => ActionVariant::None,
            (_, PanelContent::TypeSelection) => ActionVariant::None,

            (p, PanelContent::Locations) if p.contains_parts() => ActionVariant::RequirePart,
            (p, PanelContent::PartsInLocation) if p.contains_parts() => ActionVariant::RequirePart,

            (p, PanelContent::Labels) if p.contains_parts() => ActionVariant::AddLabel,
            (p, PanelContent::PartsWithLabels) if p.contains_parts() => ActionVariant::AddLabel,

            (p, PanelContent::Sources) if p.contains_parts() => ActionVariant::OrderPart,
            (p, PanelContent::PartsInOrders) if p.contains_parts() => ActionVariant::OrderPart,
            (p, PanelContent::PartsFromSources) if p.contains_parts() => ActionVariant::OrderPart,

            (p, PanelContent::PartsInProjects) if p.contains_parts() => ActionVariant::RequirePart,
            (p, PanelContent::Projects) if p.contains_parts() => ActionVariant::RequirePart,

            (PanelContent::Parts, _) => ActionVariant::ClonePart,
            (PanelContent::Projects, _) => ActionVariant::ClonePart,

            (PanelContent::Locations, _) => ActionVariant::None,

            (PanelContent::Labels, p) if p.contains_parts() => ActionVariant::AddLabel,
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
            (PanelContent::PartsInLocation, PanelContent::PartsInLocation) => {
                ActionVariant::MovePart
            }

            (p, PanelContent::Labels) if p.contains_parts() => ActionVariant::RemoveLabel,
            (p, PanelContent::PartsWithLabels) if p.contains_parts() => ActionVariant::RemoveLabel,

            (PanelContent::PartsInLocation, PanelContent::Projects) => ActionVariant::SolderPart,
            (PanelContent::PartsInLocation, PanelContent::PartsInProjects) => {
                ActionVariant::SolderPart
            }

            (PanelContent::Parts, _) => ActionVariant::None,
            (PanelContent::Locations, _) => ActionVariant::None,

            (PanelContent::Labels, p) if p.contains_parts() => ActionVariant::RemoveLabel,
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
                    ActionVariant::AddLabel => {
                        return self.finish_action_add_label(&source, &destination);
                    }
                    ActionVariant::RemoveLabel => {
                        return self.finish_action_remove_label(&source, &destination);
                    }
                    ActionVariant::RequirePart => {
                        return self.finish_action_require(&source, &destination);
                    }
                    ActionVariant::OrderPart => {
                        return self.finish_action_order(&source, &destination);
                    }
                    ActionVariant::OrderPartLocal => {
                        return self.finish_action_order(&source, &source);
                    }
                    ActionVariant::MovePart => {
                        return self.finish_action_move(&source, &destination);
                    }
                    ActionVariant::DeliverPart => {
                        return self.finish_action_deliver(&source, &destination);
                    }
                    ActionVariant::SolderPart => {
                        return self.finish_action_solder(&source, &destination);
                    }
                    ActionVariant::UnsolderPart => {
                        return self.finish_action_unsolder(source, destination);
                    }
                    ActionVariant::RequirePartLocal => {
                        return self.finish_action_require_local(source.as_ref());
                    }
                    ActionVariant::Error => Err(AppError::BadOperationContext.into()),

                    // These two are called in different way, keep the todo here to catch errors
                    ActionVariant::CreatePart => todo!(),
                    ActionVariant::ClonePart => todo!(),
                    _ => Ok(AppEvents::Redraw),
                }
            }
            view::Hot::CreatePartDialog => Ok(AppEvents::Redraw),
            _ => Ok(AppEvents::Redraw),
        }
    }

    fn finish_action_add_label(
        &mut self,
        source: &Option<ActionDescriptor>,
        destination: &Option<ActionDescriptor>,
    ) -> anyhow::Result<AppEvents> {
        if let Some(source) = source
            .as_ref()
            .and_then(|s| s.label().map(|(k, v)| (k.clone(), v.clone())))
        {
            let destination = destination
                .as_ref()
                .and_then(|d| d.part().map(Rc::clone))
                .ok_or(AppError::BadOperationContext)?;
            return self.perform_add_label(&destination, source);
        } else if let Some(destination) = destination
            .as_ref()
            .and_then(|s| s.label().map(|(k, v)| (k.clone(), v.clone())))
        {
            let source = source
                .as_ref()
                .and_then(|d| d.part().map(Rc::clone))
                .ok_or(AppError::BadOperationContext)?;
            return self.perform_add_label(&source, destination);
        }
        Ok(AppEvents::Nop)
    }

    fn finish_action_remove_label(
        &mut self,
        source: &Option<ActionDescriptor>,
        destination: &Option<ActionDescriptor>,
    ) -> anyhow::Result<AppEvents> {
        if let Some(source) = source
            .as_ref()
            .and_then(|s| s.label().map(|(k, v)| (k.clone(), v.clone())))
        {
            let destination = destination
                .as_ref()
                .and_then(|d| d.part().map(Rc::clone))
                .ok_or(AppError::BadOperationContext)?;
            return self
                .perform_remove_label(&destination, source)
                .or(Ok(AppEvents::Redraw));
        } else if let Some(destination) = destination
            .as_ref()
            .and_then(|s| s.label().map(|(k, v)| (k.clone(), v.clone())))
        {
            let source = source
                .as_ref()
                .and_then(|d| d.part().map(Rc::clone))
                .ok_or(AppError::BadOperationContext)?;
            return self
                .perform_remove_label(&source, destination)
                .or(Ok(AppEvents::Redraw));
        }
        Ok(AppEvents::Nop)
    }

    fn finish_action_require(
        &mut self,
        source: &Option<ActionDescriptor>,
        destination: &Option<ActionDescriptor>,
    ) -> anyhow::Result<AppEvents> {
        let part = source
            .as_ref()
            .and_then(|s| s.part().map(Rc::clone))
            .ok_or(AppError::BadOperationContext)?;
        let (destination, ev) = if let Some(destination) = destination
            .as_ref()
            .and_then(|d| d.location().map(Rc::clone))
        {
            (Rc::clone(&destination), LedgerEvent::RequireIn(destination))
        } else if let Some(project_id) = destination
            .as_ref()
            .and_then(|d| d.project().map(Rc::clone))
        {
            (
                Rc::clone(&project_id),
                LedgerEvent::RequireInProject(project_id),
            )
        } else {
            self.update_status("Invalid requirement?!");
            return Ok(AppEvents::Redraw);
        };

        self.update_status(&format!(
            "{} parts {} needed in {}",
            self.view.action_count_dialog_count, &part, &destination
        ));

        let event_to = LedgerEntry {
            t: Local::now().fixed_offset(),
            count: self.view.action_count_dialog_count,
            part,
            ev: ev,
        };

        self.store.record_event(&event_to)?;
        self.store.update_count_cache(&event_to);

        Ok(AppEvents::ReloadData)
    }

    fn finish_action_order(
        &mut self,
        source: &Option<ActionDescriptor>,
        destination: &Option<ActionDescriptor>,
    ) -> anyhow::Result<AppEvents> {
        let part = source
            .as_ref()
            .and_then(|s| s.part().map(Rc::clone))
            .ok_or(AppError::BadOperationContext)?;
        let destination = destination
            .as_ref()
            .and_then(|d| d.source().map(Rc::clone))
            .ok_or(AppError::BadOperationContext)?;

        self.update_status(&format!(
            "{} parts {} ordered from {}",
            self.view.action_count_dialog_count, &part, &destination
        ));

        let event_to = LedgerEntry {
            t: Local::now().fixed_offset(),
            count: self.view.action_count_dialog_count,
            part,
            ev: LedgerEvent::OrderFrom(destination),
        };

        self.store.record_event(&event_to)?;
        self.store.update_count_cache(&event_to);

        Ok(AppEvents::ReloadData)
    }

    fn finish_action_move(
        &mut self,
        source: &Option<ActionDescriptor>,
        destination: &Option<ActionDescriptor>,
    ) -> anyhow::Result<AppEvents> {
        let part = source
            .as_ref()
            .and_then(|s| s.part().map(Rc::clone))
            .ok_or(AppError::BadOperationContext)?;
        let source = source
            .as_ref()
            .and_then(|d| d.location().map(Rc::clone))
            .ok_or(AppError::BadOperationContext)?;
        let destination = destination
            .as_ref()
            .and_then(|d| d.location().map(Rc::clone))
            .ok_or(AppError::BadOperationContext)?;

        self.update_status(&format!(
            "{} parts {} moved from {} to {}",
            self.view.action_count_dialog_count, &part, &source, &destination
        ));

        let event_from = LedgerEntry {
            t: Local::now().fixed_offset(),
            count: self.view.action_count_dialog_count,
            part: part.clone(),
            ev: LedgerEvent::TakeFrom(source),
        };
        let event_to = LedgerEntry {
            t: Local::now().fixed_offset(),
            count: self.view.action_count_dialog_count,
            part,
            ev: LedgerEvent::StoreTo(destination),
        };

        self.store.record_event(&event_from)?;
        self.store.record_event(&event_to)?;

        self.store.update_count_cache(&event_from);
        self.store.update_count_cache(&event_to);

        Ok(AppEvents::ReloadData)
    }

    fn finish_action_deliver(
        &mut self,
        source: &Option<ActionDescriptor>,
        destination: &Option<ActionDescriptor>,
    ) -> anyhow::Result<AppEvents> {
        let part = source
            .as_ref()
            .and_then(|s| s.part().map(Rc::clone))
            .ok_or(AppError::BadOperationContext)?;
        let source = source
            .as_ref()
            .and_then(|d| d.source().map(Rc::clone))
            .ok_or(AppError::BadOperationContext)?;
        let destination = destination
            .as_ref()
            .and_then(|d| d.location().map(Rc::clone))
            .ok_or(AppError::BadOperationContext)?;

        self.update_status(&format!(
            "{} parts {} delivered from {} to {}",
            self.view.action_count_dialog_count, &part, &source, &destination
        ));

        let event_from = LedgerEntry {
            t: Local::now().fixed_offset(),
            count: self.view.action_count_dialog_count,
            part: part.clone(),
            ev: LedgerEvent::DeliverFrom(source),
        };
        let event_to = LedgerEntry {
            t: Local::now().fixed_offset(),
            count: self.view.action_count_dialog_count,
            part,
            ev: LedgerEvent::StoreTo(destination),
        };

        self.store.record_event(&event_from)?;
        self.store.record_event(&event_to)?;

        self.store.update_count_cache(&event_from);
        self.store.update_count_cache(&event_to);

        Ok(AppEvents::ReloadData)
    }

    fn finish_action_solder(
        &mut self,
        source: &Option<ActionDescriptor>,
        destination: &Option<ActionDescriptor>,
    ) -> anyhow::Result<AppEvents> {
        let part = source
            .as_ref()
            .and_then(|s| s.part().map(Rc::clone))
            .ok_or(AppError::BadOperationContext)?;
        let source = source
            .as_ref()
            .and_then(|d| d.location().map(Rc::clone))
            .ok_or(AppError::BadOperationContext)?;
        let destination = destination
            .as_ref()
            .and_then(|d| d.project().map(Rc::clone))
            .ok_or(AppError::BadOperationContext)?;

        self.update_status(&format!(
            "{} parts {} soldered from {} to {}",
            self.view.action_count_dialog_count, &part, &source, &destination
        ));

        let event_from = LedgerEntry {
            t: Local::now().fixed_offset(),
            count: self.view.action_count_dialog_count,
            part: part.clone(),
            ev: LedgerEvent::TakeFrom(source),
        };
        let event_to = LedgerEntry {
            t: Local::now().fixed_offset(),
            count: self.view.action_count_dialog_count,
            part,
            ev: LedgerEvent::SolderTo(destination),
        };

        self.store.record_event(&event_from)?;
        self.store.record_event(&event_to)?;

        self.store.update_count_cache(&event_from);
        self.store.update_count_cache(&event_to);

        Ok(AppEvents::ReloadData)
    }

    fn finish_action_unsolder(
        &mut self,
        source: Option<ActionDescriptor>,
        destination: Option<ActionDescriptor>,
    ) -> anyhow::Result<AppEvents> {
        let part = source
            .as_ref()
            .and_then(|s| s.part().map(Rc::clone))
            .ok_or(AppError::BadOperationContext)?;
        let source = source
            .and_then(|d| d.project().map(Rc::clone))
            .ok_or(AppError::BadOperationContext)?;
        let destination = destination
            .and_then(|d| d.location().map(Rc::clone))
            .ok_or(AppError::BadOperationContext)?;

        self.update_status(&format!(
            "{} parts {} unsoldered from {} to {}",
            self.view.action_count_dialog_count, &part, &source, &destination
        ));

        let event_from = LedgerEntry {
            t: Local::now().fixed_offset(),
            count: self.view.action_count_dialog_count,
            part: part.clone(),
            ev: LedgerEvent::UnsolderFrom(source),
        };

        let event_to = LedgerEntry {
            t: Local::now().fixed_offset(),
            count: self.view.action_count_dialog_count,
            part,
            ev: LedgerEvent::StoreTo(destination),
        };

        self.store.record_event(&event_from)?;
        self.store.record_event(&event_to)?;

        self.store.update_count_cache(&event_from);
        self.store.update_count_cache(&event_to);

        return Ok(AppEvents::ReloadData);
    }

    fn perform_add_label(
        &mut self,
        part_id: &PartId,
        label: (String, String),
    ) -> anyhow::Result<AppEvents> {
        if let Some(part) = self.store.part_by_id(part_id) {
            let mut new_part = part.clone();
            new_part
                .metadata
                .labels
                .insert(label.0.clone(), label.1.clone());
            self.update_status(&format!(
                "Label {}: {} added to {}",
                label.0.as_str(),
                label.1.as_str(),
                part.metadata.name
            ));
            self.store.store_part(&mut new_part)?;
            self.store.insert_part_to_cache(new_part);
            return Ok(AppEvents::ReloadData);
        }

        Ok(AppEvents::Nop)
    }

    fn perform_remove_label(
        &mut self,
        part_id: &PartId,
        label: (String, String),
    ) -> anyhow::Result<AppEvents> {
        let part = self
            .store
            .part_by_id(part_id)
            .ok_or(AppError::NoSuchObject(part_id.to_string()))?;
        let mut new_part = part.clone();
        let labels = new_part.metadata.labels.remove(&label.0);
        if let Some(vals) = labels {
            for val in vals {
                if val != label.1 {
                    new_part.metadata.labels.insert(label.0.clone(), val);
                }
            }
        }
        self.update_status(&format!(
            "Label {}: {} removed from {}",
            label.0.as_str(),
            label.1.as_str(),
            part.metadata.name
        ));
        self.store.store_part(&mut new_part)?;
        self.store.insert_part_to_cache(new_part);
        Ok(AppEvents::ReloadData)
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

    fn interpret_action(&mut self, action: ActionVariant) -> Result<AppEvents, AppError> {
        // Dual panel actions are ignored when both sides are not visible
        if action.dual_panel() && !self.view.layout.is_dual_panel() {
            return Ok(AppEvents::Nop);
        }

        match action {
            ActionVariant::None => return Ok(AppEvents::Nop),
            ActionVariant::Error => return Err(AppError::BadOperationContext),
            ActionVariant::MovePart => {
                self.action_dialog_common_move(action);
            }
            ActionVariant::AddLabel => {
                self.action_dialog_common_move(action);
            }
            ActionVariant::RemoveLabel => {
                self.action_dialog_common_move(action);
            }
            ActionVariant::CreatePart => todo!(),
            ActionVariant::ClonePart => {
                return self.action_clone_part();
            }
            ActionVariant::RequirePart => {
                self.action_dialog_common_move(action);
            }
            ActionVariant::OrderPart => {
                self.action_dialog_common_move(action);
            }
            ActionVariant::DeliverPart => {
                self.action_dialog_common_move(action);
            }
            ActionVariant::SolderPart => {
                self.action_dialog_common_move(action);
            }
            ActionVariant::UnsolderPart => {
                self.action_dialog_common_move(action);
            }
            ActionVariant::OrderPartLocal => {
                self.action_dialog_common_move(action);
            }
            ActionVariant::RequirePartLocal => {
                self.prepare_require_part_local(action);
            }
            ActionVariant::Delete => {
                self.prepare_delete();
            }
        };

        // The code above just opens dialogs and does not manipulate data
        // Redraw screen
        Ok(AppEvents::Redraw)
    }

    fn prepare_require_part_local(&mut self, action: ActionVariant) -> Option<AppEvents> {
        let ad = self
            .get_active_panel_data()
            .actionable_objects(self.view.get_active_panel_selection(), &self.store)?;

        let part_id = ad.part()?;
        let count = if let Some(location_id) = ad.location() {
            self.store.get_by_location(part_id, location_id).required()
        } else if let Some(project_id) = ad.project() {
            self.store.get_by_project(part_id, project_id).required()
        } else if let Some(source_id) = ad.source() {
            self.store.get_by_source(part_id, source_id).required()
        } else {
            return None;
        };

        // show currently required count as prefilled value
        self.view.show_action_dialog(action, count);
        Some(AppEvents::Redraw)
    }

    fn action_dialog_common_move(&mut self, action: ActionVariant) {
        self.view.show_action_dialog(action, 0);
    }

    pub fn full_reload(&mut self) -> anyhow::Result<()> {
        self.store.scan_parts()?;
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
            self.view.create_dialog = DialogState::VISIBLE;
            self.view.create_save_into = None;
        }
        Ok(AppEvents::Redraw)
    }

    fn press_f2(&mut self) -> Result<AppEvents, AppError> {
        let active = self.get_active_panel_data();
        if active.data_type().can_make() {
            let selection = self.view.get_active_panel_selection();
            let item = active.item(selection, &self.store);

            self.view.create_name = Input::new(item.name);
            self.view.create_summary = Input::new(item.summary);
            self.view.create_idx = Default::default();
            self.view.create_dialog = DialogState::VISIBLE;
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
                .map(|(k, _)| PanelItem::new(k, "", "", Some(&Rc::from(k.as_str()))))
                .collect();
            return;
        }

        if self.get_active_panel_data().data_type() == PanelContent::Labels {
            if let Some(ad) = self
                .get_active_panel_data()
                .actionable_objects(self.view.get_active_panel_selection(), &self.store)
            {
                if let Some(label_key) = ad.label_key() {
                    self.view.create_hints = self
                        .store
                        .all_label_values(&label_key)
                        .iter()
                        .filter(|(v, _)| v.to_lowercase().starts_with(&query))
                        .map(|(v, _)| PanelItem::new(v, "", "", Some(&Rc::from(v.as_str()))))
                        .collect();
                    return;
                }
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
            .map(|(_, p)| PanelItem::new(&p.metadata.name, &p.metadata.summary, "", Some(&p.id)))
            .take(20)
            .collect();
    }

    fn press_f8(&mut self) -> Result<AppEvents, AppError> {
        let action = self.f8_action();
        self.interpret_action(action)
    }

    pub fn select_item(&mut self, name: &str) {
        if let Some(idx) = self
            .get_active_panel_data()
            .item_idx(&name.to_string(), &self.store)
        {
            let idx = idx.min(self.get_active_panel_data().len(&self.store));
            let len = self
                .get_active_panel_data()
                .len(&self.store)
                .saturating_sub(1);
            self.view.update_active_panel(|s| s.selected = idx.min(len));
        }
    }

    fn finish_create(&mut self) -> anyhow::Result<AppEvents> {
        self.view.hide_create_dialog();

        // This was an edit of existing part, just update it and return
        if let Some(part_id) = self.view.create_save_into.clone() {
            if let Some(part) = self.store.part_by_id(&part_id) {
                let mut new_part = part.clone();
                new_part.metadata.name = self.view.create_name.value().to_string();
                new_part.metadata.summary = self.view.create_summary.value().to_string();
                self.store.store_part(&mut new_part)?;
                self.store.insert_part_to_cache(new_part);
                return Ok(AppEvents::ReloadData);
            }

            return Ok(AppEvents::ReloadData);
        }

        match self.get_active_panel_data().data_type() {
            PanelContent::None => return Ok(AppEvents::Redraw),
            PanelContent::TypeSelection => todo!(),
            PanelContent::Parts => {
                return self.finish_create_part();
            }
            PanelContent::Locations => {
                return self.finish_create_location();
            }
            PanelContent::LocationOfParts => {
                return self.finish_create_location_for_part();
            }
            PanelContent::PartsInLocation => {
                return self.finish_create_part_in_location();
            }
            PanelContent::LabelKeys => {
                return self.finish_create_label_key();
            }
            PanelContent::Labels => {
                return self.finish_create_label();
            }
            PanelContent::PartsWithLabels => {
                return self.finish_create_part_w_label();
            }
            PanelContent::Sources => {
                return self.finish_create_source();
            }
            PanelContent::PartsFromSources => {
                return self.finish_create_part_in_source();
            }
            PanelContent::PartsInOrders => {
                return self.finish_create_part_in_source();
            }
            PanelContent::Projects => {
                return self.finish_create_project();
            }
            PanelContent::PartsInProjects => {
                return self.finish_create_part_in_project();
            }
        }
    }

    fn finish_create_part(&mut self) -> anyhow::Result<AppEvents> {
        if let CreateMode::HINT(hint) = self.view.create_idx {
            Ok(AppEvents::Select(self.view.create_hints[hint].name.clone()))
        } else {
            // Enter on summary or name fields
            let part_id = self.create_object_from_dialog_data(|part| {
                part.metadata.types.insert(crate::store::ObjectType::Part);
            })?;

            self.update_status(&format!("Part {} was created.", part_id));
            return Ok(AppEvents::ReloadDataSelect(
                self.store
                    .part_by_id(&part_id)
                    .map_or(part_id.to_string(), |p| p.metadata.name.clone()),
            ));
        }
    }

    fn finish_create_location(&mut self) -> anyhow::Result<AppEvents> {
        if let CreateMode::HINT(hint) = self.view.create_idx {
            Ok(AppEvents::Select(self.view.create_hints[hint].name.clone()))
        } else {
            // Enter on summary or name fields
            let part_id = self.create_object_from_dialog_data(|part| {
                part.metadata
                    .types
                    .insert(crate::store::ObjectType::Location);
            })?;

            self.update_status(&format!("Location {} was created.", part_id));
            return Ok(AppEvents::ReloadDataSelect(
                self.store
                    .part_by_id(&part_id)
                    .map_or(part_id.to_string(), |p| p.metadata.name.clone()),
            ));
        }
    }

    fn finish_create_part_in_location(&mut self) -> anyhow::Result<AppEvents> {
        let action_desc = self
            .get_active_panel_data()
            .actionable_objects(self.view.get_active_panel_selection(), &self.store)
            .ok_or(AppError::BadOperationContext)?;

        if let CreateMode::HINT(hint) = self.view.create_idx {
            if let Some(part_id) = &self.view.create_hints[hint].id {
                let location = action_desc
                    .location()
                    .ok_or(AppError::BadOperationContext)?;

                // Update requirement to 1 if not set
                let count = self.store.get_by_location(&part_id, &location);
                if count.required() != 0 {
                    let ev = LedgerEntry {
                        t: Local::now().fixed_offset(),
                        count: 1,
                        part: part_id.clone(),
                        ev: LedgerEvent::RequireIn(location.clone()),
                    };
                    self.store.record_event(&ev)?;
                    self.store.update_count_cache(&ev);
                }

                self.store.show_empty_in_location(part_id, &location, true);
                return Ok(AppEvents::ReloadDataSelect(
                    self.store
                        .part_by_id(&part_id)
                        .map_or(part_id.to_string(), |p| p.metadata.name.clone()),
                ));
            }
        } else {
            // Enter on summary or name fields
            let part_id = self.create_object_from_dialog_data(|part| {
                part.metadata.types.insert(crate::store::ObjectType::Part);
            })?;

            if let Some(location) = action_desc.location() {
                // Update requirement to 1 if not set
                let count = self.store.get_by_location(&part_id, &location);
                if count.required() != 0 {
                    let ev = LedgerEntry {
                        t: Local::now().fixed_offset(),
                        count: 1,
                        part: part_id.clone(),
                        ev: LedgerEvent::RequireIn(location.clone()),
                    };
                    self.store.record_event(&ev)?;
                    self.store.update_count_cache(&ev);
                }

                self.store.show_empty_in_location(&part_id, &location, true);
            }

            return Ok(AppEvents::ReloadDataSelect(
                self.store
                    .part_by_id(&part_id)
                    .map_or(part_id.to_string(), |p| p.metadata.name.clone()),
            ));
        }
        Ok(AppEvents::Nop)
    }

    fn finish_create_location_for_part(&mut self) -> anyhow::Result<AppEvents> {
        let action_desc = self
            .get_active_panel_data()
            .actionable_objects(self.view.get_active_panel_selection(), &self.store)
            .ok_or(AppError::BadOperationContext)?;

        if let CreateMode::HINT(hint) = self.view.create_idx {
            if let Some(location_id) = &self.view.create_hints[hint].id {
                let part_id = action_desc.part().ok_or(AppError::BadOperationContext)?;

                // Update requirement to 1 if not set
                let count = self.store.get_by_location(&part_id, &location_id);
                if count.required() != 0 {
                    let ev = LedgerEntry {
                        t: Local::now().fixed_offset(),
                        count: 1,
                        part: part_id.clone(),
                        ev: LedgerEvent::RequireIn(location_id.clone()),
                    };
                    self.store.record_event(&ev)?;
                    self.store.update_count_cache(&ev);
                }

                self.store
                    .show_empty_in_location(part_id, &location_id, true);
                return Ok(AppEvents::ReloadDataSelect(
                    self.store
                        .part_by_id(&part_id)
                        .map_or(part_id.to_string(), |p| p.metadata.name.clone()),
                ));
            }
        } else {
            // Enter on summary or name fields
            let location_id = self.create_object_from_dialog_data(|location| {
                location
                    .metadata
                    .types
                    .insert(crate::store::ObjectType::Location);
            })?;

            if let Some(part_id) = action_desc.part() {
                // Update requirement to 1 if not set
                let count = self.store.get_by_location(&part_id, &location_id);
                if count.required() != 0 {
                    let ev = LedgerEntry {
                        t: Local::now().fixed_offset(),
                        count: 1,
                        part: part_id.clone(),
                        ev: LedgerEvent::RequireIn(location_id.clone()),
                    };
                    self.store.record_event(&ev)?;
                    self.store.update_count_cache(&ev);
                }

                self.store
                    .show_empty_in_location(&part_id, &location_id, true);
            }

            return Ok(AppEvents::ReloadDataSelect(
                self.store
                    .part_by_id(&location_id)
                    .map_or(location_id.to_string(), |p| p.metadata.name.clone()),
            ));
        }
        Ok(AppEvents::Nop)
    }

    fn finish_create_part_in_source(&mut self) -> anyhow::Result<AppEvents> {
        let action_desc = self
            .get_active_panel_data()
            .actionable_objects(self.view.get_active_panel_selection(), &self.store)
            .ok_or(AppError::BadOperationContext)?;

        if let CreateMode::HINT(hint) = self.view.create_idx {
            if let Some(part_id) = &self.view.create_hints[hint].id {
                let source = action_desc.source().ok_or(AppError::BadOperationContext)?;
                self.store.show_empty_in_source(part_id, &source, true);
                return Ok(AppEvents::ReloadDataSelect(
                    self.store
                        .part_by_id(&part_id)
                        .map_or(part_id.to_string(), |p| p.metadata.name.clone()),
                ));
            }
        } else {
            // Enter on summary or name fields
            let part_id = self.create_object_from_dialog_data(|part| {
                part.metadata.types.insert(crate::store::ObjectType::Part);
            })?;

            if let Some(source) = action_desc.source() {
                self.store.show_empty_in_source(&part_id, &source, true);
            }

            return Ok(AppEvents::ReloadDataSelect(
                self.store
                    .part_by_id(&part_id)
                    .map_or(part_id.to_string(), |p| p.metadata.name.clone()),
            ));
        }
        Ok(AppEvents::Nop)
    }

    fn finish_create_part_in_project(&mut self) -> anyhow::Result<AppEvents> {
        let action_desc = self
            .get_active_panel_data()
            .actionable_objects(self.view.get_active_panel_selection(), &self.store)
            .ok_or(AppError::BadOperationContext)?;

        if let CreateMode::HINT(hint) = self.view.create_idx {
            if let Some(part_id) = &self.view.create_hints[hint].id {
                let project_id = action_desc.project().ok_or(AppError::BadOperationContext)?;

                // Update order to 1 if not set
                let count = self.store.get_by_project(&part_id, &project_id);
                if count.required() == 0 {
                    let ev = LedgerEntry {
                        t: Local::now().fixed_offset(),
                        count: 1,
                        part: part_id.clone(),
                        ev: LedgerEvent::RequireInProject(project_id.clone()),
                    };
                    self.store.record_event(&ev)?;
                    self.store.update_count_cache(&ev);
                }

                self.store.show_empty_in_project(part_id, &project_id, true);
                return Ok(AppEvents::ReloadDataSelect(
                    self.store
                        .part_by_id(&part_id)
                        .map_or(part_id.to_string(), |p| p.metadata.name.clone()),
                ));
            }
        } else {
            // Enter on summary or name fields
            let part_id = self.create_object_from_dialog_data(|part| {
                part.metadata.types.insert(crate::store::ObjectType::Part);
            })?;

            if let Some(project_id) = action_desc.project() {
                // Update order to 1 if not set
                let count = self.store.get_by_project(&part_id, &project_id);
                if count.required() == 0 {
                    let ev = LedgerEntry {
                        t: Local::now().fixed_offset(),
                        count: 1,
                        part: part_id.clone(),
                        ev: LedgerEvent::RequireInProject(project_id.clone()),
                    };
                    self.store.record_event(&ev)?;
                    self.store.update_count_cache(&ev);
                }

                self.store
                    .show_empty_in_project(&part_id, &project_id, true);
            }

            return Ok(AppEvents::ReloadDataSelect(
                self.store
                    .part_by_id(&part_id)
                    .map_or(part_id.to_string(), |p| p.metadata.name.clone()),
            ));
        }
        Ok(AppEvents::Nop)
    }

    fn finish_create_label_key(&mut self) -> anyhow::Result<AppEvents> {
        if let CreateMode::HINT(hint) = self.view.create_idx {
            Ok(AppEvents::Select(self.view.create_hints[hint].name.clone()))
        } else {
            if self.view.create_name.value().trim().is_empty() {
                self.update_status("Label cannot be empty.");
                return Ok(AppEvents::Redraw);
            }

            let name = self.view.create_name.value().trim().to_string();
            self.store.add_label_key(&name);
            return Ok(AppEvents::ReloadDataSelect(name.clone()));
        }
    }

    fn finish_create_part_w_label(&mut self) -> anyhow::Result<AppEvents> {
        let action_desc = self
            .get_active_panel_data()
            .actionable_objects(self.view.get_active_panel_selection(), &self.store)
            .ok_or(AppError::BadOperationContext)?;

        if let CreateMode::HINT(hint) = self.view.create_idx {
            if let Some(id) = &self.view.create_hints[hint].id {
                let label = action_desc.label().ok_or(AppError::BadOperationContext)?;
                return self.perform_add_label(&id.clone(), (label.0.clone(), label.1.clone()));
            }
        } else {
            // Enter on summary or name fields
            let part_id = self.create_object_from_dialog_data(|part| {
                part.metadata.types.insert(crate::store::ObjectType::Part);
            })?;

            let label = action_desc.label().ok_or(AppError::BadOperationContext)?;

            self.perform_add_label(&part_id, (label.0.clone(), label.1.clone()))?;

            let name = self
                .store
                .part_by_id(&part_id)
                .map(|p| p.metadata.name.clone())
                .ok_or(AppError::NoSuchObject(part_id.to_string()))?;

            return Ok(AppEvents::ReloadDataSelect(name));
        }

        Ok(AppEvents::Nop)
    }

    fn finish_create_source(&mut self) -> anyhow::Result<AppEvents> {
        if let CreateMode::HINT(hint) = self.view.create_idx {
            Ok(AppEvents::Select(self.view.create_hints[hint].name.clone()))
        } else {
            // Enter on summary or name fields
            let part_id = self.create_object_from_dialog_data(|part| {
                part.metadata.types.insert(crate::store::ObjectType::Source);
            })?;

            self.update_status(&format!("Source {} was created.", part_id));
            return Ok(AppEvents::ReloadDataSelect(
                self.store
                    .part_by_id(&part_id)
                    .map_or(part_id.to_string(), |p| p.metadata.name.clone()),
            ));
        }
    }

    fn finish_create_project(&mut self) -> anyhow::Result<AppEvents> {
        if let CreateMode::HINT(hint) = self.view.create_idx {
            Ok(AppEvents::Select(self.view.create_hints[hint].name.clone()))
        } else {
            // Enter on summary or name fields
            let part_id = self.create_object_from_dialog_data(|part| {
                part.metadata.types.insert(crate::store::ObjectType::Part);
                part.metadata
                    .types
                    .insert(crate::store::ObjectType::Project);
            })?;

            self.update_status(&format!("Project {} was created.", part_id));
            let name = self
                .store
                .part_by_id(&part_id)
                .map_or(part_id.to_string(), |p| p.metadata.name.clone());
            return Ok(AppEvents::ReloadDataSelect(name));
        }
    }

    fn finish_create_label(&mut self) -> anyhow::Result<AppEvents> {
        if let CreateMode::HINT(hint) = self.view.create_idx {
            Ok(AppEvents::Select(self.view.create_hints[hint].name.clone()))
        } else {
            if self.view.create_name.value().trim().is_empty() {
                self.update_status("Label cannot be empty.");
                return Ok(AppEvents::Redraw);
            }

            let name = self.view.create_name.value().trim().to_string();
            let action_desc = self
                .get_active_panel_data()
                .actionable_objects(self.view.get_active_panel_selection(), &self.store);

            let action_desc = action_desc.ok_or(AppError::BadOperationContext)?;
            let label_key = action_desc
                .label_key()
                .ok_or(AppError::BadOperationContext)?;
            self.store.add_label(&label_key, &name);
            return Ok(AppEvents::ReloadDataSelect(name.clone()));
        }
    }

    fn make_new_id(&self, name: &str) -> PartId {
        let mut candidate = self.store.name_to_id(&name).into();
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

    fn create_object_from_dialog_data(&mut self, editor: fn(&mut Part)) -> anyhow::Result<PartId> {
        let name = self.view.create_name.value().trim().to_string();
        let mut part = Part {
            id: self.make_new_id(&name),
            filename: None,
            metadata: PartMetadata {
                id: None,
                name: name.clone(),
                summary: self.view.create_summary.value().trim().to_string(),
                ..Default::default()
            },
            content: "".to_string(),
        };

        editor(&mut part);

        self.store.store_part(&mut part)?;

        let part_id = part.id.clone();

        self.store.insert_part_to_cache(part);

        return Ok(part_id);
    }

    fn prepare_delete(&mut self) {
        if self.get_active_panel_data().data_type().can_delete() {
            self.view.delete_item = Some(
                self.get_active_panel_data()
                    .item(self.view.get_active_panel_selection(), &self.store),
            );
            self.view.delete_dialog = DialogState::VISIBLE;
            self.view.delete_from = self
                .get_active_panel_data()
                .panel_title(&self.store)
                .clone();
        }
    }

    fn finish_delete(&mut self) -> anyhow::Result<AppEvents> {
        self.view.hide_delete_dialog();
        let action_descriptor = self
            .get_active_panel_data()
            .actionable_objects(self.view.get_active_panel_selection(), &self.store);

        match self.get_active_panel_data().data_type() {
            PanelContent::None => return Ok(AppEvents::Redraw),
            PanelContent::TypeSelection => todo!(),
            PanelContent::Parts => self.update_status("Part delete is not implemented yet."),
            PanelContent::Locations => {
                self.update_status("Location delete is not implemented yet.")
            }
            PanelContent::PartsInLocation => {
                return self.finish_remove_part_from_location(action_descriptor);
            }
            PanelContent::LocationOfParts => {
                return self.finish_remove_part_from_location(action_descriptor);
            }
            PanelContent::LabelKeys | PanelContent::Labels => {
                self.update_status("Labels will disappear when not present on any parts.")
            }
            PanelContent::PartsWithLabels => {
                return self.finish_remove_label_from_part(action_descriptor);
            }
            PanelContent::Sources => self.update_status("Source deletion not implemented yet."),
            PanelContent::PartsInOrders => {
                return self.finish_remove_part_from_source(action_descriptor);
            }
            PanelContent::PartsFromSources => {
                return self.finish_remove_part_from_source(action_descriptor);
            }
            PanelContent::Projects => self.update_status("Project deletion not implemented yet."),
            PanelContent::PartsInProjects => {
                return self.finish_remove_part_from_project(action_descriptor);
            }
        }

        Ok(AppEvents::ReloadData)
    }

    fn finish_remove_label_from_part(
        &mut self,
        action_descriptor: Option<ActionDescriptor>,
    ) -> anyhow::Result<AppEvents> {
        let ad = action_descriptor.ok_or(AppError::BadOperationContext)?;
        let (key, value) = ad.label().ok_or(AppError::BadOperationContext)?;
        let part = ad.part().ok_or(AppError::BadOperationContext)?;
        self.update_status(&format!("Label {}: {} removed from {}", key, value, part));
        self.perform_remove_label(part, (key.clone(), value.clone()))
            .or(Ok(AppEvents::Redraw))
    }

    pub fn update_status(&mut self, msg: &str) {
        info!("status: {}", msg);
        self.view.status = msg.to_owned();
    }

    fn finish_remove_part_from_source(
        &mut self,
        action_descriptor: Option<ActionDescriptor>,
    ) -> anyhow::Result<AppEvents> {
        let ad = action_descriptor.ok_or(AppError::BadOperationContext)?;
        let part_id = ad.part().ok_or(AppError::BadOperationContext)?;
        let source_id = ad.source().ok_or(AppError::BadOperationContext)?;

        let count = self.store.get_by_source(part_id, source_id);
        let entry = LedgerEntry {
            t: Local::now().fixed_offset(),
            count: count.required().saturating_sub(count.added()),
            part: Rc::clone(part_id),
            ev: LedgerEvent::CancelOrderFrom(Rc::clone(source_id)),
        };
        if entry.count > 0 {
            self.store.record_event(&entry)?;
            self.store.update_count_cache(&entry);
            self.store.show_empty_in_source(part_id, source_id, true);
            self.update_status(format!("Order of {} cancelled.", part_id).as_str());
            return Ok(AppEvents::ReloadData);
        }

        self.store.show_empty_in_source(part_id, source_id, false);
        Ok(AppEvents::ReloadData)
    }

    fn finish_remove_part_from_location(
        &mut self,
        action_descriptor: Option<ActionDescriptor>,
    ) -> anyhow::Result<AppEvents> {
        let ad = action_descriptor.ok_or(AppError::BadOperationContext)?;
        let part_id = ad.part().ok_or(AppError::BadOperationContext)?;
        let location_id = ad.location().ok_or(AppError::BadOperationContext)?;

        let count = self.store.get_by_location(part_id, location_id);
        if count.required() > 0 {
            let require_zero = LedgerEntry {
                t: Local::now().fixed_offset(),
                count: 0,
                part: part_id.clone(),
                ev: LedgerEvent::RequireIn(location_id.clone()),
            };
            self.store.record_event(&require_zero)?;
            self.store.update_count_cache(&require_zero);
            self.store
                .show_empty_in_location(part_id, &location_id, true);
            self.update_status(format!("Requirement of {} cancelled.", part_id).as_str());
            return Ok(AppEvents::ReloadData);
        }

        self.store
            .show_empty_in_location(part_id, &location_id, false);
        Ok(AppEvents::ReloadData)
    }

    fn finish_remove_part_from_project(
        &mut self,
        action_descriptor: Option<ActionDescriptor>,
    ) -> anyhow::Result<AppEvents> {
        let ad = action_descriptor.ok_or(AppError::BadOperationContext)?;
        let part_id = ad.part().ok_or(AppError::BadOperationContext)?;
        let project_id = ad.project().ok_or(AppError::BadOperationContext)?;

        let count = self.store.get_by_project(part_id, project_id);
        if count.required() > 0 {
            let require_zero = LedgerEntry {
                t: Local::now().fixed_offset(),
                count: 0,
                part: part_id.clone(),
                ev: LedgerEvent::RequireInProject(project_id.clone()),
            };
            self.store.record_event(&require_zero)?;
            self.store.update_count_cache(&require_zero);
            self.store.show_empty_in_project(part_id, &project_id, true);
            self.update_status(format!("Requirement of {} cancelled.", part_id).as_str());
            return Ok(AppEvents::ReloadData);
        }

        self.store
            .show_empty_in_project(part_id, &project_id, false);
        Ok(AppEvents::ReloadData)
    }

    fn press_f4(&self) -> Result<AppEvents, AppError> {
        let item = self
            .get_active_panel_data()
            .item(self.view.get_active_panel_selection(), &self.store);
        let part_id = item.id.ok_or(AppError::PartHasNoId)?;
        Ok(AppEvents::Edit(part_id))
    }

    pub fn get_part(&self, p_id: &PartId) -> Option<&Part> {
        self.store.part_by_id(p_id)
    }

    pub fn reload_part(&mut self, part: &Part) {
        self.store.insert_part_to_cache(part.clone());
    }

    pub fn show_alert(&mut self, title: &str, alert: &str) {
        self.view.alert_dialog = DialogState::VISIBLE;
        self.view.alert_title = title.to_string();
        self.view.alert_text = alert.to_string();
        error!("{}: {}", title, alert);
    }

    fn finish_action_require_local(
        &mut self,
        source: Option<&ActionDescriptor>,
    ) -> anyhow::Result<AppEvents> {
        let ad = source.ok_or(AppError::BadOperationContext)?;
        let part_id = ad.part().ok_or(AppError::BadOperationContext)?;

        if let Some(location_id) = ad.location() {
            let ev = LedgerEntry {
                t: Local::now().fixed_offset(),
                count: self.view.action_count_dialog_count,
                part: Rc::clone(part_id),
                ev: LedgerEvent::RequireIn(Rc::clone(location_id)),
            };
            self.store.update_count_cache(&ev);
            self.store.record_event(&ev)?;
        } else if let Some(source_id) = ad.source() {
            let ev = LedgerEntry {
                t: Local::now().fixed_offset(),
                count: self.view.action_count_dialog_count,
                part: Rc::clone(part_id),
                ev: LedgerEvent::OrderFrom(Rc::clone(source_id)),
            };
            self.store.update_count_cache(&ev);
            self.store.record_event(&ev)?;
        } else if let Some(project_id) = ad.project() {
            let ev = LedgerEntry {
                t: Local::now().fixed_offset(),
                count: self.view.action_count_dialog_count,
                part: Rc::clone(part_id),
                ev: LedgerEvent::RequireInProject(Rc::clone(project_id)),
            };
            self.store.update_count_cache(&ev);
            self.store.record_event(&ev)?;
        } else {
            return Ok(AppEvents::Redraw);
        }

        return Ok(AppEvents::ReloadData);
    }

    fn action_clone_part(&mut self) -> Result<AppEvents, AppError> {
        let item_id = self
            .get_active_panel_data()
            .item(self.view.get_active_panel_selection(), &self.store)
            .id
            .ok_or(AppError::PartHasNoId)?;
        let item = self
            .store
            .part_by_id(&item_id)
            .ok_or(AppError::NoSuchObject(item_id.to_string()))?;
        let is_project = item
            .metadata
            .types
            .contains(&crate::store::ObjectType::Project);

        let mut new_item = item.clone();
        let new_id = self.make_new_id(&item.metadata.name);
        let new_name = [&item.metadata.name, " - clone"].join("");
        new_item.id = Rc::clone(&new_id);
        new_item.metadata.id = Some(new_item.id.to_string());
        new_item.metadata.name = new_name.clone();
        new_item.filename = None;

        self.store.store_part(&mut new_item)?;
        self.store.insert_part_to_cache(new_item);

        if is_project {
            // Clone requirements
            for r in self.store.count_by_project(&item_id) {
                let entry = LedgerEntry {
                    t: Local::now().fixed_offset(),
                    count: r.required(),
                    part: Rc::clone(r.part()),
                    ev: LedgerEvent::RequireInProject(Rc::clone(&new_id)),
                };
                self.store.record_event(&entry)?;
                self.store.update_count_cache(&entry);
            }
        }

        Ok(AppEvents::ReloadDataSelect(new_name))
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

    fn item_name(&self, _idx: usize, _store: &Store) -> String {
        todo!()
    }

    fn item(&self, _idx: usize, _store: &Store) -> PanelItem {
        todo!()
    }
}
