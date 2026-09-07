#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    Manager, WebviewWindowBuilder,
};

const NOTIF_POLYFILL: &str = r#"
Notification.requestPermission = function(callback) {
  return new Promise((resolve) => {
    window.__TAURI__.dialog.ask('Allow notifications from this app?', {
      title: 'Notification Permission',
      kind: 'info',
    }).then((allowed) => {
      localStorage.setItem('notifPermission', allowed ? 'granted' : 'denied');
      const result = allowed ? 'granted' : 'denied';
      if (typeof callback === 'function') callback(result);
      resolve(result);
    });
  });
};
Object.defineProperty(Notification, 'permission', {
  get() { return localStorage.getItem('notifPermission') || 'default'; }
});
const _OrigNotification = Notification;
window.Notification = class extends _OrigNotification {
  constructor(title, options = {}) {
    super(title, options);
    if (localStorage.getItem('notifPermission') === 'granted') {
      window.__TAURI__.notification.sendNotification({
        title, body: options.body || '',
      }).catch(() => {});
    }
  }
};
window.Notification.requestPermission = Notification.requestPermission;
window.Notification.permission = Notification.permission;
"#;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let window = WebviewWindowBuilder::from_config(
                app.handle(),
                &app.config().app.windows[0],
            )?
            .initialization_script(NOTIF_POLYFILL)
            .build()?;

            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => std::process::exit(0),
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { .. } = event {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            let w = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = w.hide();
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running application");
}   
