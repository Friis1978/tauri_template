use std::sync::Mutex;

use std::path::PathBuf;

use tauri::{Emitter, Manager, AppHandle};
use tauri_plugin_dialog::DialogExt;

#[cfg(desktop)]
use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};

use tauri_plugin_sql::{Migration, MigrationKind};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let migrations = vec![
        Migration {
            version: 1,
            description: "create users table",
            sql: r#"
                CREATE TABLE IF NOT EXISTS users (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    username TEXT NOT NULL,
                    password TEXT NOT NULL
                );
                "#,
            kind: MigrationKind::Up,
        },
    ];
    tauri::Builder::default()
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations("sqlite:mydatabase.db", migrations)
                .build()
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(User {
            id: 0,
            username: "".to_string(),
            password: "".to_string(),
        }))
        .invoke_handler(tauri::generate_handler![
            login,
            create_user,
            open_file,
            save_file,
            save_device_file,
            read_device_file,
            list_device_files
        ])
        .setup(|_app| {
            #[cfg(debug_assertions)] // only include this code on debug builds
            {
                let window = _app.get_webview_window("main").unwrap();
                window.open_devtools();
                window.close_devtools();
            }

            #[cfg(desktop)]
            {
                let open_menu = MenuItemBuilder::new("Open file").id("open_file").build(_app)?;
                let save_menu = MenuItemBuilder::new("Save file").id("save_file").build(_app)?;

                let file_submenu = SubmenuBuilder::new(_app, "File")
                    .items(&[&open_menu, &save_menu])
                    .build()?;

                _app.on_menu_event(move |_app, event| {
                    if event.id() == open_menu.id() {
                        println!("Open menu item clicked");
                    } else if event.id() == save_menu.id() {
                        println!("Save menu item clicked");
                    }
                });

                let menu = MenuBuilder::new(_app).item(&file_submenu).build()?;
                _app.set_menu(menu)?;
            }
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn login(state: tauri::State<Mutex<User>>, user: User) -> User {
    *state.lock().unwrap() = user;
    println!("Tauri db save locale state {}", state.lock().unwrap().username);
    state.lock().unwrap().clone()
}

#[tauri::command]
async fn create_user(username: String, _password: String) -> Result<String, String> {
    // This command should be called from the frontend, which will use Database.execute()
    // The database operations should be done from the JavaScript side using the plugin
    Ok(format!("User {} would be created", username))
}

#[tauri::command]
fn save_file(app: AppHandle, content: String){
    std::thread::spawn(move || {
        let file_path = app.dialog()
            .file()
            .add_filter("Text Files", &["txt"])
            .blocking_save_file()
            .unwrap();
        let file: String = file_path.to_string();

        std::fs::write(file.clone(), content.clone()).unwrap();

        app.emit("save_state", file).unwrap();
    });
}

#[tauri::command]
fn open_file(app: AppHandle){
    std::thread::spawn(move || {
        let file_path = app.dialog().file().blocking_pick_file().unwrap();
        let file: String = file_path.to_string();

        let content = std::fs::read_to_string(file).unwrap();

        app.emit("content", content).unwrap();
    });
}

fn sanitize_filename(filename: &str) -> Result<String, String> {
    let trimmed = filename.trim();
    if trimmed.is_empty() {
        return Err("Filename cannot be empty".to_string());
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err("Filename cannot include path separators".to_string());
    }
    let mut name = trimmed.to_string();
    if !name.to_lowercase().ends_with(".txt") {
        name.push_str(".txt");
    }
    Ok(name)
}

fn app_data_txt_path(app: &AppHandle, filename: &str) -> Result<PathBuf, String> {
    let name = sanitize_filename(filename)?;
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create app data dir: {e}"))?;
    Ok(dir.join(name))
}

#[tauri::command]
fn save_device_file(app: AppHandle, filename: String, content: String) -> Result<String, String> {
    let path = app_data_txt_path(&app, &filename)?;
    std::fs::write(&path, content).map_err(|e| format!("Failed to write file: {e}"))?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn read_device_file(app: AppHandle, filename: String) -> Result<String, String> {
    let path = app_data_txt_path(&app, &filename)?;
    std::fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {e}"))
}

#[tauri::command]
fn list_device_files(app: AppHandle) -> Result<Vec<String>, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| format!("Failed to read app data dir: {e}"))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.to_lowercase().ends_with(".txt") {
                    files.push(name.to_string());
                }
            }
        }
    }
    files.sort();
    Ok(files)
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct User {
    id: u32,
    username: String,
    password: String,
}