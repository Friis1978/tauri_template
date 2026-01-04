use std::sync::Mutex;

use tauri::{Emitter, Manager, menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder}};
use tauri_plugin_dialog::DialogExt;
use tauri::AppHandle;

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
        .manage(Mutex::new(User {
            id: 0,
            username: "".to_string(),
            password: "".to_string(),
        }))
        .invoke_handler(tauri::generate_handler![get_all_users, get_user, login, open_file, save_file])
        .setup(|app| {
            #[cfg(debug_assertions)] // only include this code on debug builds
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
                window.close_devtools();
            }

            let open_menu = MenuItemBuilder::new("Open file").id("open_file").build(app)?;
            let save_menu = MenuItemBuilder::new("Save file").id("save_file").build(app)?;

            let file_submenu = SubmenuBuilder::new(app, "File")
                .items(&[&open_menu, &save_menu])
                .build()?;

            app.on_menu_event(move |_app, event| {
                if event.id() == open_menu.id() {
                    println!("Open menu item clicked");
                } else if event.id() == save_menu.id() {
                    println!("Save menu item clicked");
                }
            });

            let menu = MenuBuilder::new(app).item(&file_submenu).build()?;
            app.set_menu(menu)?;
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn get_all_users() -> Vec<User>{

    let user1 = User {
        id: 1,
        username: "Alice".to_string(),
        password: "password123".to_string(),
    };
    let user2 = User {
        id: 2,
        username: "Bob".to_string(),
        password: "securepass".to_string(),
    };
    vec![
        user1,
        user2,
    ]
}

#[tauri::command]
fn get_user(state: tauri::State<Mutex<User>>) -> User {
    let user = &*state.lock().unwrap();
    user.clone()
}

#[tauri::command]
fn login(state: tauri::State<Mutex<User>>, user: User) -> User {
    *state.lock().unwrap() = user;
    println!("{}", state.lock().unwrap().username);
    state.lock().unwrap().clone()
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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct User {
    id: u32,
    username: String,
    password: String,
}