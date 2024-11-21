use std::default;

use tui_input::{Input, InputRequest};

use crate::store::PartId;

use super::{model::PanelItem, ActionVariant};

#[derive(Debug, Default)]
pub struct View {
    pub(super) layout: ViewLayout,
    pub(super) active: ActivePanel,
    // Focus the info panel in info layout
    pub(super) active_info: bool,
    pub(super) active_search: bool,
    pub(super) active_search_input: Input,
    pub(super) active_search_return_idx: usize,
    pub(super) info_scroll: usize,
    pub(super) panel_a: PanelState,
    pub(super) panel_b: PanelState,
    pub(super) action_count_dialog: DialogState,
    pub(super) action_count_dialog_action: ActionVariant,
    pub(super) action_count_dialog_count: usize,
    pub(super) action_count_dialog_typing: bool,
    pub(super) status: String,
    pub(super) create_dialog: DialogState,
    pub(super) delete_dialog: DialogState,
    pub(super) delete_item: Option<PanelItem>,
    pub(super) delete_from: String,
    pub(super) alert_dialog: DialogState,
    pub(super) create_idx: CreateMode,
    pub(super) create_hints: Vec<PanelItem>,
    pub(super) create_name: Input,
    pub(super) create_summary: Input,
    pub(super) create_save_into: Option<PartId>,
    pub(crate) alert_title: String,
    pub(crate) alert_text: String,
}

impl View {
    pub fn hot(&self) -> Hot {
        if self.alert_dialog == DialogState::VISIBLE {
            return Hot::AlertDialog;
        }

        if self.delete_dialog == DialogState::VISIBLE {
            return Hot::DeleteDialog;
        }

        if self.create_dialog == DialogState::VISIBLE {
            return Hot::CreatePartDialog;
        }

        if self.action_count_dialog == DialogState::VISIBLE {
            return Hot::ActionCountDialog;
        }

        if self.active_search {
            return Hot::PanelSearch;
        }

        if self.active_info {
            return Hot::PanelInfo;
        }

        return match self.active {
            ActivePanel::PanelA => Hot::PanelA,
            ActivePanel::PanelB => Hot::PanelB,
        };
    }

    pub fn update_active_panel(&mut self, cb: impl Fn(&mut PanelState)) {
        match self.active {
            ActivePanel::PanelA => cb(&mut self.panel_a),
            ActivePanel::PanelB => cb(&mut self.panel_b),
        }
    }

    // Disables info view, search and other "pop-up" and quick edit actions
    pub fn cancel_all(&mut self) {
        self.active_info = false;
        self.info_scroll = 0;
        self.active_search = false;
    }

    // Disables quick actions like search
    pub fn cancel(&mut self) {
        self.active_search = false;
    }

    pub fn switch_active_panel(&mut self) {
        self.cancel_all();

        self.active = match self.active {
            ActivePanel::PanelA => ActivePanel::PanelB,
            ActivePanel::PanelB => ActivePanel::PanelA,
        }
    }

    pub fn switch_full_split_layout(&mut self) {
        self.cancel_all();

        self.layout = match self.layout {
            ViewLayout::SPLIT => ViewLayout::INFO,
            ViewLayout::INFO => ViewLayout::WIDE,
            ViewLayout::WIDE => ViewLayout::SPLIT,
        }
    }

    pub(crate) fn move_down(&mut self, size: usize) {
        if self.active_info {
            self.info_scroll += 1;
            return;
        }

        self.cancel_all();
        self.update_active_panel(|panel| {
            if panel.selected < size - 1 {
                panel.selected = panel.selected.saturating_add(1);
            }
        });
    }

    pub(crate) fn move_up(&mut self) {
        if self.active_info {
            self.info_scroll = self.info_scroll.saturating_sub(1);
            return;
        }

        self.cancel_all();
        self.update_active_panel(|panel| panel.selected = panel.selected.saturating_sub(1));
    }

    pub(crate) fn get_active_panel_selection(&self) -> usize {
        match self.active {
            ActivePanel::PanelA => self.panel_a.selected,
            ActivePanel::PanelB => self.panel_b.selected,
        }
    }

    pub(crate) fn get_inactive_panel_selection(&self) -> usize {
        match self.active {
            ActivePanel::PanelA => self.panel_b.selected,
            ActivePanel::PanelB => self.panel_a.selected,
        }
    }

    pub(crate) fn show_action_dialog(&mut self, action: ActionVariant, count: usize) {
        self.cancel();
        self.action_count_dialog = DialogState::VISIBLE;
        self.action_count_dialog_action = action;
        self.action_count_dialog_count = count;
        self.action_count_dialog_typing = false;
    }

    pub(crate) fn hide_action_dialog(&mut self) {
        self.action_count_dialog = DialogState::HIDDEN;
    }

    pub(crate) fn hide_create_dialog(&mut self) {
        self.create_dialog = DialogState::HIDDEN;
    }

    pub(crate) fn action_dialog_count_up(&mut self) {
        if !self.action_count_dialog_action.countable() {
            return;
        }

        self.action_count_dialog_count += 1;
        self.action_count_dialog_typing = false;
    }

    pub(crate) fn action_dialog_count_down(&mut self) {
        if !self.action_count_dialog_action.countable() {
            return;
        }

        self.action_count_dialog_count = self.action_count_dialog_count.saturating_sub(1);
        self.action_count_dialog_typing = false;
    }

    pub(crate) fn action_dialog_count_set(&mut self, n: char) {
        if !self.action_count_dialog_action.countable() {
            return;
        }

        if !self.action_count_dialog_typing {
            self.action_count_dialog_count = 0;
            self.action_count_dialog_typing = true;
        }

        self.action_count_dialog_count *= 10;
        self.action_count_dialog_count += n.to_digit(10).unwrap_or(0) as usize;
    }

    pub(crate) fn action_dialog_count_backspace(&mut self) {
        if !self.action_count_dialog_action.countable() {
            self.hide_action_dialog();
        }

        self.action_count_dialog_count /= 10;
    }

    pub(crate) fn hide_delete_dialog(&mut self) {
        self.delete_dialog = DialogState::HIDDEN;
    }

    pub(crate) fn hide_alert_dialog(&mut self) {
        self.alert_dialog = DialogState::HIDDEN;
    }

    pub(crate) fn scroll_to(&mut self, arg: usize) {
        self.info_scroll = 0;

        match self.active {
            ActivePanel::PanelA => self.panel_a.selected = arg,
            ActivePanel::PanelB => self.panel_b.selected = arg,
        }
    }

    pub(crate) fn panel_search_event(&mut self, r: InputRequest) -> &str {
        if !self.active_search {
            self.active_search_input.reset();
            self.active_search_return_idx = self.get_active_panel_selection();
        }

        self.active_search = true;
        self.active_search_input.handle(r);
        self.active_search_input.value()
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum DialogState {
    #[default]
    HIDDEN,
    VISIBLE,
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum ViewLayout {
    #[default]
    SPLIT,
    WIDE,
    INFO,
}

impl ViewLayout {
    pub fn is_dual_panel(&self) -> bool {
        *self == Self::SPLIT
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum ActivePanel {
    #[default]
    PanelA,
    PanelB,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Hot {
    PanelA,
    PanelB,
    PanelInfo,
    PanelSearch,
    ActionCountDialog,
    CreatePartDialog,
    AlertDialog,
    DeleteDialog,
}

#[derive(Debug, Default)]
pub struct PanelState {
    pub(super) selected: usize,
}

#[derive(Debug, Default, PartialEq, PartialOrd)]
pub enum CreateMode {
    #[default]
    NAME,
    SUMMARY,
    HINT(usize),
}

impl CreateMode {
    pub fn next(&self) -> Self {
        match self {
            CreateMode::NAME => CreateMode::SUMMARY,
            CreateMode::SUMMARY => CreateMode::HINT(0),
            CreateMode::HINT(n) => CreateMode::HINT(n + 1),
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            CreateMode::NAME => CreateMode::NAME,
            CreateMode::SUMMARY => CreateMode::NAME,
            CreateMode::HINT(n) if *n == 0 => CreateMode::SUMMARY,
            CreateMode::HINT(n) => CreateMode::HINT(n - 1),
        }
    }
}
