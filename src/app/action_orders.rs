use chrono::Local;

use crate::store::{LedgerEntry, LedgerEvent, LocationId, ProjectId};

use super::{errs::AppError, model::ActionDescriptor, ActionVariant, App, AppEvents};

impl App {
    pub(super) fn finish_action_require(
        &mut self,
        source: &Option<ActionDescriptor>,
        destination: &Option<ActionDescriptor>,
    ) -> anyhow::Result<AppEvents> {
        let part = source
            .as_ref()
            .and_then(|s| s.part().cloned())
            .ok_or(AppError::BadOperationContext)?;
        let (destination, ev) = if let Some(destination) =
            destination.as_ref().and_then(|d| d.location().cloned())
        {
            (
                LocationId::clone(&destination),
                LedgerEvent::RequireIn(destination),
            )
        } else if let Some(project_id) = destination.as_ref().and_then(|d| d.project().cloned()) {
            (
                ProjectId::clone(&project_id),
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
            part: part.simple(),
            ev,
        };

        self.store.record_event(&event_to)?;
        self.store.update_count_cache(&event_to);

        Ok(AppEvents::ReloadData)
    }

    pub(super) fn finish_action_order(
        &mut self,
        source: &Option<ActionDescriptor>,
        destination: &Option<ActionDescriptor>,
    ) -> anyhow::Result<AppEvents> {
        let part = source
            .as_ref()
            .and_then(|s| s.part().cloned())
            .ok_or(AppError::BadOperationContext)?;
        let destination = destination
            .as_ref()
            .and_then(|d| d.source().cloned())
            .ok_or(AppError::BadOperationContext)?;

        self.update_status(&format!(
            "{} parts {} ordered from {}",
            self.view.action_count_dialog_count, &part, &destination
        ));

        let event_to = LedgerEntry {
            t: Local::now().fixed_offset(),
            count: self.view.action_count_dialog_count,
            part: part.simple(),
            ev: LedgerEvent::OrderFrom(destination),
        };

        self.store.record_event(&event_to)?;
        self.store.update_count_cache(&event_to);

        Ok(AppEvents::ReloadData)
    }

    pub(super) fn finish_action_deliver(
        &mut self,
        source: &Option<ActionDescriptor>,
        destination: &Option<ActionDescriptor>,
    ) -> anyhow::Result<AppEvents> {
        let part = source
            .as_ref()
            .and_then(|s| s.part().cloned())
            .ok_or(AppError::BadOperationContext)?;
        let source = source
            .as_ref()
            .and_then(|d| d.source().cloned())
            .ok_or(AppError::BadOperationContext)?;
        let destination = destination
            .as_ref()
            .and_then(|d| d.location().cloned())
            .ok_or(AppError::BadOperationContext)?;

        self.update_status(&format!(
            "{} parts {} delivered from {} to {}",
            self.view.action_count_dialog_count, &part, &source, &destination
        ));

        let event_from = LedgerEntry {
            t: Local::now().fixed_offset(),
            count: self.view.action_count_dialog_count,
            part: part.simple(),
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

    pub(super) fn prepare_require_part_local(
        &mut self,
        action: ActionVariant,
    ) -> Result<AppEvents, AppError> {
        let ad = self
            .get_active_panel_data()
            .actionable_objects(self.view.get_active_panel_selection(), &self.store)
            .ok_or(AppError::BadOperationContext)?;

        let part_id = ad.part().ok_or(AppError::BadOperationContext)?;
        let part_item = Some(self.panel_item_from_id(part_id)?);

        if let Some(location_id) = ad.location() {
            let count = self
                .store
                .count_by_part_location(part_id, location_id)
                .required();
            self.view.show_action_dialog(
                action,
                part_item,
                Some(self.panel_item_from_id(location_id)?),
                count,
            );
        } else if let Some(project_id) = ad.project() {
            let count = self
                .store
                .count_by_part_project(part_id, project_id)
                .required();
            self.view.show_action_dialog(
                action,
                part_item,
                Some(self.panel_item_from_id(project_id)?),
                count,
            );
        } else if let Some(source_id) = ad.source() {
            let count = self
                .store
                .count_by_part_source(part_id, source_id)
                .required();
            self.view.show_action_dialog(
                action,
                part_item,
                Some(self.panel_item_from_id(&source_id.into())?),
                count,
            );
        } else {
            return Err(AppError::BadOperationContext);
        };

        Ok(AppEvents::Redraw)
    }
}
