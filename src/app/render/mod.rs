use itertools::Itertools;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::symbols::border;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, Clear, Padding, Paragraph, Row, Scrollbar, ScrollbarState,
    StatefulWidget, Table, TableState, Widget, Wrap,
};
use render_icons::DrawFixed6x3Icon;
use tui_big_text::{BigText, PixelSize};

use super::kbd::EscMode;
use super::model::PanelContent::{
    self, LabelKeys, Labels, LocationOfParts, Locations, Parts, PartsFromSources, PartsInLocation,
    PartsInOrders, PartsInProjects, PartsWithLabels, Projects, Sources, TypeSelection,
};
use super::model::PanelData;
use super::view::{ActivePanel, CreateMode, DialogState, Hot, PanelState, ViewLayout};
use super::App;

mod filter;
mod render_icons;

// Convert F-key into its array index
#[allow(non_snake_case)]
fn Fi(f: usize) -> usize {
    f - 1
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let full_area = area;

        let layout = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

        let layout_header = layout[0];
        let layout_panels = layout[1];

        let layout_status =
            Layout::horizontal([Constraint::Min(1), Constraint::Length(6)]).split(layout[2]);

        let layout_fkeys_low = layout[3];
        let layout_fkeys_high = layout[4];

        let (layout_panel_a, layout_panel_b, layout_info) = if self.view.layout == ViewLayout::Split
        {
            let layout =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(layout_panels);
            (Some(layout[0]), Some(layout[1]), None)
        } else if self.view.layout == ViewLayout::Wide {
            match self.view.active {
                super::view::ActivePanel::PanelA => (Some(layout_panels), None, None),
                super::view::ActivePanel::PanelB => (None, Some(layout_panels), None),
            }
        } else {
            // INFO
            let layout =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(layout_panels);

            match self.view.active {
                super::view::ActivePanel::PanelA => (Some(layout[0]), None, Some(layout[1])),
                super::view::ActivePanel::PanelB => (None, Some(layout[1]), Some(layout[0])),
            }
        };

        let layout_fkeys_low = Layout::horizontal([
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
        ])
        .split(layout_fkeys_low);

        let layout_fkeys_high = Layout::horizontal([
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
        ])
        .split(layout_fkeys_high);

        if let Some(area) = layout_panel_a {
            let panel_title = self.model.panel_a.panel_title(&self.store);
            self.render_panel(
                format!(" {} ", panel_title).as_str(),
                &self.view.panel_a,
                self.model.panel_a.as_ref(),
                self.view.hot() == Hot::PanelA,
                self.view.active == ActivePanel::PanelA && self.view.active_quick_select,
                area,
                buf,
            );
        }

        if let Some(area) = layout_panel_b {
            let panel_title = self.model.panel_b.panel_title(&self.store);
            self.render_panel(
                format!(" {} ", panel_title).as_str(),
                &self.view.panel_b,
                self.model.panel_b.as_ref(),
                self.view.hot() == Hot::PanelB,
                self.view.active == ActivePanel::PanelB && self.view.active_quick_select,
                area,
                buf,
            );
        }

        if let Some(area) = layout_info {
            // Render info
            Clear.render(area, buf);

            let item = self
                .get_active_panel_data()
                .item(self.view.get_active_panel_selection(), &self.store);
            if let Some(item_id) = item.id {
                if let Some(part) = self.store.part_by_id(&item_id) {
                    let mut content: Vec<Line> = vec![];
                    content.push(format!("id: {}", part.id).into());
                    content.push(format!("name: {}", part.metadata.name).into());
                    content.push(part.metadata.summary.to_string().into());
                    content.push("".into());
                    for l in &part.metadata.labels {
                        for v in l.1 {
                            content.push(format!("{}: {}", l.0, v).into());
                        }
                    }
                    content.push("".into());

                    // TODO nicer parser for Markdown
                    for l in part.content.split('\n') {
                        content.push(l.into());
                    }

                    content.push("".into());
                    content.push("--".into());
                    content.push(format!("path: {:?}", part.filename).into());

                    let block = Block::bordered()
                        .title(part.metadata.name.as_str())
                        .title_bottom(
                            part.metadata
                                .types
                                .iter()
                                .map(|v| format!("{:?}", v))
                                .join(", "),
                        )
                        .border_set(border::PLAIN);

                    let block = if self.view.active_info {
                        block.border_style(Color::Yellow)
                    } else {
                        block
                    };

                    let inner = block.inner(area);
                    block.render(area, buf);

                    let cols = Layout::horizontal([Constraint::Min(1), Constraint::Length(1)])
                        .split(inner);

                    let mut scrollbar_state =
                        ScrollbarState::new(content.len()).position(self.view.info_scroll);
                    let scrollbar =
                        Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight);
                    let scrollbar = if self.view.active_info {
                        scrollbar.style(Style::new().yellow())
                    } else {
                        scrollbar
                    };

                    scrollbar.render(cols[1], buf, &mut scrollbar_state);

                    Paragraph::new(content)
                        .wrap(Wrap { trim: false })
                        .scroll((self.view.info_scroll as u16, 0))
                        .render(cols[0], buf);
                }
            }
        }

        let item_actionable = self
            .get_active_panel_data()
            .item_actionable(self.view.get_active_panel_selection());

        let s_edit_action = if self.view.escape_keys == EscMode::FKeysControl {
            self.ctrl_f9_action()
        } else {
            self.f9_action()
        };

        let s_copy_action = self.f5_action();
        let s_move_action = self.f6_action();
        let s_del_action = self.f8_action();

        let actions = [
            "filter",
            "rename",
            "view",
            "edit",
            s_copy_action.name(),
            s_move_action.name(),
            "make",
            s_del_action.name(),
            s_edit_action.name(),
            "", // menu
            "", // save
            "quit",
        ];

        let mut action_style = [Style::new(); 12];

        for idx in Fi(2)..=Fi(6) {
            if actions[idx].is_empty() || !item_actionable {
                action_style[idx] = action_style[idx].not_bold().dim().dark_gray()
            }
        }

        for idx in Fi(8)..=Fi(9) {
            if actions[idx].is_empty() || !item_actionable {
                action_style[idx] = action_style[idx].dim().dark_gray()
            }
        }

        for idx in Fi(10)..=Fi(12) {
            if actions[idx].is_empty() {
                action_style[idx] = action_style[idx].dim().dark_gray()
            }
        }

        match self.get_active_panel_data().filter_status() {
            super::model::FilterStatus::NotSupported => {
                action_style[Fi(1)] = action_style[Fi(1)].dim().dark_gray();
            }
            super::model::FilterStatus::NotApplied => (),
            super::model::FilterStatus::Query(_) => {
                action_style[Fi(1)] = action_style[Fi(1)].on_green();
            }
        }

        if !self.view.layout.is_dual_panel() && s_copy_action.dual_panel() || !item_actionable {
            action_style[Fi(5)] = action_style[Fi(5)].dim().dark_gray();
        }

        if !self.view.layout.is_dual_panel() && s_move_action.dual_panel() || !item_actionable {
            action_style[Fi(6)] = action_style[Fi(6)].dim().dark_gray();
        }

        if !self.get_active_panel_data().data_type().can_make() {
            action_style[Fi(7)] = action_style[Fi(7)].dim().dark_gray();
        };

        if !self.get_active_panel_data().data_type().can_delete() || !item_actionable {
            action_style[Fi(8)] = action_style[Fi(8)].dim().dark_gray();
        };

        if !self.view.layout.is_dual_panel() && s_edit_action.dual_panel() {
            action_style[Fi(9)] = action_style[Fi(9)].dim().dark_gray();
        }

        let f_names: Vec<String> = if self.view.escape_keys == EscMode::None {
            (1..=12).map(|i| format!("F{}", i)).collect()
        } else {
            (1..=9)
                .map(|i| format!(" {}", i))
                .chain([" 0 ", "", " Q "].map(String::from))
                .collect()
        };

        for idx in Fi(1)..=Fi(6) {
            Line::from(vec![
                f_names[idx].as_str().bold(),
                " ".into(),
                actions[idx].into(),
                " ".into(),
            ])
            .patch_style(action_style[idx])
            .render(layout_fkeys_low[idx], buf);
        }

        for idx in Fi(7)..=Fi(12) {
            Line::from(vec![
                f_names[idx].as_str().bold(),
                " ".into(),
                actions[idx].into(),
                " ".into(),
            ])
            .patch_style(action_style[idx])
            .render(layout_fkeys_high[idx - Fi(7)], buf);
        }

        let header_text = match self.view.active {
            ActivePanel::PanelA => self.model.panel_a.title(&self.store),
            ActivePanel::PanelB => self.model.panel_b.title(&self.store),
        };

        Paragraph::new(Line::from(vec!["[diiLo] ".into(), header_text.into()]))
            .on_dark_gray()
            .gray()
            .render(layout_header, buf);

        Paragraph::new(self.view.status.as_str())
            .on_dark_gray()
            .gray()
            .render(layout_status[0], buf);

        let esc_flags = match self.view.escape_keys {
            EscMode::None => "",
            EscMode::Fkeys => "[Fx]",
            EscMode::FKeysControl => "[C-Fx]",
        };

        Paragraph::new(esc_flags)
            .on_dark_gray()
            .light_yellow()
            .bold()
            .right_aligned()
            .render(layout_status[1], buf);

        if self.view.action_count_dialog == DialogState::Visible {
            self.action_count_dialog(full_area, buf);
        }

        if self.view.create_dialog == DialogState::Visible {
            self.create_dialog(full_area, buf);
        }

        if self.view.delete_dialog == DialogState::Visible {
            let item = self.view.delete_item.clone().unwrap();
            self.alert_dialog(
                full_area,
                buf,
                "Delete?",
                vec![
                    Line::from(vec![item.name.bold(), " ".into(), item.summary.black()]),
                    Line::from("from"),
                    Line::from(self.view.delete_from.clone()),
                ],
            );
        }

        if self.view.filter_dialog == DialogState::Visible {
            self.filter_dialog(full_area, buf);
        }

        if self.view.alert_dialog == DialogState::Visible {
            self.alert_dialog(
                full_area,
                buf,
                &self.view.alert_title,
                self.view.alert_text.as_str(),
            );
        }
    }
}

impl App {
    #[allow(clippy::too_many_arguments)]
    fn render_panel(
        &self,
        name: &str,
        panel: &PanelState,
        content: &dyn PanelData,
        active: bool,
        search_active: bool,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let title_style = if active {
            Style::new().bold()
        } else {
            Style::new()
        };

        let panel_content = content.items(&self.store);

        let panel_area = Layout::vertical([Constraint::Min(1), Constraint::Length(3)]).split(area);

        let block = Block::new()
            .borders(Borders::BOTTOM.complement()) // All except bottom
            .title(name)
            .title_style(title_style)
            .border_set(border::PLAIN);

        let block = if active {
            block.border_style(Color::Yellow)
        } else {
            block
        };

        let block_inner_area = block.inner(panel_area[0]);
        block.render(panel_area[0], buf);

        let summary_block = Block::bordered()
            .border_type(BorderType::Plain)
            .title_style(title_style);
        let summary_block = if active || search_active {
            summary_block.border_style(Color::Yellow)
        } else {
            summary_block
        };

        if search_active {
            let input_width = summary_block.inner(panel_area[1]).width - 3; // keep 2 for borders and 1 for cursor

            // Emulate cursor
            let parts = emulate_cursor(
                self.view.active_search_input.cursor(),
                self.view.active_search_input.value(),
            );

            Paragraph::new(Line::from(parts))
                .gray()
                .scroll((
                    0,
                    self.view
                        .active_search_input
                        .visual_scroll(input_width as usize) as u16,
                ))
                .block(summary_block.title("search"))
                .render(panel_area[1], buf);
        } else {
            let summary_block = summary_block.title(format!(
                " {} / {} ",
                panel.selected + 1,
                panel_content.len()
            ));

            Paragraph::new(content.item_summary(panel.selected, &self.store))
                .block(summary_block)
                .render(panel_area[1], buf);
        }

        let panel_content_area = Layout::horizontal([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(block_inner_area);

        let mut table_state = TableState::new();
        table_state.select(Some(panel.selected));

        let table_select_style = if active {
            Style::new().on_yellow()
        } else {
            Style::new().on_dark_gray()
        };

        let cell_length = (panel_content_area[0].width - 1) as usize;

        let table = Table::new(
            content.items(&self.store).into_iter().map(|v| {
                let name_length = v.name.char_indices().count();
                let data_length = v.data.char_indices().count();
                let summary_length = v.summary.char_indices().count();
                let summary_length =
                    summary_length.min(cell_length.saturating_sub(name_length + 4 + data_length));
                let padding_length =
                    cell_length.saturating_sub(name_length + data_length + summary_length + 4);
                let padding = " ".repeat(padding_length);

                // Split according to UTF-8 character boundaries
                let summary_split = v
                    .summary
                    .char_indices()
                    .nth(summary_length)
                    .map_or_else(|| v.summary.len(), |(index, _)| index);

                let line = Line::from(vec![
                    v.name.into(),
                    "  ".dark_gray(),
                    v.summary[..summary_split].to_string().dark_gray(),
                    padding.into(),
                    "  ".dark_gray(),
                    v.data.into(),
                ]);

                Row::new(vec![Cell::new(line)])
            }),
            [Constraint::Fill(1)],
        )
        .row_highlight_style(table_select_style)
        .highlight_symbol(">");
        StatefulWidget::render(table, panel_content_area[0], buf, &mut table_state);

        let mut scrollbar_state = ScrollbarState::new(panel_content.len()).position(panel.selected);
        let scrollbar = Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight);
        let scrollbar = if active {
            scrollbar.style(Style::new().yellow())
        } else {
            scrollbar
        };

        scrollbar.render(panel_content_area[2], buf, &mut scrollbar_state);
    }

    fn action_count_dialog(&self, area: Rect, buf: &mut Buffer) {
        let area = Self::center(area, Constraint::Length(60), Constraint::Length(20));
        Clear.render(area, buf);

        let block = Block::bordered()
            .border_set(border::DOUBLE)
            .border_style(Color::Yellow)
            .padding(Padding::symmetric(2, 1))
            .title(format!(" {} ", self.view.action_count_dialog_action.name()))
            .title_bottom(" confirm by <Enter> / cancel by <ESC> ")
            .on_gray();

        let block_area = block.inner(area);
        block.render(area, buf);

        let block_area = Layout::vertical([
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Min(8),
        ])
        .split(block_area);

        let action_symbol = match self.view.action_count_dialog_action {
            super::ActionVariant::None => render_icons::EMPTY,
            super::ActionVariant::Error => todo!(),
            super::ActionVariant::AddLabel => render_icons::LABEL,
            super::ActionVariant::RemoveLabel => render_icons::LABEL_X,
            super::ActionVariant::CreatePart => todo!(),
            super::ActionVariant::ClonePart => todo!(),
            super::ActionVariant::RequirePart => render_icons::REQUIRE,
            super::ActionVariant::OrderPart => render_icons::ORDER,
            super::ActionVariant::MovePart => render_icons::MOVE,
            super::ActionVariant::DeliverPart => render_icons::TRUCK,
            super::ActionVariant::SolderPart => render_icons::SOLDER,
            super::ActionVariant::UnsolderPart => render_icons::UNSOLDER,
            super::ActionVariant::OrderPartLocal => render_icons::ORDER,
            super::ActionVariant::RequirePartInLocationLocal => render_icons::REQUIRE,
            super::ActionVariant::RequirePartInProjectLocal => render_icons::REQUIRE,
            super::ActionVariant::Delete => render_icons::DELETE,
            super::ActionVariant::ForceCount => render_icons::FORCE_COUNT,
            super::ActionVariant::ForceCountLocal => render_icons::FORCE_COUNT,
        };

        if self.view.action_count_dialog_action.countable() {
            // Count area
            BigText::builder()
                .pixel_size(PixelSize::Full)
                .style(Style::new().blue())
                .alignment(Alignment::Center)
                .lines(vec![
                    format!("{}", self.view.action_count_dialog_count).into()
                ])
                .build()
                .render(block_area[2], buf);
        }

        let source_area =
            Layout::horizontal([Constraint::Length(8), Constraint::Min(8)]).split(block_area[0]);

        // Action icon
        DrawFixed6x3Icon::with_icon(action_symbol)
            .with_style(Style::new().blue())
            .render(source_area[0], buf);

        if let Some(obj) = self.view.action_count_dialog_source.as_ref() {
            Paragraph::new(vec![
                Line::from(vec![self
                    .view
                    .action_count_dialog_action
                    .description()
                    .blue()
                    .bold()]),
                Line::from(obj.name.as_str().black().bold()),
                Line::from(obj.summary.as_str().dark_gray()),
            ])
            .alignment(ratatui::layout::Alignment::Left)
            .render(source_area[1], buf);
        }

        if self.view.action_count_dialog_action.dual_panel() {
            let destination_area = Layout::horizontal([Constraint::Length(8), Constraint::Min(8)])
                .split(block_area[1]);

            if let Some(obj) = self.view.action_count_dialog_destination.as_ref() {
                Paragraph::new(vec![
                    Line::from(vec!["-> to ".blue().bold()]),
                    Line::from(obj.name.as_str().black().bold()),
                    Line::from(obj.summary.as_str().dark_gray()),
                ])
                .alignment(ratatui::layout::Alignment::Left)
                .render(destination_area[1], buf);
            }
        }
    }

    fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
        let [area] = Layout::horizontal([horizontal])
            .flex(Flex::Center)
            .areas(area);
        let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
        area
    }

    fn create_dialog(&self, area: Rect, buf: &mut Buffer) {
        let area = Self::center(area, Constraint::Length(60), Constraint::Length(20));
        Clear.render(area, buf);

        let title_prefix = if self.view.create_save_into.is_some() {
            "Update"
        } else {
            "Create"
        };

        let title_suffix = match self.get_active_panel_data().data_type() {
            Parts | PartsInLocation | PartsWithLabels | PartsFromSources | PartsInOrders
            | PartsInProjects => "part",
            Locations | LocationOfParts => "location",
            Sources => "source",
            Projects => "project",
            LabelKeys => "label key",
            Labels => "label value",

            // Not used, but left here to catch errors
            PanelContent::None | TypeSelection => todo!(),
        };

        let block = Block::bordered()
            .border_set(border::EMPTY)
            .border_style(Style::new().on_green())
            .padding(Padding::symmetric(2, 1))
            .title(format!(" {} {} ", title_prefix, title_suffix))
            .title_bottom(" confirm by <Enter> / cancel by <ESC> ")
            .on_dark_gray();

        let block_area = block.inner(area);
        block.render(area, buf);

        let block_area = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(1),
        ])
        .split(block_area);

        let input_width = block_area[1].width - 3; // keep 2 for borders and 1 for cursor
        let mut input_block = Block::bordered().border_type(BorderType::Plain).black();

        if self.view.create_idx == CreateMode::Name {
            input_block = input_block.yellow().border_type(BorderType::Thick);
        } else {
            input_block = input_block.black().border_type(BorderType::Plain);
        }

        // Emulate cursor
        let parts = emulate_cursor(
            self.view.create_name.cursor(),
            self.view.create_name.value(),
        );

        Paragraph::new(Line::from(parts))
            .gray()
            .scroll((
                0,
                self.view.create_name.visual_scroll(input_width as usize) as u16,
            ))
            .block(input_block.clone().title("name"))
            .render(block_area[0], buf);

        if self.view.create_idx == CreateMode::Summary {
            input_block = input_block.yellow().border_type(BorderType::Thick);
        } else {
            input_block = input_block.black().border_type(BorderType::Plain);
        }

        let parts = emulate_cursor(
            self.view.create_summary.cursor(),
            self.view.create_summary.value(),
        );

        Paragraph::new(Line::from(parts))
            .gray()
            .scroll((
                0,
                self.view.create_summary.visual_scroll(input_width as usize) as u16,
            ))
            .block(input_block.title("summary"))
            .render(block_area[1], buf);

        let rows = self.view.create_hints.iter().enumerate().map(|(idx, h)| {
            Row::new(vec![
                Cell::new((idx + 1).to_string()),
                Cell::new(Line::from(vec![
                    h.name.clone().into(),
                    "  ".into(),
                    h.summary.clone().dim().black(),
                ])),
            ])
        });

        let mut table_state = TableState::new();
        let mut scrollbar_state = ScrollbarState::new(self.view.create_hints.len());

        let (hints_highlight_style, hints_hl_symbol) =
            if let CreateMode::Hint(hint) = self.view.create_idx {
                table_state = table_state.with_selected(hint);
                scrollbar_state = scrollbar_state.position(hint);
                (Style::new().on_yellow(), "> ")
            } else {
                scrollbar_state = scrollbar_state.position(0);
                (Style::default(), "  ")
            };

        let table_area =
            Layout::horizontal([Constraint::Min(1), Constraint::Length(2)]).split(block_area[2]);

        StatefulWidget::render(
            Table::new(rows, vec![Constraint::Length(3), Constraint::Min(5)])
                .row_highlight_style(hints_highlight_style)
                .highlight_symbol(hints_hl_symbol),
            table_area[0],
            buf,
            &mut table_state,
        );

        let scrollbar = Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight);
        let scrollbar = if let CreateMode::Hint(_) = self.view.create_idx {
            scrollbar.style(Style::new().yellow())
        } else {
            scrollbar
        };

        scrollbar.render(table_area[1], buf, &mut scrollbar_state);
    }

    fn alert_dialog<'a, T: Into<Text<'a>>>(
        &self,
        area: Rect,
        buf: &mut Buffer,
        title: &str,
        msg: T,
    ) {
        let area = Self::center(area, Constraint::Length(60), Constraint::Length(18));
        Clear.render(area, buf);

        let block = Block::bordered()
            .border_set(border::DOUBLE)
            .border_style(Color::Gray)
            .padding(Padding::symmetric(2, 1))
            .title_bottom(" confirm by <Enter> / cancel by <ESC> ")
            .on_light_red();

        let block_inner = block.inner(area);
        block.render(area, buf);

        let rows = Layout::vertical([Constraint::Length(4), Constraint::Min(1)]).split(block_inner);

        BigText::builder()
            .pixel_size(PixelSize::Sextant)
            .lines(vec![title.into()])
            .build()
            .render(rows[0], buf);

        Paragraph::new(msg)
            .wrap(Wrap { trim: false })
            .render(rows[1], buf);
    }
}

fn emulate_cursor(cur: usize, val: &str) -> Vec<Span<'_>> {
    let mut parts = vec![];

    let split_1 = val
        .char_indices()
        .nth(cur)
        .map_or_else(|| val.len(), |(index, _)| index);

    let split_2 = val
        .char_indices()
        .nth(cur + 1)
        .map_or_else(|| val.len(), |(index, _)| index);

    if split_1 < val.len() {
        parts.push(val[..split_1].into());
        parts.push(val[split_1..split_2].on_black().gray());
        if split_2 < val.len() {
            parts.push(val[split_2..].into());
        }
    } else {
        parts.push(val.into());
        parts.push(" ".on_black());
    }
    parts
}
