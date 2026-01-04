use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_user])
        .setup(|app| {
            #[cfg(debug_assertions)] // only include this code on debug builds
    {
      let window = app.get_webview_window("main").unwrap();
      window.open_devtools();
      window.close_devtools();
    }
    Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn get_user() -> User{
    User {
        id: 1,
        name: "Alice".to_string(),
        password: "password123".to_string(),
    }
}

#[derive(serde::Serialize)]
struct User {
    id: u32,
    name: String,
    password: String,
}