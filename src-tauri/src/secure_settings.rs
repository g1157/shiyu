use crate::models::SettingItem;
use keyring::{Entry, Error as KeyringError};
use rusqlite::OptionalExtension;

const KEYRING_SERVICE_NAME: &str = "com.shiyu.app";

pub fn is_secure_setting_key(key: &str) -> bool {
    matches!(key, "api_key" | "ocr_api_token")
}

fn keyring_entry(key: &str) -> Result<Entry, String> {
    Entry::new(KEYRING_SERVICE_NAME, key).map_err(|e| format!("初始化系统凭据存储失败: {}", e))
}

fn read_db_setting(conn: &rusqlite::Connection, key: &str) -> Result<Option<String>, String> {
    conn.prepare("SELECT value FROM settings WHERE key = ?1")
        .map_err(|e| e.to_string())?
        .query_row(rusqlite::params![key], |row| row.get::<_, String>(0))
        .optional()
        .map_err(|e| e.to_string())
}

fn write_db_setting(conn: &rusqlite::Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
        rusqlite::params![key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn delete_db_setting(conn: &rusqlite::Connection, key: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM settings WHERE key = ?1",
        rusqlite::params![key],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn read_secret(key: &str) -> Result<Option<String>, String> {
    let entry = keyring_entry(key)?;
    match entry.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(e) => Err(format!("读取系统凭据失败 ({}): {}", key, e)),
    }
}

fn write_secret(key: &str, value: &str) -> Result<(), String> {
    keyring_entry(key)?
        .set_password(value)
        .map_err(|e| format!("写入系统凭据失败 ({}): {}", key, e))
}

fn delete_secret(key: &str) -> Result<(), String> {
    match keyring_entry(key)?.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(e) => Err(format!("删除系统凭据失败 ({}): {}", key, e)),
    }
}

pub fn get_setting_value(conn: &rusqlite::Connection, key: &str) -> Result<Option<String>, String> {
    if !is_secure_setting_key(key) {
        return read_db_setting(conn, key);
    }

    if let Some(value) = read_secret(key)? {
        return Ok(Some(value));
    }

    let legacy_value = read_db_setting(conn, key)?;
    if let Some(ref value) = legacy_value {
        write_secret(key, value)?;
        delete_db_setting(conn, key)?;
    }

    Ok(legacy_value)
}

pub fn set_setting_value(
    conn: &rusqlite::Connection,
    key: &str,
    value: &str,
) -> Result<(), String> {
    if !is_secure_setting_key(key) {
        return write_db_setting(conn, key, value);
    }

    if value.trim().is_empty() {
        delete_secret(key)?;
    } else {
        write_secret(key, value)?;
    }
    delete_db_setting(conn, key)?;
    Ok(())
}

pub fn delete_setting_value(conn: &rusqlite::Connection, key: &str) -> Result<(), String> {
    if is_secure_setting_key(key) {
        delete_secret(key)?;
    }
    delete_db_setting(conn, key)
}

pub fn get_all_settings(conn: &rusqlite::Connection) -> Result<Vec<SettingItem>, String> {
    let mut stmt = conn
        .prepare("SELECT key, value FROM settings WHERE key NOT IN ('api_key', 'ocr_api_token') ORDER BY key")
        .map_err(|e| e.to_string())?;

    let mut items = stmt
        .query_map([], |row| {
            Ok(SettingItem {
                key: row.get(0)?,
                value: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    for key in ["api_key", "ocr_api_token"] {
        if let Some(value) = get_setting_value(conn, key)? {
            items.push(SettingItem {
                key: key.to_string(),
                value,
            });
        }
    }

    items.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::is_secure_setting_key;

    #[test]
    fn recognizes_sensitive_keys() {
        assert!(is_secure_setting_key("api_key"));
        assert!(is_secure_setting_key("ocr_api_token"));
        assert!(!is_secure_setting_key("api_url"));
        assert!(!is_secure_setting_key("theme"));
    }
}
