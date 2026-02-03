use std::path::PathBuf;

use crossterm::event::KeyCode;
use diilo::{
    app::{App, AppEvents},
    store::Store,
};
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget as _};
use tempfile::TempDir;

#[test]
fn test_start_stop() -> anyhow::Result<()> {
    let store_path = TempDir::new()?;

    let store = Store::new(store_path.path().to_path_buf())?;
    let mut app = App::new(store)?;
    app.full_reload()?;

    let mut buf = Buffer::empty(Rect::new(0, 0, 60, 20));
    let area = *buf.area();
    app.render(area, &mut buf);

    let event = app.handle_key_event(KeyCode::F(12).into())?;
    assert_eq!(event, AppEvents::Quit, "The app should quit on F12");

    Ok(())
}

#[test]
fn test_loading_simple_data() -> anyhow::Result<()> {
    let store_path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", "resources", "simple"]
        .iter()
        .collect();

    let store = Store::new(store_path)?;
    let mut app = App::new(store)?;
    app.full_reload()?;

    Ok(())
}
