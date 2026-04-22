use crate::db::Database;
use crate::secure_settings::{
    delete_setting_value, get_all_settings as load_all_settings, get_setting_value,
    set_setting_value,
};
use tauri::State;

#[tauri::command]
pub fn get_setting(db: State<Database>, key: String) -> Result<Option<String>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    get_setting_value(&conn, &key)
}

#[tauri::command]
pub fn set_setting(db: State<Database>, key: String, value: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    set_setting_value(&conn, &key, &value)
}

#[tauri::command]
pub fn get_all_settings(db: State<Database>) -> Result<Vec<crate::models::SettingItem>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    load_all_settings(&conn)
}

#[tauri::command]
pub fn delete_setting(db: State<Database>, key: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    delete_setting_value(&conn, &key)
}
