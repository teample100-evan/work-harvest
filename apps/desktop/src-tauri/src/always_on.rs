use tauri::{
    App, AppHandle, Emitter, Manager, Runtime, Window, WindowEvent,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};
use tauri_plugin_window_state::{AppHandleExt, StateFlags};
use work_harvest_core::DataRootSnapshot;

const TRAY_ID: &str = "work-harvest";
const OPEN_ID: &str = "open";
const QUIT_ID: &str = "quit";
const WORK_ITEM_PREFIX: &str = "work-item:";
const RECENT_WORK_ITEM_LIMIT: usize = 5;

pub fn window_state_flags() -> StateFlags {
    StateFlags::SIZE
        | StateFlags::POSITION
        | StateFlags::MAXIMIZED
        | StateFlags::DECORATIONS
        | StateFlags::FULLSCREEN
}

fn menu_item<R: Runtime>(
    app: &AppHandle<R>,
    id: impl Into<tauri::menu::MenuId>,
    text: impl AsRef<str>,
    enabled: bool,
) -> tauri::Result<MenuItem<R>> {
    MenuItem::with_id(app, id, text, enabled, None::<&str>)
}

fn short_label(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let prefix = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{prefix}…")
    } else {
        prefix
    }
}

fn build_menu<R: Runtime>(
    app: &AppHandle<R>,
    snapshot: Option<&DataRootSnapshot>,
) -> tauri::Result<Menu<R>> {
    let menu = Menu::new(app)?;
    menu.append(&menu_item(app, OPEN_ID, "Work Harvest 열기", true)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;

    if let Some(snapshot) = snapshot {
        menu.append(&menu_item(
            app,
            "summary",
            format!(
                "업무 {}개 · 체크포인트 {}개",
                snapshot.counts.work_items, snapshot.counts.checkpoints
            ),
            false,
        )?)?;

        if let Some((work_item, checkpoint_id)) = snapshot.work_items.iter().find_map(|item| {
            item.last_checkpoint_id
                .as_ref()
                .filter(|id| !snapshot.restricted_checkpoint_ids.contains(id))
                .map(|id| (item, id))
        }) {
            menu.append(&menu_item(
                app,
                "last-checkpoint",
                format!(
                    "마지막 기록 · {} · {}",
                    checkpoint_id,
                    short_label(&work_item.title, 30)
                ),
                false,
            )?)?;
        }

        menu.append(&PredefinedMenuItem::separator(app)?)?;
        menu.append(&menu_item(app, "recent-heading", "최근 업무", false)?)?;
        for item in snapshot.work_items.iter().take(RECENT_WORK_ITEM_LIMIT) {
            menu.append(&menu_item(
                app,
                format!("{WORK_ITEM_PREFIX}{}", item.id),
                format!("{} · {}", short_label(&item.title, 36), item.status),
                true,
            )?)?;
        }
    } else {
        menu.append(&menu_item(
            app,
            "not-connected",
            "데이터 폴더를 연결하세요",
            false,
        )?)?;
    }

    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&menu_item(app, QUIT_ID, "Work Harvest 종료", true)?)?;
    Ok(menu)
}

fn show_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    app.show()?;

    if let Some(window) = app.get_webview_window("main") {
        window.unminimize()?;
        window.show()?;
        window.set_focus()?;
    }
    Ok(())
}

fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event_id: &str) {
    match event_id {
        OPEN_ID => {
            let _ = show_main_window(app);
        }
        QUIT_ID => app.exit(0),
        _ => {
            if let Some(work_item_id) = event_id.strip_prefix(WORK_ITEM_PREFIX) {
                let _ = show_main_window(app);
                let _ = app.emit("tray-work-item-selected", work_item_id.to_string());
            }
        }
    }
}

pub fn install(app: &mut App) -> tauri::Result<()> {
    let menu = build_menu(app.handle(), None)?;
    TrayIconBuilder::with_id(TRAY_ID)
        .title("WH")
        .tooltip("Work Harvest")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| handle_menu_event(app, event.id().as_ref()))
        .build(app)?;
    Ok(())
}

pub fn update_menu(app: &AppHandle, snapshot: &DataRootSnapshot) -> Result<(), String> {
    let tray = app
        .tray_by_id(TRAY_ID)
        .ok_or_else(|| "Work Harvest menu bar item is unavailable".to_string())?;
    let menu = build_menu(app, Some(snapshot))
        .map_err(|error| format!("Could not build menu bar content: {error}"))?;
    tray.set_menu(Some(menu))
        .map_err(|error| format!("Could not update menu bar content: {error}"))
}

pub fn handle_window_event(window: &Window, event: &WindowEvent) {
    if window.label() != "main" {
        return;
    }
    if let WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let _ = window.app_handle().save_window_state(window_state_flags());
        let _ = window.hide();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_label_preserves_short_and_truncates_long_unicode_text() {
        assert_eq!(short_label("짧은 업무", 10), "짧은 업무");
        assert_eq!(short_label("가나다라마바사", 4), "가나다라…");
    }

    #[test]
    fn work_item_menu_id_round_trips() {
        let id = "WH-20260714-menu-bar";
        let menu_id = format!("{WORK_ITEM_PREFIX}{id}");
        assert_eq!(menu_id.strip_prefix(WORK_ITEM_PREFIX), Some(id));
    }

    #[test]
    fn window_state_does_not_restore_hidden_visibility() {
        assert!(!window_state_flags().contains(StateFlags::VISIBLE));
    }
}
