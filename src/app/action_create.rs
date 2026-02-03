use std::rc::Rc;

use chrono::Local;

use crate::store::{LedgerEntry, LedgerEvent, Part, PartId, PartMetadata, ProjectId};

use super::{App, AppEvents, errs::AppError, model::PanelContent, view::CreateMode};

impl App {
    pub(super) fn action_clone_part(&mut self) -> Result<AppEvents, AppError> {
        let item_id = self
            .get_active_panel_data()
            .item(self.view.get_active_panel_selection(), &self.store)
            .id
            .ok_or(AppError::PartHasNoId)?;
        let item = self
            .store
            .part_by_id(item_id.part_type())
            .ok_or(AppError::NoSuchObject(item_id.to_string()))?;
        let is_project = item
            .metadata
            .types
            .contains(&crate::store::ObjectType::Project);

        let mut new_item = item.clone();
        let new_id = self.make_new_type_id(&item.metadata.name);
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
                    part: PartId::clone(r.part()),
                    ev: LedgerEvent::RequireInProject(ProjectId::clone(&new_id.as_ref().into())),
                };
                self.store.record_event(&entry)?;
                self.store.update_count_cache(&entry);
            }
        }

        Ok(AppEvents::ReloadDataSelectByPartId(
            new_id.into(),
            new_name.clone(),
        ))
    }

    fn finish_create_part_w_label(&mut self) -> anyhow::Result<AppEvents> {
        let action_desc = self
            .get_active_panel_data()
            .actionable_objects(self.view.get_active_panel_selection(), &self.store)
            .ok_or(AppError::BadOperationContext)?;

        if let CreateMode::Hint(hint) = self.view.create_idx {
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

            return Ok(AppEvents::ReloadDataSelectByPartId(
                part_id,
                self.view.create_name.to_string(),
            ));
        }

        Ok(AppEvents::Nop)
    }

    fn finish_create_source(&mut self) -> anyhow::Result<AppEvents> {
        if let CreateMode::Hint(hint) = self.view.create_idx {
            Ok(match &self.view.create_hints[hint].id {
                Some(id) => AppEvents::ReloadDataSelectByPartId(
                    id.clone(),
                    self.view.create_hints[hint].name.clone(),
                ),
                None => AppEvents::Redraw,
            })
        } else {
            // Enter on summary or name fields
            let part_id = self.create_object_from_dialog_data(|part| {
                part.metadata.types.insert(crate::store::ObjectType::Source);
            })?;

            self.update_status(&format!("Source {} was created.", part_id));
            Ok(AppEvents::ReloadDataSelectByPartId(
                part_id,
                self.view.create_name.to_string(),
            ))
        }
    }

    fn finish_create_project(&mut self) -> anyhow::Result<AppEvents> {
        if let CreateMode::Hint(hint) = self.view.create_idx {
            Ok(match &self.view.create_hints[hint].id {
                Some(id) => AppEvents::ReloadDataSelectByPartId(
                    id.clone(),
                    self.view.create_hints[hint].name.clone(),
                ),
                None => AppEvents::Redraw,
            })
        } else {
            // Enter on summary or name fields
            let part_id = self.create_object_from_dialog_data(|part| {
                part.metadata.types.insert(crate::store::ObjectType::Part);
                part.metadata
                    .types
                    .insert(crate::store::ObjectType::Project);
            })?;

            self.update_status(&format!("Project {} was created.", part_id));
            Ok(AppEvents::ReloadDataSelectByPartId(
                part_id,
                self.view.create_name.to_string(),
            ))
        }
    }

    fn finish_create_label(&mut self) -> anyhow::Result<AppEvents> {
        if let CreateMode::Hint(hint) = self.view.create_idx {
            Ok(match &self.view.create_hints[hint].id {
                Some(id) => AppEvents::ReloadDataSelectByName(id.part_type().to_string()),
                None => AppEvents::Redraw,
            })
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
            self.store.add_label(label_key, &name);
            Ok(AppEvents::ReloadDataSelectByName(name))
        }
    }

    pub(super) fn finish_create(&mut self) -> anyhow::Result<AppEvents> {
        self.view.hide_create_dialog();

        if let CreateMode::Hint(_) = self.view.create_idx {
            // NOP, process hints later
        } else if self.view.create_name.to_string().trim().is_empty() {
            self.update_status("Name cannot be empty. Nothing was created.");
            return Ok(AppEvents::Redraw);
        } else if self
            .view
            .create_hints
            .first()
            .map(|hint| {
                hint.name.to_lowercase() == self.view.create_name.to_string().trim().to_lowercase()
            })
            .unwrap_or(false)
        {
            self.view.create_idx = CreateMode::Hint(0);
        }

        // This was an edit of existing part, just update it and return
        if let Some(part_id) = self.view.create_save_into.clone() {
            if let Some(part) = self.store.part_by_id(part_id.part_type()) {
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
            PanelContent::None => Ok(AppEvents::Redraw),
            PanelContent::TypeSelection => todo!(),
            PanelContent::Parts => self.finish_create_part(),
            PanelContent::Locations => self.finish_create_location(),
            PanelContent::LocationOfParts => self.finish_create_location_for_part(),
            PanelContent::PartsInLocation => self.finish_create_part_in_location(),
            PanelContent::LabelKeys => self.finish_create_label_key(),
            PanelContent::Labels => self.finish_create_label(),
            PanelContent::PartsWithLabels => self.finish_create_part_w_label(),
            PanelContent::Sources => self.finish_create_source(),
            PanelContent::PartsFromSources => self.finish_create_part_in_source(),
            PanelContent::PartsInOrders => self.finish_create_part_in_source(),
            PanelContent::Projects => self.finish_create_project(),
            PanelContent::PartsInProjects => self.finish_create_part_in_project(),
        }
    }

    fn finish_create_part(&mut self) -> anyhow::Result<AppEvents> {
        if let CreateMode::Hint(hint) = self.view.create_idx {
            Ok(match &self.view.create_hints[hint].id {
                Some(id) => AppEvents::ReloadDataSelectByPartId(
                    id.clone(),
                    self.view.create_hints[hint].name.clone(),
                ),
                None => AppEvents::Redraw,
            })
        } else {
            // Enter on summary or name fields
            let part_id = self.create_object_from_dialog_data(|part| {
                part.metadata.types.insert(crate::store::ObjectType::Part);
            })?;

            self.update_status(&format!("Part {} was created.", part_id));
            Ok(AppEvents::ReloadDataSelectByPartId(
                part_id,
                self.view.create_name.to_string(),
            ))
        }
    }

    fn finish_create_location(&mut self) -> anyhow::Result<AppEvents> {
        if let CreateMode::Hint(hint) = self.view.create_idx {
            Ok(match &self.view.create_hints[hint].id {
                Some(id) => AppEvents::ReloadDataSelectByPartId(
                    id.clone(),
                    self.view.create_hints[hint].name.clone(),
                ),
                None => AppEvents::Redraw,
            })
        } else {
            // Enter on summary or name fields
            let part_id = self.create_object_from_dialog_data(|part| {
                part.metadata
                    .types
                    .insert(crate::store::ObjectType::Location);
            })?;

            self.update_status(&format!("Location {} was created.", part_id));
            Ok(AppEvents::ReloadDataSelectByPartId(
                part_id,
                self.view.create_name.to_string(),
            ))
        }
    }

    fn finish_create_part_in_location(&mut self) -> anyhow::Result<AppEvents> {
        let action_desc = self
            .get_active_panel_data()
            .actionable_objects(self.view.get_active_panel_selection(), &self.store)
            .ok_or(AppError::BadOperationContext)?;

        if let CreateMode::Hint(hint) = self.view.create_idx {
            if let Some(part_id) = &self.view.create_hints[hint].id {
                let location = action_desc
                    .location()
                    .ok_or(AppError::BadOperationContext)?;

                self.store.show_empty_in_location(part_id, location, true);
                return Ok(AppEvents::ReloadDataSelectByPartId(
                    part_id.clone(),
                    self.view.create_hints[hint].name.clone(),
                ));
            }
        } else {
            // Enter on summary or name fields
            let part_id = self.create_object_from_dialog_data(|part| {
                part.metadata.types.insert(crate::store::ObjectType::Part);
            })?;

            if let Some(location) = action_desc.location() {
                self.store.show_empty_in_location(&part_id, location, true);
            }

            return Ok(AppEvents::ReloadDataSelectByPartId(
                part_id,
                self.view.create_name.to_string(),
            ));
        }
        Ok(AppEvents::Nop)
    }

    fn finish_create_location_for_part(&mut self) -> anyhow::Result<AppEvents> {
        let action_desc = self
            .get_active_panel_data()
            .actionable_objects(self.view.get_active_panel_selection(), &self.store)
            .ok_or(AppError::BadOperationContext)?;

        if let CreateMode::Hint(hint) = self.view.create_idx {
            if let Some(location_id) = &self.view.create_hints[hint].id {
                let part_id = action_desc
                    .part()
                    .map(PartId::to_simple)
                    .ok_or(AppError::BadOperationContext)?;

                self.store
                    .show_empty_in_location(&part_id, location_id, true);
                return Ok(AppEvents::ReloadDataSelectByPartId(
                    location_id.clone(),
                    self.view.create_hints[hint].name.clone(),
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

            if let Some(part_id) = action_desc.part().map(PartId::to_simple) {
                self.store
                    .show_empty_in_location(&part_id, &location_id, true);
            }

            return Ok(AppEvents::ReloadDataSelectByPartId(
                location_id,
                self.view.create_name.to_string(),
            ));
        }
        Ok(AppEvents::Nop)
    }

    fn finish_create_part_in_source(&mut self) -> anyhow::Result<AppEvents> {
        let action_desc = self
            .get_active_panel_data()
            .actionable_objects(self.view.get_active_panel_selection(), &self.store)
            .ok_or(AppError::BadOperationContext)?;

        if let CreateMode::Hint(hint) = self.view.create_idx {
            if let Some(part_id) = &self.view.create_hints[hint].id {
                let source = action_desc.source().ok_or(AppError::BadOperationContext)?;
                self.store
                    .show_empty_in_source(part_id, &source.into(), true);
                return Ok(AppEvents::ReloadDataSelectByPartId(
                    part_id.clone(),
                    self.view.create_hints[hint].name.clone(),
                ));
            }
        } else {
            // Enter on summary or name fields
            let part_id = self.create_object_from_dialog_data(|part| {
                part.metadata.types.insert(crate::store::ObjectType::Part);
            })?;

            if let Some(source) = action_desc.source() {
                self.store
                    .show_empty_in_source(&part_id, &source.into(), true);
            }

            return Ok(AppEvents::ReloadDataSelectByPartId(
                part_id,
                self.view.create_name.to_string(),
            ));
        }
        Ok(AppEvents::Nop)
    }

    fn finish_create_part_in_project(&mut self) -> anyhow::Result<AppEvents> {
        let action_desc = self
            .get_active_panel_data()
            .actionable_objects(self.view.get_active_panel_selection(), &self.store)
            .ok_or(AppError::BadOperationContext)?;

        if let CreateMode::Hint(hint) = self.view.create_idx {
            if let Some(part_id) = &self.view.create_hints[hint].id {
                let project_id = action_desc.project().ok_or(AppError::BadOperationContext)?;

                self.store.show_empty_in_project(part_id, project_id, true);
                return Ok(AppEvents::ReloadDataSelectByPartId(
                    part_id.clone(),
                    self.view.create_hints[hint].name.clone(),
                ));
            }
        } else {
            // Enter on summary or name fields
            let part_id = self.create_object_from_dialog_data(|part| {
                part.metadata.types.insert(crate::store::ObjectType::Part);
            })?;

            if let Some(project_id) = action_desc.project() {
                self.store.show_empty_in_project(&part_id, project_id, true);
            }

            return Ok(AppEvents::ReloadDataSelectByPartId(
                part_id,
                self.view.create_name.to_string(),
            ));
        }
        Ok(AppEvents::Nop)
    }

    fn finish_create_label_key(&mut self) -> anyhow::Result<AppEvents> {
        if let CreateMode::Hint(hint) = self.view.create_idx {
            Ok(match &self.view.create_hints[hint].id {
                Some(id) => AppEvents::ReloadDataSelectByName(id.part_type().to_string()),
                None => AppEvents::Redraw,
            })
        } else {
            if self.view.create_name.value().trim().is_empty() {
                self.update_status("Label cannot be empty.");
                return Ok(AppEvents::Redraw);
            }

            let name = self.view.create_name.value().trim().to_string();
            self.store.add_label_key(&name);
            Ok(AppEvents::ReloadDataSelectByName(name))
        }
    }

    fn create_object_from_dialog_data(&mut self, editor: fn(&mut Part)) -> anyhow::Result<PartId> {
        let name = self.view.create_name.value().trim().to_string();
        let mut part = Part {
            id: self.make_new_type_id(&name),
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

        // TODO allow creating a Part with pieces tracking support
        Ok(part_id.into())
    }
}
