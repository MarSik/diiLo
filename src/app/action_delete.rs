use std::rc::Rc;

use chrono::Local;

use crate::store::{cache::CountCacheSum, LedgerEntry, LedgerEvent, PartId};

use super::{
    errs::AppError,
    model::{ActionDescriptor, PanelContent},
    view::DialogState,
    App, AppEvents,
};

impl App {
    pub(super) fn prepare_delete(&mut self) {
        if self.get_active_panel_data().data_type().can_delete() {
            self.view.delete_item = Some(
                self.get_active_panel_data()
                    .item(self.view.get_active_panel_selection(), &self.store),
            );
            self.view.delete_dialog = DialogState::Visible;
            self.view.delete_from = self
                .get_active_panel_data()
                .panel_title(&self.store)
                .clone();
        }
    }

    pub(super) fn finish_delete(&mut self) -> anyhow::Result<AppEvents> {
        self.view.hide_delete_dialog();
        let action_descriptor = self
            .get_active_panel_data()
            .actionable_objects(self.view.get_active_panel_selection(), &self.store);

        match self.get_active_panel_data().data_type() {
            PanelContent::None => return Ok(AppEvents::Redraw),
            PanelContent::TypeSelection => todo!(),
            PanelContent::Parts => {
                return self.finish_delete_part(action_descriptor);
            }
            PanelContent::Locations => {
                return self.finish_delete_location(action_descriptor);
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
            PanelContent::Sources => {
                return self.finish_delete_source(action_descriptor);
            }
            PanelContent::PartsInOrders => {
                return self.finish_remove_part_from_source(action_descriptor);
            }
            PanelContent::PartsFromSources => {
                return self.finish_remove_part_from_source(action_descriptor);
            }
            PanelContent::Projects => {
                return self.finish_delete_project(action_descriptor);
            }
            PanelContent::PartsInProjects => {
                return self.finish_remove_part_from_project(action_descriptor);
            }
        }

        Ok(AppEvents::ReloadData)
    }

    fn finish_delete_part(
        &mut self,
        action_descriptor: Option<ActionDescriptor>,
    ) -> Result<AppEvents, anyhow::Error> {
        let part_id = action_descriptor
            .and_then(|d| d.part().cloned())
            .ok_or(AppError::BadOperationContext)?;
        let counts = self.store.count_by_part_type(part_id.part_type()).sum();
        if counts.added != 0 || counts.removed != 0 || counts.required != 0 {
            self.update_status("Part cannot be deleted, because it is tracked.");
            return Ok(AppEvents::Nop);
        }

        let counts = self.store.get_projects_by_part(&part_id).sum();
        if counts.added != 0 || counts.removed != 0 || counts.required != 0 {
            self.update_status("Part cannot be deleted, because it is tracked in projects.");
            return Ok(AppEvents::Nop);
        }

        let counts = self.store.get_sources_by_part(&part_id).sum();
        if counts.added != 0 || counts.removed != 0 || counts.required != 0 {
            self.update_status("Part cannot be deleted, because it is tracked in sources.");
            return Ok(AppEvents::Nop);
        }

        let res = self
            .store
            .remove(part_id.part_type())
            .map(|_| AppEvents::ReloadData)?;
        self.update_status(format!("Part {} was DELETED!", part_id).as_str());
        Ok(res)
    }

    fn finish_delete_location(
        &mut self,
        action_descriptor: Option<ActionDescriptor>,
    ) -> Result<AppEvents, anyhow::Error> {
        let location_id = action_descriptor
            .and_then(|d| d.location().cloned())
            .ok_or(AppError::BadOperationContext)?;
        let counts = self.store.count_by_location(&location_id).sum();
        if counts.added != 0 || counts.removed != 0 || counts.required != 0 {
            self.update_status("Location cannot be deleted, because it contains parts");
            return Ok(AppEvents::Nop);
        }

        let res = self
            .store
            .remove(location_id.part_type())
            .map(|_| AppEvents::ReloadData)?;
        self.update_status(format!("Location {} was DELETED!", location_id).as_str());
        Ok(res)
    }

    fn finish_delete_project(
        &mut self,
        action_descriptor: Option<ActionDescriptor>,
    ) -> Result<AppEvents, anyhow::Error> {
        let project_id = action_descriptor
            .and_then(|d| d.project().cloned())
            .ok_or(AppError::BadOperationContext)?;
        let counts = self.store.count_by_project(&project_id).sum();
        if counts.added != 0 || counts.removed != 0 || counts.required != 0 {
            self.update_status("Project cannot be deleted, because it contains parts");
            return Ok(AppEvents::Nop);
        }

        let res = self
            .store
            .remove(project_id.part_type())
            .map(|_| AppEvents::ReloadData)?;
        self.update_status(format!("Project {} was DELETED!", project_id).as_str());
        Ok(res)
    }

    fn finish_delete_source(
        &mut self,
        action_descriptor: Option<ActionDescriptor>,
    ) -> Result<AppEvents, anyhow::Error> {
        let source_id = action_descriptor
            .and_then(|d| d.source().cloned())
            .ok_or(AppError::BadOperationContext)?;
        let counts = self.store.count_by_source(&source_id).sum();
        if counts.required != 0 {
            self.update_status("Source cannot be deleted, because it contains ordered parts");
            return Ok(AppEvents::Nop);
        }

        let res = self
            .store
            .remove(&source_id)
            .map(|_| AppEvents::ReloadData)?;
        self.update_status(format!("Source {} was DELETED!", source_id).as_str());
        Ok(res)
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
                .show_empty_in_location(part_id, location_id, true);
            self.update_status(format!("Requirement of {} cancelled.", part_id).as_str());
            return Ok(AppEvents::ReloadData);
        }

        self.store
            .show_empty_in_location(part_id, location_id, false);
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
            self.store.show_empty_in_project(part_id, project_id, true);
            self.update_status(format!("Requirement of {} cancelled.", part_id).as_str());
            return Ok(AppEvents::ReloadData);
        }

        self.store.show_empty_in_project(part_id, project_id, false);
        Ok(AppEvents::ReloadData)
    }

    pub(super) fn finish_remove_part_from_source(
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
            part: PartId::clone(part_id),
            ev: LedgerEvent::CancelOrderFrom(Rc::clone(source_id)),
        };
        if entry.count > 0 {
            self.store.record_event(&entry)?;
            self.store.update_count_cache(&entry);
            self.store
                .show_empty_in_source(part_id, &source_id.into(), true);
            self.update_status(format!("Order of {} cancelled.", part_id).as_str());
            return Ok(AppEvents::ReloadData);
        }

        self.store
            .show_empty_in_source(part_id, &source_id.into(), false);
        Ok(AppEvents::ReloadData)
    }
}
