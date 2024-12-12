use std::backtrace::BacktraceStatus;
use std::fs::{self, File};
use std::process::Command;
use std::{env, path::PathBuf};

use crossterm::event::{Event, EventStream, KeyEventKind};
use diilo::app::{App, AppEvents};
use diilo::store::{default_store_path, Store};
use futures::{executor::block_on, select, FutureExt, StreamExt};
use log::{debug, error, info, trace};
use tempdir::TempDir;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let store_path = if let Some(p) = args.get(1) {
        PathBuf::from(p)
    } else {
        default_store_path()?
    };

    // Init logger
    let log_path = if let Ok(fname) = env::var("LOG_FILE") {
        PathBuf::from(fname)
    } else {
        default_log_path()?
    };

    let about = format!(
        "diiLo storage manager {} ({}) - (c) MarSik {}",
        env!("CARGO_PKG_VERSION"),
        env!("GIT_HASH"),
        env!("BUILD_YEAR")
    );
    println!("{}", about);
    println!("Using: {:?}", store_path);
    println!("Logging into {:?}", log_path);

    let mut log_dir_path = log_path.clone();
    log_dir_path.pop();
    fs::create_dir_all(log_dir_path)?;
    let log_file = File::create(log_path)?;
    env_logger::Builder::new()
        .filter(None, log::LevelFilter::Info)
        .parse_default_env()
        .format_timestamp(Some(env_logger::TimestampPrecision::Micros))
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .init();

    info!("{}", about);
    info!("Using: {:?}", store_path);

    let store = Store::new(store_path.clone())?;
    let mut app = App::new(store)?;
    app.full_reload()?;
    app.update_status(format!("Loaded data from {:?}", store_path).as_str());

    let mut event_stream = EventStream::new();
    let mut needs_refresh = true;

    let mut terminal = ratatui::init();
    terminal.clear()?;

    block_on(async {
        loop {
            if needs_refresh {
                needs_refresh = false;
                let _ = terminal.draw(|frame| frame.render_widget(&app, frame.area()));
            }

            match handle_events(&mut app, &mut event_stream).await {
                Ok(AppEvents::Redraw) => {
                    needs_refresh = true;
                }
                Ok(AppEvents::FullRedraw) => {
                    let _ = terminal.clear();
                    needs_refresh = true;
                }
                Ok(AppEvents::ReloadData) => {
                    // TODO reload data store?
                    app.reload();
                    needs_refresh = true;
                }
                Ok(AppEvents::ReloadDataSelectByName(name)) => {
                    app.reload();
                    app.select_item(&name);
                    needs_refresh = true;
                }
                Ok(AppEvents::SelectByName(name)) => {
                    app.select_item(&name);
                    needs_refresh = true;
                }
                Ok(AppEvents::ReloadDataSelectByDisplayId(display_id, name)) => {
                    app.reload();
                    app.select_item_by_display_id(display_id, &name);
                    needs_refresh = true;
                }
                Ok(AppEvents::SelectByDisplayId(display_id, name)) => {
                    app.select_item_by_display_id(display_id, &name);
                    needs_refresh = true;
                }
                Ok(AppEvents::ReloadDataSelectByPartId(part_id, name)) => {
                    app.reload();
                    app.select_item_by_part_id(&part_id, &name);
                    needs_refresh = true;
                }
                Ok(AppEvents::SelectByPartId(part_id, name)) => {
                    app.select_item_by_part_id(&part_id, &name);
                    needs_refresh = true;
                }
                Ok(AppEvents::Quit) => break,
                Ok(AppEvents::Edit(part_id)) => {
                    match open_in_editor(&mut app, part_id.part_type()) {
                        Ok(name) => {
                            app.reload();
                            app.select_item(&name);
                        }
                        Err(err) => {
                            error!("open in editor: {}", err);
                            app.show_alert("Edit", err.to_string().as_str());
                        }
                    }

                    // The clear is necessary to force full redraw
                    let _ = terminal.clear();
                    needs_refresh = true;
                }
                Ok(AppEvents::Nop) => {
                    // Redraw just to be sure
                    needs_refresh = true;
                }
                Err(err) => {
                    error!("{:}", err);
                    let backtrace = err.backtrace();
                    if backtrace.status() == BacktraceStatus::Captured {
                        error!("Error context: {}", err.backtrace());
                    }
                    app.show_alert("Error", err.to_string().as_str());
                    needs_refresh = true;
                }
            }
        }
    });

    ratatui::restore();
    Ok(())
}

fn open_in_editor(app: &mut App, part_id: &std::rc::Rc<str>) -> anyhow::Result<String> {
    let part = app.get_part(part_id);
    if part.is_none() {
        return Err(anyhow::format_err!("No such part?!"));
    }
    let part = part.unwrap();
    if part.filename.is_none() {
        return Err(anyhow::format_err!("No known filename?!"));
    }

    let editor = env::var("EDITOR").unwrap_or("vi".to_string());

    let temp_dir = TempDir::new("diilo-edit")?;

    let mut temp_file = temp_dir.path().join(part.id.to_string());
    temp_file.set_extension("md");

    debug!(
        "Copying {:?} to temporary location {:?}",
        part.filename.as_ref().unwrap(),
        &temp_file
    );
    fs::copy(part.filename.as_ref().unwrap(), temp_file.clone())?;

    ratatui::restore();

    info!("Launching {} with {:?}", editor, &temp_file);
    println!("Launching {} with {:?}", editor, &temp_file);

    let mut cmd = Command::new(editor)
        .arg(temp_file.clone().as_os_str())
        .spawn()?;

    match cmd.wait() {
        Ok(_) => info!("Edit completed."),
        Err(exit) => {
            return Err(anyhow::format_err!("editor exited with error: {}", exit));
        }
    }

    let mut terminal = ratatui::init();
    terminal.clear()?;

    // Check for changes, validate and reload
    match Store::load_part_from_file(temp_file.clone()) {
        Ok(mut new_part) => {
            debug!(
                "Copying {:?} back to storage location {:?}",
                &temp_file,
                part.filename.as_ref().unwrap()
            );
            fs::copy(temp_file, part.filename.as_ref().unwrap())?;

            // Restore the original filename and id to make sure
            // the new content is linked to the proper object
            new_part.filename = part.filename.clone();
            new_part.id = part.id.clone();

            app.reload_part(&new_part);
            app.update_status(format!("{} edited and reloaded", new_part.id).as_str());
            Ok(new_part.metadata.name)
        }
        Err(err) => Err(anyhow::format_err!(
            "the new content could not be parsed: {}",
            err
        )),
    }
}

/// updates the application's state based on user input
async fn handle_events(app: &mut App, event_stream: &mut EventStream) -> anyhow::Result<AppEvents> {
    // Wait on multiple sources - event bus (TODO), keyboard
    select! {
        event = event_stream.next().fuse() => {
            trace!("Event: {:?}", event);
            match event {
                // it's important to check that the event is a key press event as
                // crossterm also emits key release and repeat events on Windows.
                Some(Ok(Event::Key(key_event))) if key_event.kind == KeyEventKind::Press => {
                    return app.handle_key_event(key_event);
                }
                Some(Ok(Event::Resize(_, _))) => return Ok(AppEvents::Redraw),
                Some(Err(e)) => return Err(anyhow::anyhow!(e)),
                _ => {}
            };
        }
    }

    Ok(AppEvents::Nop)
}

// Compute proper log path based on Free Desktop environment variables.
// This is currently Linux only
pub fn default_log_path() -> anyhow::Result<PathBuf> {
    let xdg_path = env::var("XDG_CACHE_HOME");
    if let Ok(xdg_path) = xdg_path {
        Ok(PathBuf::from(xdg_path).join("diilo"))
    } else {
        let home_path = env::var("HOME")?;
        Ok(PathBuf::from(home_path)
            .join(".cache")
            .join("diilo")
            .join("diilo.log"))
    }
}
