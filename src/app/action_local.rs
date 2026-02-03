use chrono::Local;

use crate::store::{LedgerEntry, LedgerEvent, LocationId, PartId, ProjectId, SourceId};

use super::{ActionVariant, App, AppEvents, errs::AppError, model::ActionDescriptor};

impl App {
    pub(super) fn finish_action_split_local(
        &mut self,
        ad: Option<&ActionDescriptor>,
    ) -> anyhow::Result<AppEvents> {
        let part_id = ad
            .and_then(|ad| ad.part().cloned())
            .ok_or(AppError::BadOperationContext)?;
        let location_id = ad
            .and_then(|ad| ad.location().cloned())
            .ok_or(AppError::BadOperationContext)?;

        let t = Local::now().fixed_offset();

        let ev_rm = LedgerEntry {
            t,
            count: self.view.action_count_dialog_count,
            part: part_id.clone(),
            ev: LedgerEvent::TakeFrom(location_id.clone()),
        };

        let ev_st = LedgerEntry {
            t,
            count: self.view.action_count_dialog_count,
            part: part_id,
            ev: LedgerEvent::StoreTo(location_id),
        };

        self.store.record_event(&ev_rm)?;
        self.store.record_event(&ev_st)?;

        self.store.update_count_cache(&ev_rm);
        self.store.update_count_cache(&ev_st);

        Ok(AppEvents::ReloadData)
    }

    pub(super) fn prepare_split_local(&mut self) -> Result<AppEvents, AppError> {
        let ad = self
            .get_active_panel_data()
            .actionable_objects(self.view.get_active_panel_selection(), &self.store);
        let part_id = ad
            .as_ref()
            .and_then(|ad| ad.part())
            .ok_or(AppError::BadOperationContext)?;
        let location_id = ad
            .as_ref()
            .and_then(|ad| ad.location())
            .ok_or(AppError::BadOperationContext)?;
        let count = self.store.count_by_part_location(part_id, location_id);

        if part_id.piece_size_option().is_none() {
            self.update_status(&format! {"Part {} is not tracking pieces", part_id.part_type()});
            return Ok(AppEvents::Redraw);
        }

        self.view.show_action_dialog(
            ActionVariant::SplitLocal,
            Some(self.panel_item_from_id(part_id)?),
            Some(self.panel_item_from_id(location_id)?),
            count.count().max(0) as usize,
            1, // Split can cut any amount
        );
        Ok(AppEvents::Redraw)
    }

    pub(super) fn prepare_force_count(&mut self) -> Result<AppEvents, AppError> {
        let part_id = self
            .get_active_panel_data()
            .actionable_objects(self.view.get_active_panel_selection(), &self.store)
            .and_then(|ad| ad.part().cloned())
            .ok_or(AppError::BadOperationContext)?;
        let location_id = self
            .get_inactive_panel_data()
            .actionable_objects(self.view.get_inactive_panel_selection(), &self.store)
            .and_then(|ad| ad.location().cloned())
            .ok_or(AppError::BadOperationContext)?;
        let count = self.store.count_by_part_location(&part_id, &location_id);

        self.view.show_action_dialog(
            ActionVariant::ForceCount,
            Some(self.panel_item_from_id(&part_id)?),
            Some(self.panel_item_from_id(&location_id)?),
            count.count().max(0) as usize,
            part_id.piece_size(),
        );
        Ok(AppEvents::Redraw)
    }

    pub(super) fn prepare_force_count_local(&mut self) -> Result<AppEvents, AppError> {
        let ad = self
            .get_active_panel_data()
            .actionable_objects(self.view.get_active_panel_selection(), &self.store);
        let part_id = ad
            .as_ref()
            .and_then(|ad| ad.part())
            .ok_or(AppError::BadOperationContext)?;
        let location_id = ad
            .as_ref()
            .and_then(|ad| ad.location())
            .ok_or(AppError::BadOperationContext)?;
        let count = self.store.count_by_part_location(part_id, location_id);

        self.view.show_action_dialog(
            ActionVariant::ForceCountLocal,
            Some(self.panel_item_from_id(part_id)?),
            Some(self.panel_item_from_id(location_id)?),
            count.count().max(0) as usize,
            part_id.piece_size(),
        );
        Ok(AppEvents::Redraw)
    }

    pub(super) fn finish_action_force_count(
        &mut self,
        source: Option<ActionDescriptor>,
        destination: Option<ActionDescriptor>,
    ) -> Result<AppEvents, anyhow::Error> {
        let part_id = source
            .and_then(|ad| ad.part().cloned())
            .ok_or(AppError::BadOperationContext)?;
        let location_id = destination
            .and_then(|ad| ad.location().cloned())
            .ok_or(AppError::BadOperationContext)?;
        let ev = LedgerEntry {
            t: Local::now().fixed_offset(),
            count: self.view.action_count_dialog_count,
            part: part_id,
            ev: LedgerEvent::ForceCount(location_id),
        };
        self.store.record_event(&ev)?;
        self.store.update_count_cache(&ev);
        Ok(AppEvents::ReloadData)
    }

    pub(super) fn finish_action_force_count_local(
        &mut self,
        ad: Option<&ActionDescriptor>,
    ) -> Result<AppEvents, anyhow::Error> {
        let part_id = ad
            .and_then(|ad| ad.part())
            .ok_or(AppError::BadOperationContext)?;
        let location_id = ad
            .and_then(|ad| ad.location())
            .ok_or(AppError::BadOperationContext)?;
        let ev = LedgerEntry {
            t: Local::now().fixed_offset(),
            count: self.view.action_count_dialog_count,
            part: PartId::clone(part_id),
            ev: LedgerEvent::ForceCount(LocationId::clone(location_id)),
        };
        self.store.record_event(&ev)?;
        self.store.update_count_cache(&ev);
        Ok(AppEvents::ReloadData)
    }

    pub(super) fn finish_action_require_local(
        &mut self,
        source: Option<&ActionDescriptor>,
    ) -> anyhow::Result<AppEvents> {
        let ad = source.ok_or(AppError::BadOperationContext)?;
        let part_id = ad.part().ok_or(AppError::BadOperationContext)?;

        if let Some(location_id) = ad.location() {
            let ev = LedgerEntry {
                t: Local::now().fixed_offset(),
                count: self.view.action_count_dialog_count,
                part: PartId::clone(part_id),
                ev: LedgerEvent::RequireIn(LocationId::clone(location_id)),
            };
            self.store.update_count_cache(&ev);
            self.store.record_event(&ev)?;
        } else if let Some(source_id) = ad.source() {
            let ev = LedgerEntry {
                t: Local::now().fixed_offset(),
                count: self.view.action_count_dialog_count,
                part: PartId::clone(part_id),
                ev: LedgerEvent::OrderFrom(SourceId::clone(source_id)),
            };
            self.store.update_count_cache(&ev);
            self.store.record_event(&ev)?;
        } else if let Some(project_id) = ad.project() {
            let ev = LedgerEntry {
                t: Local::now().fixed_offset(),
                count: self.view.action_count_dialog_count,
                part: PartId::clone(part_id),
                ev: LedgerEvent::RequireInProject(ProjectId::clone(project_id)),
            };
            self.store.update_count_cache(&ev);
            self.store.record_event(&ev)?;
        } else {
            return Ok(AppEvents::Redraw);
        }

        Ok(AppEvents::ReloadData)
    }

    pub(super) fn prepare_force_count_project(&mut self) -> Result<AppEvents, AppError> {
        let part_id = self
            .get_active_panel_data()
            .actionable_objects(self.view.get_active_panel_selection(), &self.store)
            .and_then(|ad| ad.part().cloned())
            .ok_or(AppError::BadOperationContext)?;
        let project_id = self
            .get_inactive_panel_data()
            .actionable_objects(self.view.get_inactive_panel_selection(), &self.store)
            .and_then(|ad| ad.project().cloned())
            .ok_or(AppError::BadOperationContext)?;
        let count = self.store.count_by_part_project(&part_id, &project_id);

        self.view.show_action_dialog(
            ActionVariant::ForceCountProject,
            Some(self.panel_item_from_id(&part_id)?),
            Some(self.panel_item_from_id(&project_id)?),
            count.count().max(0) as usize,
            part_id.piece_size(),
        );
        Ok(AppEvents::Redraw)
    }

    pub(super) fn prepare_force_count_project_local(&mut self) -> Result<AppEvents, AppError> {
        let ad = self
            .get_active_panel_data()
            .actionable_objects(self.view.get_active_panel_selection(), &self.store);
        let part_id = ad
            .as_ref()
            .and_then(|ad| ad.part())
            .ok_or(AppError::BadOperationContext)?;
        let project_id = ad
            .as_ref()
            .and_then(|ad| ad.project())
            .ok_or(AppError::BadOperationContext)?;
        let count = self.store.count_by_part_project(part_id, project_id);

        self.view.show_action_dialog(
            ActionVariant::ForceCountProjectLocal,
            Some(self.panel_item_from_id(part_id)?),
            Some(self.panel_item_from_id(project_id)?),
            count.count().max(0) as usize,
            part_id.piece_size(),
        );
        Ok(AppEvents::Redraw)
    }

    pub(super) fn finish_action_force_count_project(
        &mut self,
        source: Option<ActionDescriptor>,
        destination: Option<ActionDescriptor>,
    ) -> Result<AppEvents, anyhow::Error> {
        let part_id = source
            .and_then(|ad| ad.part().cloned())
            .ok_or(AppError::BadOperationContext)?;
        let project_id = destination
            .and_then(|ad| ad.project().cloned())
            .ok_or(AppError::BadOperationContext)?;
        let ev = LedgerEntry {
            t: Local::now().fixed_offset(),
            count: self.view.action_count_dialog_count,
            part: part_id,
            ev: LedgerEvent::ForceCountProject(project_id),
        };
        self.store.record_event(&ev)?;
        self.store.update_count_cache(&ev);
        Ok(AppEvents::ReloadData)
    }

    pub(super) fn finish_action_force_count_project_local(
        &mut self,
        ad: Option<&ActionDescriptor>,
    ) -> Result<AppEvents, anyhow::Error> {
        let part_id = ad
            .and_then(|ad| ad.part())
            .ok_or(AppError::BadOperationContext)?;
        let project_id = ad
            .and_then(|ad| ad.project())
            .ok_or(AppError::BadOperationContext)?;
        let ev = LedgerEntry {
            t: Local::now().fixed_offset(),
            count: self.view.action_count_dialog_count,
            part: PartId::clone(part_id),
            ev: LedgerEvent::ForceCountProject(ProjectId::clone(project_id)),
        };
        self.store.record_event(&ev)?;
        self.store.update_count_cache(&ev);
        Ok(AppEvents::ReloadData)
    }
}
