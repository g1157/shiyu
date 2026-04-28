use crate::models::{AddSentenceRequest, SentenceItem, UpdateSrsRequest};
use crate::repositories::FromRow;
use chrono::Utc;
use rusqlite::{Connection, Result};
use std::sync::MutexGuard;
use uuid::Uuid;

impl FromRow for SentenceItem {
    fn from_row(row: &rusqlite::Row) -> Result<Self> {
        Ok(SentenceItem {
            id: row.get(0)?,
            sentence: row.get(1)?,
            explanation: row.get(2)?,
            article_path: row.get(3)?,
            ebook_id: row.get(4)?,
            ebook_cfi: row.get(5)?,
            ebook_href: row.get(6)?,
            document_kind: row.get(7)?,
            document_id: row.get(8)?,
            review_count: row.get(9)?,
            last_reviewed_at: row.get(10)?,
            created_at: row.get(11)?,
            srs_due: row.get(12)?,
            srs_stability: row.get(13)?,
            srs_difficulty: row.get(14)?,
            srs_state: row.get(15)?,
            srs_lapses: row.get(16)?,
            srs_reps: row.get(17)?,
            srs_last_review: row.get(18)?,
        })
    }
}

const SENTENCE_SELECT: &str =
    "SELECT id, sentence, explanation, article_path, ebook_id, ebook_cfi, ebook_href,
            document_kind, document_id,
            review_count, last_reviewed_at, created_at, srs_due, srs_stability, srs_difficulty,
            srs_state, srs_lapses, srs_reps, srs_last_review
     FROM sentences";

/// 句子Repository
pub struct SentenceRepository;

impl SentenceRepository {
    pub fn new() -> Self {
        Self
    }

    /// 获取所有句子
    pub fn find_all(&self, conn: &MutexGuard<Connection>) -> Result<Vec<SentenceItem>> {
        let sql = format!("{} ORDER BY created_at DESC", SENTENCE_SELECT);
        let mut stmt = conn.prepare(&sql)?;

        let items = stmt
            .query_map([], |row| SentenceItem::from_row(row))?
            .collect::<Result<Vec<_>>>()?;

        Ok(items)
    }

    /// 根据文章ID查找关联句子
    pub fn find_by_article(
        &self,
        conn: &MutexGuard<Connection>,
        article_path: &str,
    ) -> Result<Vec<SentenceItem>> {
        let sql = format!(
            "{} WHERE article_path = ?1 OR (document_kind = 'article' AND document_id = ?1) ORDER BY created_at DESC",
            SENTENCE_SELECT
        );
        let mut stmt = conn.prepare(&sql)?;

        let items = stmt
            .query_map([article_path], |row| SentenceItem::from_row(row))?
            .collect::<Result<Vec<_>>>()?;

        Ok(items)
    }

    pub fn find_by_ebook(
        &self,
        conn: &MutexGuard<Connection>,
        ebook_id: &str,
    ) -> Result<Vec<SentenceItem>> {
        let sql = format!(
            "{} WHERE ebook_id = ?1 OR (document_kind = 'ebook' AND document_id = ?1) ORDER BY created_at DESC",
            SENTENCE_SELECT
        );
        let mut stmt = conn.prepare(&sql)?;

        let items = stmt
            .query_map([ebook_id], |row| SentenceItem::from_row(row))?
            .collect::<Result<Vec<_>>>()?;

        Ok(items)
    }

    /// 获取到期待复习的句子
    pub fn find_due(
        &self,
        conn: &MutexGuard<Connection>,
        now_ms: i64,
    ) -> Result<Vec<SentenceItem>> {
        let sql = format!(
            "{} WHERE (srs_due IS NULL OR srs_due <= ?1) AND srs_state != -1 ORDER BY srs_due ASC",
            SENTENCE_SELECT
        );
        let mut stmt = conn.prepare(&sql)?;

        let items = stmt
            .query_map([now_ms], |row| SentenceItem::from_row(row))?
            .collect::<Result<Vec<_>>>()?;

        Ok(items)
    }

    /// 添加句子
    pub fn create(
        &self,
        conn: &MutexGuard<Connection>,
        req: AddSentenceRequest,
    ) -> Result<SentenceItem> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp_millis();
        let document_kind = req.document_kind.or_else(|| {
            if req.ebook_id.is_some() {
                Some("ebook".to_string())
            } else if req.article_path.is_some() {
                Some("article".to_string())
            } else {
                None
            }
        });
        let document_id = req
            .document_id
            .or_else(|| req.ebook_id.clone())
            .or_else(|| req.article_path.clone());

        conn.execute(
            "INSERT INTO sentences (
                id, sentence, explanation, article_path, ebook_id, ebook_cfi, ebook_href, document_kind, document_id, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                id,
                req.sentence,
                req.explanation,
                req.article_path,
                req.ebook_id,
                req.ebook_cfi,
                req.ebook_href,
                document_kind,
                document_id,
                now
            ],
        )?;

        Ok(SentenceItem {
            id,
            sentence: req.sentence,
            explanation: req.explanation,
            article_path: req.article_path,
            ebook_id: req.ebook_id,
            ebook_cfi: req.ebook_cfi,
            ebook_href: req.ebook_href,
            document_kind,
            document_id,
            review_count: 0,
            last_reviewed_at: None,
            created_at: now,
            srs_due: None,
            srs_stability: 0.0,
            srs_difficulty: 0.0,
            srs_state: 0,
            srs_lapses: 0,
            srs_reps: 0,
            srs_last_review: None,
        })
    }

    /// 删除句子
    pub fn delete(&self, conn: &MutexGuard<Connection>, id: &str) -> Result<usize> {
        conn.execute("DELETE FROM sentences WHERE id = ?1", [id])
    }

    /// 更新复习状态（旧接口，保持兼容）
    pub fn update_review(&self, conn: &MutexGuard<Connection>, id: &str) -> Result<usize> {
        let now = Utc::now().timestamp_millis();
        conn.execute(
            "UPDATE sentences SET review_count = review_count + 1, last_reviewed_at = ?1 WHERE id = ?2",
            rusqlite::params![now, id],
        )
    }

    /// 更新 SRS 状态（FSRS 算法计算后的结果）
    pub fn update_srs(
        &self,
        conn: &MutexGuard<Connection>,
        req: &UpdateSrsRequest,
    ) -> Result<usize> {
        let now = Utc::now().timestamp_millis();
        conn.execute(
            "UPDATE sentences SET
                review_count = review_count + 1,
                last_reviewed_at = ?1,
                srs_due = ?2,
                srs_stability = ?3,
                srs_difficulty = ?4,
                srs_state = ?5,
                srs_lapses = ?6,
                srs_reps = ?7,
                srs_last_review = ?8
             WHERE id = ?9",
            rusqlite::params![
                now,
                req.srs_due,
                req.srs_stability,
                req.srs_difficulty,
                req.srs_state,
                req.srs_lapses,
                req.srs_reps,
                req.srs_last_review,
                req.id,
            ],
        )
    }

    /// 根据文章路径删除所有关联句子
    pub fn delete_by_article(
        &self,
        conn: &MutexGuard<Connection>,
        article_path: &str,
    ) -> Result<usize> {
        conn.execute(
            "DELETE FROM sentences WHERE article_path = ?1 OR (document_kind = 'article' AND document_id = ?1)",
            [article_path],
        )
    }

    pub fn delete_by_ebook(&self, conn: &MutexGuard<Connection>, ebook_id: &str) -> Result<usize> {
        conn.execute(
            "DELETE FROM sentences WHERE ebook_id = ?1 OR (document_kind = 'ebook' AND document_id = ?1)",
            [ebook_id],
        )
    }
}
