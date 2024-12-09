use chrono::Local;

use crate::store::{LedgerEntry, LedgerEvent};

use super::{errs::AppError, model::ActionDescriptor, App, AppEvents};

impl App {
    pub(super) fn finish_action_move(
        &mut self,
        source: &Option<ActionDescriptor>,
        destination: &Option<ActionDescriptor>,
    ) -> anyhow::Result<AppEvents> {
        let part_id = source
            .as_ref()
            .and_then(|s| s.part().cloned())
            .ok_or(AppError::BadOperationContext)?;
        let source = source
            .as_ref()
            .and_then(|d| d.location().cloned())
            .ok_or(AppError::BadOperationContext)?;
        let destination = destination
            .as_ref()
            .and_then(|d| d.location().cloned())
            .ok_or(AppError::BadOperationContext)?;

        self.update_status(&format!(
            "{} parts {} moved from {} to {}",
            self.view.action_count_dialog_count, &part_id, &source, &destination
        ));

        let now = Local::now().fixed_offset();

        let event_from = LedgerEntry {
            t: now,
            count: self.view.action_count_dialog_count,
            part: part_id.clone(),
            ev: LedgerEvent::TakeFrom(source),
        };
        let event_to = LedgerEntry {
            t: now,
            count: self.view.action_count_dialog_count,
            part: part_id,
            ev: LedgerEvent::StoreTo(destination),
        };

        self.store.record_event(&event_from)?;
        self.store.record_event(&event_to)?;

        self.store.update_count_cache(&event_from);
        self.store.update_count_cache(&event_to);

        Ok(AppEvents::ReloadData)
    }
}
