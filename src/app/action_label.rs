use crate::store::PartId;

use super::{
    errs::AppError,
    model::{ActionDescriptor, PanelItem},
    ActionVariant, App, AppEvents,
};

impl App {
    pub(super) fn prepare_add_remove_label(
        &mut self,
        label_ad: ActionDescriptor,
        part_ad: ActionDescriptor,
        action: ActionVariant,
    ) -> Result<(), AppError> {
        let part_id = part_ad.part().ok_or(AppError::BadOperationContext)?;
        let label = label_ad.label().ok_or(AppError::BadOperationContext)?;
        let label_item = PanelItem {
            name: format!("{}: {}", label.0, label.1),
            summary: String::with_capacity(0),
            data: String::with_capacity(0),
            id: None,
            parent_id: None,
        };
        self.view.show_action_dialog(
            action,
            Some(label_item),
            Some(self.panel_item_from_id(part_id)?),
            0,
        );

        Ok(())
    }

    pub(super) fn finish_action_add_label_to_part(
        &mut self,
        label_ad: &Option<ActionDescriptor>,
        part_ad: &Option<ActionDescriptor>,
    ) -> anyhow::Result<AppEvents> {
        let part_id = part_ad
            .as_ref()
            .and_then(ActionDescriptor::part)
            .ok_or(AppError::BadOperationContext)?;
        let label = label_ad
            .as_ref()
            .and_then(ActionDescriptor::label)
            .ok_or(AppError::BadOperationContext)?;

        self.perform_add_label(part_id, label)
    }

    pub(super) fn finish_action_remove_label_from_part(
        &mut self,
        label_ad: &Option<ActionDescriptor>,
        part_ad: &Option<ActionDescriptor>,
    ) -> anyhow::Result<AppEvents> {
        let part_id = part_ad
            .as_ref()
            .and_then(ActionDescriptor::part)
            .ok_or(AppError::BadOperationContext)?;
        let label = label_ad
            .as_ref()
            .and_then(ActionDescriptor::label)
            .ok_or(AppError::BadOperationContext)?;
        self.perform_remove_label(part_id, label)
            .or(Ok(AppEvents::Redraw))
    }

    pub(super) fn finish_remove_label_from_part(
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

    fn perform_remove_label(
        &mut self,
        part_id: &PartId,
        label: (String, String),
    ) -> anyhow::Result<AppEvents> {
        let part = self
            .store
            .part_by_id(part_id.part_type())
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

    pub(super) fn perform_add_label(
        &mut self,
        part_id: &PartId,
        label: (String, String),
    ) -> anyhow::Result<AppEvents> {
        if let Some(part) = self.store.part_by_id(part_id.part_type()) {
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
}
