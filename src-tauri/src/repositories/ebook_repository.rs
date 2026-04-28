use crate::models::{EbookItem, UpdateEbookProgressRequest};
use crate::repositories::FromRow;
use chrono::Utc;
use rusqlite::{Connection, Result};
use std::sync::MutexGuard;
use uuid::Uuid;

impl FromRow for EbookItem {
    fn from_row(row: &rusqlite::Row) -> Result<Self> {
        Ok(EbookItem {
            id: row.get(0)?,
            title: row.get(1)?,
            file_path: row.get(2)?,
            author: row.get(3)?,
            format: row.get(4)?,
            progress: row.get(5)?,
            cfi_position: row.get(6)?,
            last_read_at: row.get(7)?,
            created_at: row.get(8)?,
            source_hash: row.get(9)?,
            cover_path: row.get(10)?,
        })
    }
}

pub struct EbookRepository;

impl EbookRepository {
    pub fn new() -> Self {
        Self
    }

    pub fn find_all(&self, conn: &MutexGuard<Connection>) -> Result<Vec<EbookItem>> {
        let mut stmt = conn.prepare(
            "SELECT id, title, file_path, author, format, progress, cfi_position, last_read_at, created_at, source_hash, cover_path
             FROM ebooks
             ORDER BY COALESCE(last_read_at, created_at) DESC, created_at DESC"
        )?;

        let items = stmt
            .query_map([], |row| EbookItem::from_row(row))?
            .collect::<Result<Vec<_>>>()?;

        Ok(items)
    }

    pub fn find_by_id(&self, conn: &MutexGuard<Connection>, id: &str) -> Result<Option<EbookItem>> {
        let result = conn.query_row(
            "SELECT id, title, file_path, author, format, progress, cfi_position, last_read_at, created_at, source_hash, cover_path
             FROM ebooks WHERE id = ?1",
            [id],
            |row| EbookItem::from_row(row),
        );

        match result {
            Ok(item) => Ok(Some(item)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn find_by_source_hash(
        &self,
        conn: &MutexGuard<Connection>,
        source_hash: &str,
    ) -> Result<Option<EbookItem>> {
        let result = conn.query_row(
            "SELECT id, title, file_path, author, format, progress, cfi_position, last_read_at, created_at, source_hash, cover_path
             FROM ebooks WHERE source_hash = ?1",
            [source_hash],
            |row| EbookItem::from_row(row),
        );

        match result {
            Ok(item) => Ok(Some(item)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn create(
        &self,
        conn: &MutexGuard<Connection>,
        title: String,
        file_path: String,
        author: Option<String>,
        format: String,
        source_hash: Option<String>,
        cover_path: Option<String>,
    ) -> Result<EbookItem> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp_millis();

        conn.execute(
            "INSERT INTO ebooks (id, title, file_path, author, format, progress, cfi_position, last_read_at, created_at, source_hash, cover_path)
             VALUES (?1, ?2, ?3, ?4, ?5, 0.0, NULL, NULL, ?6, ?7, ?8)",
            rusqlite::params![id, title, file_path, author, format, now, source_hash, cover_path],
        )?;

        Ok(EbookItem {
            id,
            title,
            file_path,
            author,
            format,
            progress: 0.0,
            cfi_position: None,
            last_read_at: None,
            created_at: now,
            source_hash,
            cover_path,
        })
    }

    pub fn update_cover_path(
        &self,
        conn: &MutexGuard<Connection>,
        id: &str,
        cover_path: Option<String>,
    ) -> Result<EbookItem> {
        conn.execute(
            "UPDATE ebooks SET cover_path = ?1 WHERE id = ?2",
            rusqlite::params![cover_path, id],
        )?;

        self.find_by_id(conn, id)?
            .map_or(Err(rusqlite::Error::QueryReturnedNoRows), Ok)
    }

    pub fn update_progress(
        &self,
        conn: &MutexGuard<Connection>,
        req: UpdateEbookProgressRequest,
    ) -> Result<EbookItem> {
        let now = Utc::now().timestamp_millis();
        conn.execute(
            "UPDATE ebooks
             SET progress = ?1, cfi_position = ?2, last_read_at = ?3
             WHERE id = ?4",
            rusqlite::params![req.progress, req.cfi_position, now, req.id],
        )?;

        self.find_by_id(conn, &req.id)?
            .map_or(Err(rusqlite::Error::QueryReturnedNoRows), Ok)
    }

    pub fn delete(&self, conn: &MutexGuard<Connection>, id: &str) -> Result<usize> {
        conn.execute("DELETE FROM ebooks WHERE id = ?1", [id])
    }
}
