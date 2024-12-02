use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{
    errs::AppError,
    view::{ActivePanel, CreateMode, DialogState, Hot, ViewLayout},
    App, AppEvents,
};

impl App {
    // Function keys follow a common pattern
    // F1 - filter, info, view only action
    // F2 - quick name and summary change
    // F3 - view, details and reports
    // F4 - edit
    // F5 - non-destructive copy, typically a dual panel action
    // F6 - destructive move, typically a dual panel action
    // F7 - make something
    // F8 - destroy or remove something
    // F9 - update count or requirements
    // F10 - app menu
    // F11 - ?
    // F12 - save, exit
    fn handle_global_key_event(&mut self, key_event: KeyEvent) -> Result<AppEvents, AppError> {
        match key_event.code {
            KeyCode::Esc => {
                self.view.escape_keys = true;
                return Ok(AppEvents::Redraw);
            }
            KeyCode::F(2) => return self.press_f2(),

            KeyCode::F(3) => self.view.switch_full_split_layout(),
            KeyCode::F(4) => return self.press_f4(),

            KeyCode::F(5) => return self.press_f5(),
            KeyCode::F(6) => return self.press_f6(),

            KeyCode::F(7) => return self.press_f7(),
            KeyCode::F(8) => return self.press_f8(),

            KeyCode::F(9) if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                return self.press_ctrl_f9()
            }
            KeyCode::F(9) => return self.press_f9(),

            KeyCode::F(12) => return Ok(AppEvents::Quit),
            KeyCode::Down => self
                .view
                .move_down(self.get_active_panel_data().len(&self.store)),
            KeyCode::Up => self.view.move_up(),
            KeyCode::PageDown => {
                for _i in 0..10 {
                    self.view
                        .move_down(self.get_active_panel_data().len(&self.store));
                }
            }
            KeyCode::PageUp => {
                for _i in 0..10 {
                    self.view.move_up();
                }
            }
            KeyCode::Right => {
                if self.view.layout == ViewLayout::Split {
                    self.view.active = ActivePanel::PanelB
                } else if self.view.layout == ViewLayout::Wide {
                    // NOP
                } else {
                    self.view.active_info = self.view.active == ActivePanel::PanelA;
                }
            }
            KeyCode::Left => {
                if self.view.layout == ViewLayout::Split {
                    self.view.active = ActivePanel::PanelA
                } else if self.view.layout == ViewLayout::Wide {
                    // NOP
                } else {
                    self.view.active_info = self.view.active == ActivePanel::PanelB;
                }
            }
            KeyCode::Home => self.view.scroll_to(0),
            KeyCode::End => self
                .view
                .scroll_to(self.get_active_panel_data().len(&self.store) - 1),
            KeyCode::Tab => self.view.switch_active_panel(),
            KeyCode::Enter => return Ok(self.press_enter()),
            KeyCode::F(1) | KeyCode::Char('/') => self.open_filter_dialog(),
            KeyCode::Char(c) => {
                let val = self
                    .view
                    .panel_quick_select_event(tui_input::InputRequest::InsertChar(c))
                    .to_string();
                self.select_item(val.as_str());
            }
            _ => {}
        }

        Ok(AppEvents::Redraw)
    }

    fn escape_key(&self, key_event: KeyEvent) -> KeyEvent {
        match key_event.code {
            KeyCode::Char('0') => KeyEvent {
                code: KeyCode::F(10),
                ..key_event
            },
            KeyCode::Char('1') => KeyEvent {
                code: KeyCode::F(1),
                ..key_event
            },
            KeyCode::Char('2') => KeyEvent {
                code: KeyCode::F(2),
                ..key_event
            },
            KeyCode::Char('3') => KeyEvent {
                code: KeyCode::F(3),
                ..key_event
            },
            KeyCode::Char('4') => KeyEvent {
                code: KeyCode::F(4),
                ..key_event
            },
            KeyCode::Char('5') => KeyEvent {
                code: KeyCode::F(5),
                ..key_event
            },
            KeyCode::Char('6') => KeyEvent {
                code: KeyCode::F(6),
                ..key_event
            },
            KeyCode::Char('7') => KeyEvent {
                code: KeyCode::F(7),
                ..key_event
            },
            KeyCode::Char('8') => KeyEvent {
                code: KeyCode::F(8),
                ..key_event
            },
            KeyCode::Char('9') => KeyEvent {
                code: KeyCode::F(9),
                ..key_event
            },
            KeyCode::Char('q') => KeyEvent {
                code: KeyCode::F(12),
                ..key_event
            },
            KeyCode::Esc => KeyEvent {
                code: KeyCode::Menu,
                ..key_event
            },
            _ => key_event,
        }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> anyhow::Result<AppEvents> {
        let key_event = if self.view.escape_keys {
            self.escape_key(key_event)
        } else {
            key_event
        };

        self.view.escape_keys = false;

        match self.view.hot() {
            Hot::ActionCountDialog => match key_event.code {
                KeyCode::Up => self.view.action_dialog_count_up(),
                KeyCode::Down => self.view.action_dialog_count_down(),
                KeyCode::Left => (),
                KeyCode::Right => (),
                KeyCode::Char(n) if "0123456789".contains(n) => {
                    self.view.action_dialog_count_set(n)
                }
                KeyCode::Tab => (),
                KeyCode::Enter => return self.finish_action(),
                KeyCode::Esc => self.view.hide_action_dialog(),
                KeyCode::Backspace => self.view.action_dialog_count_backspace(),
                KeyCode::Delete => self.view.action_count_dialog_count = 0,
                _ => {}
            },
            Hot::CreatePartDialog => {
                let field = if self.view.create_idx == CreateMode::Name {
                    &mut self.view.create_name
                } else {
                    &mut self.view.create_summary
                };

                match key_event.code {
                    KeyCode::Esc => self.view.hide_create_dialog(),
                    KeyCode::Enter => {
                        return self.finish_create();
                    }
                    KeyCode::Char(c) => {
                        field.handle(tui_input::InputRequest::InsertChar(c));
                        self.update_create_dialog_hints();
                    }
                    KeyCode::Left => {
                        field.handle(tui_input::InputRequest::GoToPrevChar);
                        self.update_create_dialog_hints();
                    }
                    KeyCode::Right => {
                        field.handle(tui_input::InputRequest::GoToNextChar);
                        self.update_create_dialog_hints();
                    }
                    KeyCode::Backspace => {
                        field.handle(tui_input::InputRequest::DeletePrevChar);
                        self.update_create_dialog_hints();
                    }
                    KeyCode::Delete => {
                        field.handle(tui_input::InputRequest::DeleteNextChar);
                        self.update_create_dialog_hints();
                    }
                    KeyCode::Home => {
                        field.handle(tui_input::InputRequest::GoToStart);
                    }
                    KeyCode::End => {
                        field.handle(tui_input::InputRequest::GoToEnd);
                    }
                    KeyCode::Down | KeyCode::Tab => {
                        self.view.create_idx = self.view.create_idx.next();
                        if let CreateMode::Hint(h) = self.view.create_idx {
                            if self.view.create_hints.is_empty() {
                                self.view.create_idx = CreateMode::Summary;
                            } else if h >= self.view.create_hints.len() {
                                self.view.create_idx =
                                    CreateMode::Hint(self.view.create_hints.len() - 1);
                            }
                        }
                    }
                    KeyCode::Up => self.view.create_idx = self.view.create_idx.prev(),
                    _ => {}
                }
            }
            Hot::DeleteDialog => match key_event.code {
                KeyCode::Esc => self.view.hide_delete_dialog(),
                KeyCode::Enter => return self.finish_delete(),
                _ => {}
            },
            Hot::AlertDialog => match key_event.code {
                KeyCode::Esc => self.view.hide_alert_dialog(),
                KeyCode::Enter => self.view.hide_alert_dialog(),
                _ => {}
            },
            Hot::PanelInfo => match key_event.code {
                KeyCode::F(2)
                | KeyCode::F(3)
                | KeyCode::F(4)
                | KeyCode::F(8)
                | KeyCode::F(9)
                | KeyCode::F(12)
                | KeyCode::Down
                | KeyCode::Up
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::PageUp
                | KeyCode::PageDown => return Ok(self.handle_global_key_event(key_event)?),
                _ => (),
            },
            Hot::PanelQuickSelect => match key_event.code {
                KeyCode::Esc => {
                    self.view.active_quick_select = false;
                    let return_to = self.view.active_search_return_idx;
                    self.view.update_active_panel(|p| p.selected = return_to);
                }
                KeyCode::Enter => {
                    self.view.active_quick_select = false;
                }
                KeyCode::Char(c) => {
                    let val = self
                        .view
                        .panel_quick_select_event(tui_input::InputRequest::InsertChar(c))
                        .to_string();
                    self.select_item(val.as_str());
                }
                KeyCode::Left => {
                    self.view
                        .panel_quick_select_event(tui_input::InputRequest::GoToPrevChar);
                }
                KeyCode::Right => {
                    self.view
                        .panel_quick_select_event(tui_input::InputRequest::GoToNextChar);
                }
                KeyCode::Backspace => {
                    let val = self
                        .view
                        .panel_quick_select_event(tui_input::InputRequest::DeletePrevChar)
                        .to_string();
                    self.select_item(val.as_str());
                }
                KeyCode::Delete => {
                    let val = self
                        .view
                        .panel_quick_select_event(tui_input::InputRequest::DeleteNextChar)
                        .to_string();
                    self.select_item(val.as_str());
                }
                KeyCode::Home => {
                    self.view
                        .panel_quick_select_event(tui_input::InputRequest::GoToStart);
                }
                KeyCode::End => {
                    self.view
                        .panel_quick_select_event(tui_input::InputRequest::GoToEnd);
                }
                KeyCode::F(_)
                | KeyCode::Down
                | KeyCode::Up
                | KeyCode::PageDown
                | KeyCode::PageUp => return Ok(self.handle_global_key_event(key_event)?),
                _ => {}
            },
            Hot::FilterDialog => match key_event.code {
                KeyCode::Esc => {
                    self.view.filter_dialog = DialogState::Hidden;
                }
                KeyCode::Enter => {
                    return Ok(self.perform_filter());
                }
                KeyCode::F(12) => {
                    self.view.filter_query.reset();
                    return Ok(self.perform_filter());
                }
                KeyCode::Char(c) => {
                    self.view
                        .filter_query
                        .handle(tui_input::InputRequest::InsertChar(c));
                }
                KeyCode::Left => {
                    self.view
                        .filter_query
                        .handle(tui_input::InputRequest::GoToPrevChar);
                }
                KeyCode::Right => {
                    self.view
                        .filter_query
                        .handle(tui_input::InputRequest::GoToNextChar);
                }
                KeyCode::Backspace => {
                    self.view
                        .filter_query
                        .handle(tui_input::InputRequest::DeletePrevChar);
                }
                KeyCode::Delete => {
                    self.view
                        .filter_query
                        .handle(tui_input::InputRequest::DeleteNextChar);
                }
                KeyCode::Home => {
                    self.view
                        .filter_query
                        .handle(tui_input::InputRequest::GoToStart);
                }
                KeyCode::End => {
                    self.view
                        .filter_query
                        .handle(tui_input::InputRequest::GoToEnd);
                }
                _ => {}
            },
            _ => return Ok(self.handle_global_key_event(key_event)?),
        }

        Ok(AppEvents::Redraw)
    }
}
