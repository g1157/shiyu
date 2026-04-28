use crate::models::{AddVocabularyRequest, UpdateSrsRequest, VocabularyGrouped, VocabularyItem};
use crate::repositories::FromRow;
use chrono::Utc;
use rusqlite::{Connection, Result};
use std::sync::MutexGuard;
use uuid::Uuid;

impl FromRow for VocabularyItem {
    fn from_row(row: &rusqlite::Row) -> Result<Self> {
        Ok(VocabularyItem {
            id: row.get(0)?,
            word: row.get(1)?,
            meaning: row.get(2)?,
            context: row.get(3)?,
            article_path: row.get(4)?,
            ebook_id: row.get(5)?,
            ebook_cfi: row.get(6)?,
            ebook_href: row.get(7)?,
            document_kind: row.get(8)?,
            document_id: row.get(9)?,
            review_count: row.get(10)?,
            last_reviewed_at: row.get(11)?,
            created_at: row.get(12)?,
            srs_due: row.get(13)?,
            srs_stability: row.get(14)?,
            srs_difficulty: row.get(15)?,
            srs_state: row.get(16)?,
            srs_lapses: row.get(17)?,
            srs_reps: row.get(18)?,
            srs_last_review: row.get(19)?,
        })
    }
}

const VOCAB_SELECT: &str =
    "SELECT id, word, meaning, context, article_path, ebook_id, ebook_cfi, ebook_href,
            document_kind, document_id,
            review_count, last_reviewed_at, created_at, srs_due, srs_stability, srs_difficulty,
            srs_state, srs_lapses, srs_reps, srs_last_review
     FROM vocabulary";

/// 生词Repository
pub struct VocabularyRepository;

impl VocabularyRepository {
    pub fn new() -> Self {
        Self
    }

    /// 获取所有生词
    pub fn find_all(&self, conn: &MutexGuard<Connection>) -> Result<Vec<VocabularyItem>> {
        let sql = format!("{} ORDER BY created_at DESC", VOCAB_SELECT);
        let mut stmt = conn.prepare(&sql)?;

        let items = stmt
            .query_map([], |row| VocabularyItem::from_row(row))?
            .collect::<Result<Vec<_>>>()?;

        Ok(items)
    }

    /// 根据文章ID查找关联生词
    pub fn find_by_article(
        &self,
        conn: &MutexGuard<Connection>,
        article_path: &str,
    ) -> Result<Vec<VocabularyItem>> {
        let sql = format!(
            "{} WHERE article_path = ?1 OR (document_kind = 'article' AND document_id = ?1) ORDER BY created_at DESC",
            VOCAB_SELECT
        );
        let mut stmt = conn.prepare(&sql)?;

        let items = stmt
            .query_map([article_path], |row| VocabularyItem::from_row(row))?
            .collect::<Result<Vec<_>>>()?;

        Ok(items)
    }

    pub fn find_by_ebook(
        &self,
        conn: &MutexGuard<Connection>,
        ebook_id: &str,
    ) -> Result<Vec<VocabularyItem>> {
        let sql = format!(
            "{} WHERE ebook_id = ?1 OR (document_kind = 'ebook' AND document_id = ?1) ORDER BY created_at DESC",
            VOCAB_SELECT
        );
        let mut stmt = conn.prepare(&sql)?;

        let items = stmt
            .query_map([ebook_id], |row| VocabularyItem::from_row(row))?
            .collect::<Result<Vec<_>>>()?;

        Ok(items)
    }

    /// 获取到期待复习的生词
    pub fn find_due(
        &self,
        conn: &MutexGuard<Connection>,
        now_ms: i64,
    ) -> Result<Vec<VocabularyItem>> {
        let sql = format!(
            "{} WHERE (srs_due IS NULL OR srs_due <= ?1) AND srs_state != -1 ORDER BY srs_due ASC",
            VOCAB_SELECT
        );
        let mut stmt = conn.prepare(&sql)?;

        let items = stmt
            .query_map([now_ms], |row| VocabularyItem::from_row(row))?
            .collect::<Result<Vec<_>>>()?;

        Ok(items)
    }

    /// 添加生词
    pub fn create(
        &self,
        conn: &MutexGuard<Connection>,
        req: AddVocabularyRequest,
    ) -> Result<VocabularyItem> {
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
            "INSERT INTO vocabulary (
                id, word, meaning, context, article_path, ebook_id, ebook_cfi, ebook_href, document_kind, document_id, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                id,
                req.word,
                req.meaning,
                req.context,
                req.article_path,
                req.ebook_id,
                req.ebook_cfi,
                req.ebook_href,
                document_kind,
                document_id,
                now
            ],
        )?;

        Ok(VocabularyItem {
            id,
            word: req.word,
            meaning: req.meaning,
            context: req.context,
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

    /// 删除生词
    pub fn delete(&self, conn: &MutexGuard<Connection>, id: &str) -> Result<usize> {
        conn.execute("DELETE FROM vocabulary WHERE id = ?1", [id])
    }

    /// 更新复习状态（旧接口，保持兼容）
    pub fn update_review(&self, conn: &MutexGuard<Connection>, id: &str) -> Result<usize> {
        let now = Utc::now().timestamp_millis();
        conn.execute(
            "UPDATE vocabulary SET review_count = review_count + 1, last_reviewed_at = ?1 WHERE id = ?2",
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
            "UPDATE vocabulary SET
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

    /// 根据文章路径删除所有关联生词
    pub fn delete_by_article(
        &self,
        conn: &MutexGuard<Connection>,
        article_path: &str,
    ) -> Result<usize> {
        conn.execute(
            "DELETE FROM vocabulary WHERE article_path = ?1 OR (document_kind = 'article' AND document_id = ?1)",
            [article_path],
        )
    }

    pub fn delete_by_ebook(&self, conn: &MutexGuard<Connection>, ebook_id: &str) -> Result<usize> {
        conn.execute(
            "DELETE FROM vocabulary WHERE ebook_id = ?1 OR (document_kind = 'ebook' AND document_id = ?1)",
            [ebook_id],
        )
    }

    /// 按 word 分组聚合，用于全局生词本视图
    pub fn find_grouped(&self, conn: &MutexGuard<Connection>) -> Result<Vec<VocabularyGrouped>> {
        let all = self.find_all(conn)?;

        let mut groups: std::collections::HashMap<String, Vec<VocabularyItem>> =
            std::collections::HashMap::new();
        for item in all {
            let key = item.word.to_lowercase();
            groups.entry(key).or_default().push(item);
        }

        let mut result: Vec<VocabularyGrouped> = groups
            .into_iter()
            .map(|(_, mut entries)| {
                entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                let primary_meaning = entries[0].meaning.clone();
                let word = entries[0].word.clone();
                let article_count = entries
                    .iter()
                    .filter_map(|e| e.document_id.as_ref().or(e.article_path.as_ref()))
                    .collect::<std::collections::HashSet<_>>()
                    .len() as u32;
                let total_review_count: i64 = entries.iter().map(|e| e.review_count).sum();
                let last_reviewed_at = entries.iter().filter_map(|e| e.last_reviewed_at).max();

                VocabularyGrouped {
                    word,
                    primary_meaning,
                    article_count,
                    total_review_count,
                    last_reviewed_at,
                    entries,
                }
            })
            .collect();

        result.sort_by(|a, b| {
            let a_time = a.entries.first().map(|e| e.created_at).unwrap_or(0);
            let b_time = b.entries.first().map(|e| e.created_at).unwrap_or(0);
            b_time.cmp(&a_time)
        });

        Ok(result)
    }
}
