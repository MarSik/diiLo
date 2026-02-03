use chrono::Local;

use crate::store::{LedgerEntry, LedgerEvent};

use super::{App, AppEvents, errs::AppError, model::ActionDescriptor};

impl App {
    pub(super) fn finish_action_solder(
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
            .and_then(|d| d.location().cloned())
            .ok_or(AppError::BadOperationContext)?;
        let destination = destination
            .as_ref()
            .and_then(|d| d.project().cloned())
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

    pub(super) fn finish_action_unsolder(
        &mut self,
        source: Option<ActionDescriptor>,
        destination: Option<ActionDescriptor>,
    ) -> anyhow::Result<AppEvents> {
        let part = source
            .as_ref()
            .and_then(|s| s.part().cloned())
            .ok_or(AppError::BadOperationContext)?;
        let source = source
            .and_then(|d| d.project().cloned())
            .ok_or(AppError::BadOperationContext)?;
        let destination = destination
            .and_then(|d| d.location().cloned())
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

        Ok(AppEvents::ReloadData)
    }
}
