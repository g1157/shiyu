use crate::models::{DocumentTranslationItem, SaveDocumentTranslationsRequest};
use chrono::Utc;
use rusqlite::{Connection, Result};
use std::sync::MutexGuard;

pub struct DocumentTranslationRepository;

impl DocumentTranslationRepository {
    pub fn new() -> Self {
        Self
    }

    pub fn find_by_document(
        &self,
        conn: &MutexGuard<Connection>,
        document_kind: &str,
        document_id: &str,
        anchor: Option<&str>,
    ) -> Result<Vec<DocumentTranslationItem>> {
        let anchor = anchor.unwrap_or("");
        let mut stmt = conn.prepare(
            "SELECT document_kind, document_id, anchor, segment_index, source_hash, translation, updated_at
             FROM document_translations
             WHERE document_kind = ?1 AND document_id = ?2 AND anchor = ?3
             ORDER BY segment_index ASC",
        )?;

        let rows = stmt.query_map(
            rusqlite::params![document_kind, document_id, anchor],
            |row| {
                Ok(DocumentTranslationItem {
                    document_kind: row.get(0)?,
                    document_id: row.get(1)?,
                    anchor: row.get(2)?,
                    segment_index: row.get(3)?,
                    source_hash: row.get(4)?,
                    translation: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )?;

        rows.collect::<Result<Vec<_>>>()
    }

    pub fn replace_for_document(
        &self,
        conn: &MutexGuard<Connection>,
        req: SaveDocumentTranslationsRequest,
    ) -> Result<Vec<DocumentTranslationItem>> {
        let anchor = req.anchor.unwrap_or_default();
        let now = Utc::now().timestamp_millis();

        conn.execute(
            "DELETE FROM document_translations WHERE document_kind = ?1 AND document_id = ?2 AND anchor = ?3",
            rusqlite::params![req.document_kind, req.document_id, anchor],
        )?;

        let mut items = Vec::with_capacity(req.entries.len());
        for entry in req.entries {
            conn.execute(
                "INSERT INTO document_translations (
                    document_kind, document_id, anchor, segment_index, source_hash, translation, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    req.document_kind,
                    req.document_id,
                    anchor,
                    entry.segment_index,
                    entry.source_hash,
                    entry.translation,
                    now
                ],
            )?;

            items.push(DocumentTranslationItem {
                document_kind: req.document_kind.clone(),
                document_id: req.document_id.clone(),
                anchor: anchor.clone(),
                segment_index: entry.segment_index,
                source_hash: entry.source_hash,
                translation: entry.translation,
                updated_at: now,
            });
        }

        Ok(items)
    }

    pub fn delete_by_document(
        &self,
        conn: &MutexGuard<Connection>,
        document_kind: &str,
        document_id: &str,
    ) -> Result<usize> {
        conn.execute(
            "DELETE FROM document_translations WHERE document_kind = ?1 AND document_id = ?2",
            rusqlite::params![document_kind, document_id],
        )
    }
}
