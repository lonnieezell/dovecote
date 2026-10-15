use std::sync::Arc;

use dovecote_engine::{Command, Engine, Event, Keychain};
use tauri::{AppHandle, Emitter, Manager, State};

/// Returns the events that answer the command to the caller, and broadcasts them too.
/// The command has already taken effect, so a failed broadcast must not turn it into an error.
#[tauri::command]
async fn send_command(
    app: AppHandle,
    engine: State<'_, Engine>,
    command: Command,
) -> Result<Vec<Event>, String> {
    // Tauri requires a Result from an async command that borrows its state. It never fails.
    let events = engine.handle(command).await;
    for event in &events {
        let _ = app.emit("engine-event", event);
    }
    Ok(events)
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let keychain = Arc::new(Keychain::new(&app.config().identifier));
            app.manage(Engine::open(&dir.join("mail.db"), keychain)?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![send_command])
        .run(tauri::generate_context!())
        .expect("error while running Dovecote");
}
