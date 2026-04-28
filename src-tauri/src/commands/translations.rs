use crate::db::Database;
use crate::models::{DocumentTranslationItem, SaveDocumentTranslationsRequest};
use crate::repositories::document_translation_repository::DocumentTranslationRepository;
use tauri::State;

#[tauri::command]
pub fn get_document_translations(
    db: State<Database>,
    document_kind: String,
    document_id: String,
    anchor: Option<String>,
) -> Result<Vec<DocumentTranslationItem>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = DocumentTranslationRepository::new();
    repo.find_by_document(&conn, &document_kind, &document_id, anchor.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_document_translations(
    db: State<Database>,
    req: SaveDocumentTranslationsRequest,
) -> Result<Vec<DocumentTranslationItem>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = DocumentTranslationRepository::new();
    repo.replace_for_document(&conn, req)
        .map_err(|e| e.to_string())
}
