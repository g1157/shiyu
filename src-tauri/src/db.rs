// 数据库初始化与迁移
use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;

const CURRENT_SCHEMA_VERSION: i32 = 7;

pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    pub fn new() -> Result<Self> {
        let db_path = Self::resolve_db_path();

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(&db_path)?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;
        Self::initialize_tables(&conn)?;
        Self::run_migrations(&conn)?;
        Self::ensure_indexes(&conn)?;
        Self::check_version_announcement(&conn);

        Ok(Database {
            conn: Mutex::new(conn),
        })
    }

    /// 版本变更时自动清除 last_seen_version，确保版本公告弹窗在新版本首次启动时显示
    fn check_version_announcement(conn: &Connection) {
        let current = env!("CARGO_PKG_VERSION");
        let stored: Option<String> = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'last_seen_version'",
                [],
                |row| row.get(0),
            )
            .ok();
        if stored.as_deref() != Some(current) && stored.is_some() {
            conn.execute("DELETE FROM settings WHERE key = 'last_seen_version'", [])
                .ok();
        }
    }

    fn resolve_db_path() -> PathBuf {
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(".shiyu");
        path.push("shiyu.db");
        path
    }

    /// 获取当前schema版本
    fn get_schema_version(conn: &Connection) -> Result<i32> {
        let version: i32 = conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?;
        Ok(version)
    }

    /// 运行数据库迁移
    fn run_migrations(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at INTEGER NOT NULL
            )",
            [],
        )?;

        let current_version = Self::get_schema_version(conn)?;

        if current_version < CURRENT_SCHEMA_VERSION {
            // 应用迁移
            Self::apply_migration(conn, current_version)?;

            // 更新版本记录
            let now = chrono::Utc::now().timestamp_millis();
            conn.execute(
                "INSERT OR REPLACE INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
                rusqlite::params![CURRENT_SCHEMA_VERSION, now],
            )?;
        }

        Ok(())
    }

    fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
        let sql = format!("PRAGMA table_info({table})");
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let existing: String = row.get(1)?;
            if existing == column {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn add_column_if_missing(
        conn: &Connection,
        table: &str,
        column: &str,
        definition: &str,
    ) -> Result<()> {
        if Self::column_exists(conn, table, column)? {
            return Ok(());
        }

        let sql = format!("ALTER TABLE {table} ADD COLUMN {definition}");
        conn.execute(&sql, [])?;
        Ok(())
    }

    fn backfill_document_refs(conn: &Connection, table: &str) -> Result<()> {
        let sql = format!(
            "UPDATE {table}
             SET document_kind = CASE
                    WHEN document_kind IS NOT NULL AND TRIM(document_kind) != '' THEN document_kind
                    WHEN ebook_id IS NOT NULL AND TRIM(ebook_id) != '' THEN 'ebook'
                    WHEN article_path IS NOT NULL AND TRIM(article_path) != '' THEN 'article'
                    ELSE NULL
                 END,
                 document_id = CASE
                    WHEN document_id IS NOT NULL AND TRIM(document_id) != '' THEN document_id
                    WHEN ebook_id IS NOT NULL AND TRIM(ebook_id) != '' THEN ebook_id
                    WHEN article_path IS NOT NULL AND TRIM(article_path) != '' THEN article_path
                    ELSE NULL
                 END"
        );
        conn.execute(&sql, [])?;
        Ok(())
    }

    /// 应用具体的迁移（逐版本递进）
    fn apply_migration(conn: &Connection, from_version: i32) -> Result<()> {
        if from_version < 1 {
            conn.execute_batch(
                "
                CREATE INDEX IF NOT EXISTS idx_vocabulary_article_path ON vocabulary(article_path);
                CREATE INDEX IF NOT EXISTS idx_sentences_article_path ON sentences(article_path);
                CREATE INDEX IF NOT EXISTS idx_articles_created_at ON articles(created_at);
                ",
            )?;
        }

        if from_version < 2 {
            // v2: 添加 FSRS 间隔重复字段
            Self::add_column_if_missing(conn, "vocabulary", "srs_due", "srs_due INTEGER")?;
            Self::add_column_if_missing(
                conn,
                "vocabulary",
                "srs_stability",
                "srs_stability REAL NOT NULL DEFAULT 0",
            )?;
            Self::add_column_if_missing(
                conn,
                "vocabulary",
                "srs_difficulty",
                "srs_difficulty REAL NOT NULL DEFAULT 0",
            )?;
            Self::add_column_if_missing(
                conn,
                "vocabulary",
                "srs_state",
                "srs_state INTEGER NOT NULL DEFAULT 0",
            )?;
            Self::add_column_if_missing(
                conn,
                "vocabulary",
                "srs_lapses",
                "srs_lapses INTEGER NOT NULL DEFAULT 0",
            )?;
            Self::add_column_if_missing(
                conn,
                "vocabulary",
                "srs_reps",
                "srs_reps INTEGER NOT NULL DEFAULT 0",
            )?;
            Self::add_column_if_missing(
                conn,
                "vocabulary",
                "srs_last_review",
                "srs_last_review INTEGER",
            )?;

            Self::add_column_if_missing(conn, "sentences", "srs_due", "srs_due INTEGER")?;
            Self::add_column_if_missing(
                conn,
                "sentences",
                "srs_stability",
                "srs_stability REAL NOT NULL DEFAULT 0",
            )?;
            Self::add_column_if_missing(
                conn,
                "sentences",
                "srs_difficulty",
                "srs_difficulty REAL NOT NULL DEFAULT 0",
            )?;
            Self::add_column_if_missing(
                conn,
                "sentences",
                "srs_state",
                "srs_state INTEGER NOT NULL DEFAULT 0",
            )?;
            Self::add_column_if_missing(
                conn,
                "sentences",
                "srs_lapses",
                "srs_lapses INTEGER NOT NULL DEFAULT 0",
            )?;
            Self::add_column_if_missing(
                conn,
                "sentences",
                "srs_reps",
                "srs_reps INTEGER NOT NULL DEFAULT 0",
            )?;
            Self::add_column_if_missing(
                conn,
                "sentences",
                "srs_last_review",
                "srs_last_review INTEGER",
            )?;

            conn.execute_batch(
                "
                CREATE INDEX IF NOT EXISTS idx_vocabulary_srs_due ON vocabulary(srs_due);
                CREATE INDEX IF NOT EXISTS idx_sentences_srs_due ON sentences(srs_due);
                ",
            )?;
        }

        if from_version < 3 {
            // v3: 思维导图持久化
            Self::add_column_if_missing(
                conn,
                "articles",
                "mindmap_markdown",
                "mindmap_markdown TEXT",
            )?;
        }

        if from_version < 4 {
            // v4: 图书导入去重与书架支持
            Self::add_column_if_missing(conn, "ebooks", "source_hash", "source_hash TEXT")?;
            conn.execute_batch(
                "
                CREATE UNIQUE INDEX IF NOT EXISTS idx_ebooks_source_hash ON ebooks(source_hash);
                CREATE INDEX IF NOT EXISTS idx_ebooks_last_read_at ON ebooks(last_read_at);
                ",
            )?;
        }

        if from_version < 5 {
            // v5: 生词/句子支持图书锚点，便于回跳 EPUB 原文
            Self::add_column_if_missing(conn, "vocabulary", "ebook_id", "ebook_id TEXT")?;
            Self::add_column_if_missing(conn, "vocabulary", "ebook_cfi", "ebook_cfi TEXT")?;
            Self::add_column_if_missing(conn, "vocabulary", "ebook_href", "ebook_href TEXT")?;
            Self::add_column_if_missing(conn, "sentences", "ebook_id", "ebook_id TEXT")?;
            Self::add_column_if_missing(conn, "sentences", "ebook_cfi", "ebook_cfi TEXT")?;
            Self::add_column_if_missing(conn, "sentences", "ebook_href", "ebook_href TEXT")?;

            conn.execute_batch(
                "
                CREATE INDEX IF NOT EXISTS idx_vocabulary_ebook_id ON vocabulary(ebook_id);
                CREATE INDEX IF NOT EXISTS idx_sentences_ebook_id ON sentences(ebook_id);
                ",
            )?;
        }

        if from_version < 6 {
            Self::add_column_if_missing(
                conn,
                "articles",
                "content_kind",
                "content_kind TEXT NOT NULL DEFAULT 'article'",
            )?;
            Self::add_column_if_missing(conn, "articles", "source_kind", "source_kind TEXT")?;
            Self::add_column_if_missing(
                conn,
                "articles",
                "source_document_id",
                "source_document_id TEXT",
            )?;
            Self::add_column_if_missing(
                conn,
                "articles",
                "source_document_title",
                "source_document_title TEXT",
            )?;
            Self::add_column_if_missing(conn, "articles", "source_href", "source_href TEXT")?;
            Self::add_column_if_missing(conn, "articles", "source_cfi", "source_cfi TEXT")?;
            Self::add_column_if_missing(conn, "articles", "source_anchor", "source_anchor TEXT")?;
            Self::add_column_if_missing(conn, "articles", "import_source", "import_source TEXT")?;
            Self::add_column_if_missing(conn, "articles", "published_at", "published_at INTEGER")?;

            Self::add_column_if_missing(conn, "vocabulary", "document_kind", "document_kind TEXT")?;
            Self::add_column_if_missing(conn, "vocabulary", "document_id", "document_id TEXT")?;
            Self::add_column_if_missing(conn, "sentences", "document_kind", "document_kind TEXT")?;
            Self::add_column_if_missing(conn, "sentences", "document_id", "document_id TEXT")?;

            Self::backfill_document_refs(conn, "vocabulary")?;
            Self::backfill_document_refs(conn, "sentences")?;

            conn.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS document_translations (
                    document_kind TEXT NOT NULL,
                    document_id TEXT NOT NULL,
                    anchor TEXT NOT NULL DEFAULT '',
                    segment_index INTEGER NOT NULL,
                    source_hash TEXT NOT NULL,
                    translation TEXT NOT NULL,
                    updated_at INTEGER NOT NULL,
                    PRIMARY KEY (document_kind, document_id, anchor, segment_index)
                );

                CREATE INDEX IF NOT EXISTS idx_document_translations_document
                    ON document_translations(document_kind, document_id, anchor);
                CREATE INDEX IF NOT EXISTS idx_document_translations_updated_at
                    ON document_translations(updated_at);
                CREATE INDEX IF NOT EXISTS idx_vocabulary_document_ref
                    ON vocabulary(document_kind, document_id);
                CREATE INDEX IF NOT EXISTS idx_sentences_document_ref
                    ON sentences(document_kind, document_id);
                CREATE INDEX IF NOT EXISTS idx_articles_source_document
                    ON articles(source_kind, source_document_id);
                ",
            )?;
        }

        if from_version < 7 {
            Self::add_column_if_missing(conn, "ebooks", "cover_path", "cover_path TEXT")?;
        }

        Ok(())
    }

    fn initialize_tables(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS vocabulary (
                id TEXT PRIMARY KEY,
                word TEXT NOT NULL,
                meaning TEXT NOT NULL,
                context TEXT,
                article_path TEXT,
                ebook_id TEXT,
                ebook_cfi TEXT,
                ebook_href TEXT,
                document_kind TEXT,
                document_id TEXT,
                review_count INTEGER NOT NULL DEFAULT 0,
                last_reviewed_at INTEGER,
                created_at INTEGER NOT NULL,
                srs_due INTEGER,
                srs_stability REAL NOT NULL DEFAULT 0,
                srs_difficulty REAL NOT NULL DEFAULT 0,
                srs_state INTEGER NOT NULL DEFAULT 0,
                srs_lapses INTEGER NOT NULL DEFAULT 0,
                srs_reps INTEGER NOT NULL DEFAULT 0,
                srs_last_review INTEGER
            );

            CREATE UNIQUE INDEX IF NOT EXISTS idx_vocabulary_word_path
                ON vocabulary(word, article_path);

            CREATE TABLE IF NOT EXISTS sentences (
                id TEXT PRIMARY KEY,
                sentence TEXT NOT NULL,
                explanation TEXT NOT NULL,
                article_path TEXT,
                ebook_id TEXT,
                ebook_cfi TEXT,
                ebook_href TEXT,
                document_kind TEXT,
                document_id TEXT,
                review_count INTEGER NOT NULL DEFAULT 0,
                last_reviewed_at INTEGER,
                created_at INTEGER NOT NULL,
                srs_due INTEGER,
                srs_stability REAL NOT NULL DEFAULT 0,
                srs_difficulty REAL NOT NULL DEFAULT 0,
                srs_state INTEGER NOT NULL DEFAULT 0,
                srs_lapses INTEGER NOT NULL DEFAULT 0,
                srs_reps INTEGER NOT NULL DEFAULT 0,
                srs_last_review INTEGER
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS articles (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                author TEXT,
                category TEXT,
                description TEXT,
                word_count INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                content_kind TEXT NOT NULL DEFAULT 'article',
                source_kind TEXT,
                source_document_id TEXT,
                source_document_title TEXT,
                source_href TEXT,
                source_cfi TEXT,
                source_anchor TEXT,
                import_source TEXT,
                published_at INTEGER,
                mindmap_markdown TEXT
            );

            CREATE TABLE IF NOT EXISTS ebooks (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                file_path TEXT NOT NULL,
                author TEXT,
                format TEXT NOT NULL DEFAULT 'epub',
                progress REAL NOT NULL DEFAULT 0.0,
                cfi_position TEXT,
                last_read_at INTEGER,
                created_at INTEGER NOT NULL,
                source_hash TEXT,
                cover_path TEXT
            );

            CREATE TABLE IF NOT EXISTS document_translations (
                document_kind TEXT NOT NULL,
                document_id TEXT NOT NULL,
                anchor TEXT NOT NULL DEFAULT '',
                segment_index INTEGER NOT NULL,
                source_hash TEXT NOT NULL,
                translation TEXT NOT NULL,
                updated_at INTEGER NOT NULL,
                PRIMARY KEY (document_kind, document_id, anchor, segment_index)
            );

            -- Schema版本表（用于迁移）
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at INTEGER NOT NULL
            );

            -- 创建常用索引
            CREATE INDEX IF NOT EXISTS idx_vocabulary_article_path ON vocabulary(article_path);
            CREATE INDEX IF NOT EXISTS idx_sentences_article_path ON sentences(article_path);
            CREATE INDEX IF NOT EXISTS idx_articles_created_at ON articles(created_at);
            ",
        )?;
        Ok(())
    }

    fn ensure_indexes(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "
            CREATE UNIQUE INDEX IF NOT EXISTS idx_vocabulary_word_path
                ON vocabulary(word, article_path);
            CREATE INDEX IF NOT EXISTS idx_vocabulary_article_path ON vocabulary(article_path);
            CREATE INDEX IF NOT EXISTS idx_sentences_article_path ON sentences(article_path);
            CREATE INDEX IF NOT EXISTS idx_vocabulary_srs_due ON vocabulary(srs_due);
            CREATE INDEX IF NOT EXISTS idx_sentences_srs_due ON sentences(srs_due);
            CREATE INDEX IF NOT EXISTS idx_vocabulary_ebook_id ON vocabulary(ebook_id);
            CREATE INDEX IF NOT EXISTS idx_sentences_ebook_id ON sentences(ebook_id);
            CREATE INDEX IF NOT EXISTS idx_vocabulary_document_ref ON vocabulary(document_kind, document_id);
            CREATE INDEX IF NOT EXISTS idx_sentences_document_ref ON sentences(document_kind, document_id);
            CREATE INDEX IF NOT EXISTS idx_articles_created_at ON articles(created_at);
            CREATE INDEX IF NOT EXISTS idx_articles_source_document ON articles(source_kind, source_document_id);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_ebooks_source_hash ON ebooks(source_hash);
            CREATE INDEX IF NOT EXISTS idx_ebooks_last_read_at ON ebooks(last_read_at);
            CREATE INDEX IF NOT EXISTS idx_document_translations_document
                ON document_translations(document_kind, document_id, anchor);
            CREATE INDEX IF NOT EXISTS idx_document_translations_updated_at
                ON document_translations(updated_at);
            ",
        )?;
        Ok(())
    }
}
