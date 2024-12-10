use log::trace;
use ratatui::{style::Style, widgets::Widget};

pub struct Fixed6x3Icon([&'static str; 3]);

pub struct DrawFixed6x3Icon(pub Fixed6x3Icon, pub Style);

impl DrawFixed6x3Icon {
    pub fn with_icon(i: Fixed6x3Icon) -> Self {
        DrawFixed6x3Icon(i, Style::default())
    }

    pub fn with_style(self, s: Style) -> Self {
        DrawFixed6x3Icon(self.0, s)
    }
}

impl Widget for DrawFixed6x3Icon {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let x_offset = area.width.saturating_sub(6) / 2;
        let y_offset = area.height.saturating_sub(3) / 2;

        for r in 0..area.height.min(3) {
            for c in 0..area.width.min(6) {
                let row = self.0 .0[r as usize];
                // Handle multibyte characters properly
                let mut row_it = row.char_indices().skip(c as usize);
                // Get the beginning of the multibyte symbol
                let first = row_it.next().unwrap_or((row.len(), ' '));
                // Get the beginning of the next multibyte symbol
                let next = row_it.next().unwrap_or((row.len(), ' '));
                // Collect all chars in between
                let chr: String = row[first.0..next.0].to_string();
                trace!(
                    "Icon row {}: taking {:?}..{:?} ({}) = {:?} {:x?}",
                    r,
                    first,
                    next,
                    chr.len(),
                    &chr,
                    chr.bytes()
                );
                // Render
                buf.cell_mut((area.x + c + x_offset, area.y + r + y_offset))
                    .map(|cell| cell.set_symbol(&chr).set_style(self.1));
            }
        }
    }
}

pub const EMPTY: Fixed6x3Icon = Fixed6x3Icon(["      ", "      ", "      "]);

pub const TRUCK: Fixed6x3Icon = Fixed6x3Icon(["    ▛▜", " ▙▄▄██", " O   O"]);

pub const LABEL: Fixed6x3Icon = Fixed6x3Icon([" ▞▀▀▀▜", "▐    ▐", " ▚▄▄▄▟"]);

pub const LABEL_X: Fixed6x3Icon = Fixed6x3Icon([" ▞▜▀▛▜", "▐  █ ▐", " ▚▟▄▙▟"]);

pub const SOLDER: Fixed6x3Icon = Fixed6x3Icon(["▚ ▐▌ ▞", " ▚▐▌▞ ", "▄▄▟▙▄▄"]);

pub const UNSOLDER: Fixed6x3Icon = Fixed6x3Icon([" ▞▐▌▚ ", "▞ ▐▌ ▚", "▄▄▟▙▄▄"]);

pub const ORDER: Fixed6x3Icon = Fixed6x3Icon(["▛▀▀▀▛▜", "▌   ▙▟", "▙▄▄▄▄▟"]);

pub const MOVE: Fixed6x3Icon = Fixed6x3Icon(["▚ ▚   ", " ▌ ▌▐▌", "▞ ▞   "]);

pub const DELETE: Fixed6x3Icon = Fixed6x3Icon([" ▚  ▞ ", "  ▐▌  ", " ▞  ▚ "]);

pub const REQUIRE: Fixed6x3Icon = Fixed6x3Icon(["▛▀▀▀▀▜", "▌ n? ▐", "▙▄▄▄▄▟"]);

pub const FORCE_COUNT: Fixed6x3Icon = Fixed6x3Icon(["▛▀▀▀▀▜", "!!!!!!", "▙▄▄▄▄▟"]);

pub const SPLIT: Fixed6x3Icon = Fixed6x3Icon(["▛▜  ▛▜", "▌▐▐▌▌▐", "▙▟  ▙▟"]);
