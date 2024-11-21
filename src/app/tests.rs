use super::*;
use crossterm::event::{self, KeyEvent, KeyModifiers};
use event::KeyCode;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Style, Stylize},
    widgets::Widget,
};
use tempdir::TempDir;

/*
#[test]
fn render() {
    let app = App::new().unwrap();
    let mut buf = Buffer::empty(Rect::new(0, 0, 50, 4));

    app.render(buf.area, &mut buf);

    let mut expected = Buffer::with_lines(vec![
        "┏━━━━━━━━━━━━━ Counter App Tutorial ━━━━━━━━━━━━━┓",
        "┃                    Value: 0                    ┃",
        "┃                                                ┃",
        "┗━ Decrement <Left> Increment <Right> Quit <Q> ━━┛",
    ]);
    let title_style = Style::new().bold();
    let counter_style = Style::new().yellow();
    let key_style = Style::new().blue().bold();
    expected.set_style(Rect::new(14, 0, 22, 1), title_style);
    expected.set_style(Rect::new(28, 1, 1, 1), counter_style);
    expected.set_style(Rect::new(13, 3, 6, 1), key_style);
    expected.set_style(Rect::new(30, 3, 7, 1), key_style);
    expected.set_style(Rect::new(43, 3, 4, 1), key_style);

    assert_eq!(buf, expected);
}
    */

#[test]
fn test_start_stop() -> anyhow::Result<()> {
    let store_path = TempDir::new("test")?;

    let store = Store::new(store_path.into_path())?;
    let mut app = App::new(store)?;
    app.full_reload()?;

    let mut buf = Buffer::empty(Rect::new(0, 0, 60, 20));
    let area = buf.area().clone();
    app.render(area, &mut buf);

    let event = app.handle_key_event(KeyCode::F(12).into());
    assert_eq!(event, AppEvents::QUIT, "The app should quit on F12");

    Ok(())
}