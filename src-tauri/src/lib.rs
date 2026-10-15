use std::sync::Mutex;

use dovecote_engine::{Command, Engine};
use tauri::{AppHandle, Emitter, Manager, State};

struct EngineState(Mutex<Engine>);

#[tauri::command]
fn send_command(app: AppHandle, state: State<EngineState>, command: Command) -> Result<(), String> {
    let event = state.0.lock().map_err(|e| e.to_string())?.handle(command);
    app.emit("engine-event", event).map_err(|e| e.to_string())
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            app.manage(EngineState(Mutex::new(Engine::open(&dir.join("mail.db"))?)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![send_command])
        .run(tauri::generate_context!())
        .expect("error while running Dovecote");
}
