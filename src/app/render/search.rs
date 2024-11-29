use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Stylize},
    symbols::border,
    text::Line,
    widgets::{Block, Clear, Padding, Paragraph, Widget},
};

use crate::app::App;

use super::emulate_cursor;

impl App {
    pub(crate) fn search_dialog(&self, area: Rect, buf: &mut Buffer) {
        let area = Self::center(area, Constraint::Percentage(90), Constraint::Length(6));
        Clear.render(area, buf);

        let block = Block::bordered()
            .border_set(border::PLAIN)
            .border_style(Color::Gray)
            .padding(Padding::symmetric(2, 1))
            .title(" Search ")
            .title_bottom(" confirm by <Enter> / close by <ESC> / cancel by <F12> ")
            .on_blue();

        let block_inner = block.inner(area);
        block.render(area, buf);

        let rows = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(block_inner);

        // Emulate cursor
        let parts = emulate_cursor(
            self.view.search_query.cursor(),
            self.view.search_query.value(),
        );

        let input_width = rows[0].width - 2; // keep 2 for borders and 1 for cursor

        Paragraph::new(Line::from(parts))
            .on_gray()
            .black()
            .scroll((
                0,
                self.view.search_query.visual_scroll(input_width as usize) as u16,
            ))
            .render(rows[0], buf);
    }
}
