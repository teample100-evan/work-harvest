use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};
use work_harvest_core::{DataRootSnapshot, inspect_data_root as inspect_root};

#[derive(Default)]
struct DesktopState {
    root: Mutex<Option<PathBuf>>,
    watcher: Mutex<Option<RecommendedWatcher>>,
}

#[derive(Clone, Serialize)]
struct DataRootChange {
    paths: Vec<String>,
}

fn lock_error(label: &str) -> String {
    format!("Could not lock desktop {label} state")
}

fn start_watcher(app: &AppHandle, root: &Path) -> Result<RecommendedWatcher, String> {
    let app = app.clone();
    let mut watcher =
        notify::recommended_watcher(move |result: notify::Result<notify::Event>| match result {
            Ok(event) => {
                let payload = DataRootChange {
                    paths: event
                        .paths
                        .into_iter()
                        .map(|path| path.to_string_lossy().into_owned())
                        .collect(),
                };
                let _ = app.emit("data-root-changed", payload);
            }
            Err(error) => {
                let _ = app.emit("data-root-watch-error", error.to_string());
            }
        })
        .map_err(|error| format!("Could not create data root watcher: {error}"))?;

    watcher
        .watch(root, RecursiveMode::Recursive)
        .map_err(|error| format!("Could not watch data root: {error}"))?;
    Ok(watcher)
}

#[tauri::command]
fn set_data_root(
    app: AppHandle,
    state: State<'_, DesktopState>,
    root: String,
) -> Result<DataRootSnapshot, String> {
    let canonical_root = PathBuf::from(root)
        .canonicalize()
        .map_err(|error| format!("Could not open data root: {error}"))?;
    if !canonical_root.is_dir() {
        return Err("The selected data root is not a directory".to_string());
    }

    let snapshot = inspect_root(&canonical_root).map_err(|error| error.to_string())?;
    let watcher = start_watcher(&app, &canonical_root)?;

    *state.root.lock().map_err(|_| lock_error("root"))? = Some(canonical_root);
    *state.watcher.lock().map_err(|_| lock_error("watcher"))? = Some(watcher);

    Ok(snapshot)
}

#[tauri::command]
fn inspect_data_root(state: State<'_, DesktopState>) -> Result<DataRootSnapshot, String> {
    let root = state
        .root
        .lock()
        .map_err(|_| lock_error("root"))?
        .clone()
        .ok_or_else(|| "Choose a Work Harvest data root first".to_string())?;
    inspect_root(root).map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(DesktopState::default())
        .invoke_handler(tauri::generate_handler![set_data_root, inspect_data_root])
        .setup(|app| {
            let window = app
                .get_webview_window("main")
                .ok_or("main window was not created")?;
            window.set_title("Work Harvest")?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Work Harvest desktop application");
}
