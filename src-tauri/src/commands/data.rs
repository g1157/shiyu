// 数据导入/导出
use crate::db::Database;
use crate::models::{
    ArticleItem, DocumentTranslationItem, ExportAssetItem, ExportData, ExportEbookItem,
    SentenceItem, SettingItem, VocabularyItem,
};
use crate::secure_settings::delete_setting_value;
use base64::Engine;
use regex::Regex;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;

const IMAGE_ASSET_SCHEME: &str = "shiyu-asset://images/";

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn resolve_shiyu_dir() -> Result<PathBuf, String> {
    let mut path = dirs::home_dir().ok_or_else(|| "无法定位用户目录".to_string())?;
    path.push(".shiyu");
    fs::create_dir_all(&path).map_err(|e| format!("创建数据目录失败: {}", e))?;
    Ok(path)
}

fn resolve_images_dir() -> Result<PathBuf, String> {
    let path = resolve_shiyu_dir()?.join("images");
    fs::create_dir_all(&path).map_err(|e| format!("创建图片目录失败: {}", e))?;
    Ok(path)
}

fn resolve_ebooks_dir() -> Result<PathBuf, String> {
    let path = resolve_shiyu_dir()?.join("ebooks");
    fs::create_dir_all(&path).map_err(|e| format!("创建图书目录失败: {}", e))?;
    Ok(path)
}

fn should_export_setting(key: &str) -> bool {
    let normalized = key.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return false;
    }

    if matches!(normalized.as_str(), "api_key" | "ocr_api_token") {
        return false;
    }

    !normalized.contains("secret")
        && !normalized.contains("password")
        && !normalized.ends_with("_token")
        && !normalized.ends_with("_key")
}

fn normalize_article_content_for_export(content: &str) -> (String, Vec<String>) {
    let image_path_re = Regex::new(r#"[^)"'\s<>]*\.shiyu/images/(?P<file>[^)"'\s<>]+)"#)
        .expect("valid image regex");

    let mut file_names = BTreeSet::new();
    let normalized = image_path_re
        .replace_all(content, |caps: &regex::Captures| {
            let file_name = caps["file"].to_string();
            file_names.insert(file_name.clone());
            format!("{IMAGE_ASSET_SCHEME}{file_name}")
        })
        .to_string();

    (normalized, file_names.into_iter().collect())
}

fn restore_article_content_assets(content: &str) -> Result<String, String> {
    if !content.contains(IMAGE_ASSET_SCHEME) {
        return Ok(content.to_string());
    }

    let images_dir = normalize_path(&resolve_images_dir()?);
    Ok(content.replace(IMAGE_ASSET_SCHEME, &format!("{images_dir}/")))
}

fn encode_file_to_base64(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    Some(base64::engine::general_purpose::STANDARD.encode(bytes))
}

fn decode_base64_to_bytes(data: &str) -> Result<Vec<u8>, String> {
    base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|e| format!("Base64 解码失败: {}", e))
}

fn clear_directory(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(path).map_err(|e| format!("读取目录失败: {}", e))? {
        let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            fs::remove_dir_all(&entry_path).map_err(|e| format!("删除目录失败: {}", e))?;
        } else {
            fs::remove_file(&entry_path).map_err(|e| format!("删除文件失败: {}", e))?;
        }
    }

    Ok(())
}

fn ebook_exists(tx: &rusqlite::Transaction<'_>, item: &ExportEbookItem) -> Result<bool, String> {
    let exists: i64 = tx
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM ebooks
                WHERE id = ?1
                   OR (?2 IS NOT NULL AND source_hash = ?2)
            )",
            rusqlite::params![item.id, item.source_hash],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(exists != 0)
}

fn collect_export_data(conn: &rusqlite::Connection) -> Result<ExportData, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, word, meaning, context, article_path, ebook_id, ebook_cfi, ebook_href,
                    document_kind, document_id,
                    review_count, last_reviewed_at, created_at, srs_due, srs_stability, srs_difficulty,
                    srs_state, srs_lapses, srs_reps, srs_last_review
             FROM vocabulary",
        )
        .map_err(|e| e.to_string())?;
    let vocabulary = stmt
        .query_map([], |row| {
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
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, sentence, explanation, article_path, ebook_id, ebook_cfi, ebook_href,
                    document_kind, document_id,
                    review_count, last_reviewed_at, created_at, srs_due, srs_stability, srs_difficulty,
                    srs_state, srs_lapses, srs_reps, srs_last_review
             FROM sentences",
        )
        .map_err(|e| e.to_string())?;
    let sentences = stmt
        .query_map([], |row| {
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
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT key, value FROM settings")
        .map_err(|e| e.to_string())?;
    let settings = stmt
        .query_map([], |row| {
            Ok(SettingItem {
                key: row.get(0)?,
                value: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|item| should_export_setting(&item.key))
        .collect::<Vec<_>>();

    let mut stmt = conn
        .prepare(
            "SELECT id, title, content, author, category, description, word_count, created_at,
                    content_kind, source_kind, source_document_id, source_document_title,
                    source_href, source_cfi, source_anchor, import_source, published_at, mindmap_markdown
             FROM articles ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;
    let raw_articles = stmt
        .query_map([], |row| {
            Ok(ArticleItem {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                author: row.get(3)?,
                category: row.get(4)?,
                description: row.get(5)?,
                word_count: row.get(6)?,
                created_at: row.get(7)?,
                content_kind: row.get(8)?,
                source_kind: row.get(9)?,
                source_document_id: row.get(10)?,
                source_document_title: row.get(11)?,
                source_href: row.get(12)?,
                source_cfi: row.get(13)?,
                source_anchor: row.get(14)?,
                import_source: row.get(15)?,
                published_at: row.get(16)?,
                mindmap_markdown: row.get(17)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut referenced_image_files = BTreeSet::new();
    let articles = raw_articles
        .into_iter()
        .map(|mut item| {
            let (normalized_content, file_names) =
                normalize_article_content_for_export(&item.content);
            item.content = normalized_content;
            for file_name in file_names {
                referenced_image_files.insert(file_name);
            }
            item
        })
        .collect::<Vec<_>>();

    let mut stmt = conn
        .prepare(
            "SELECT document_kind, document_id, anchor, segment_index, source_hash, translation, updated_at
             FROM document_translations
             ORDER BY document_kind, document_id, anchor, segment_index",
        )
        .map_err(|e| e.to_string())?;
    let translations = stmt
        .query_map([], |row| {
            Ok(DocumentTranslationItem {
                document_kind: row.get(0)?,
                document_id: row.get(1)?,
                anchor: row.get(2)?,
                segment_index: row.get(3)?,
                source_hash: row.get(4)?,
                translation: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, title, file_path, author, format, progress, cfi_position, last_read_at, created_at, source_hash, cover_path
             FROM ebooks
             ORDER BY COALESCE(last_read_at, created_at) DESC, created_at DESC",
        )
        .map_err(|e| e.to_string())?;
    let ebooks = stmt
        .query_map([], |row| {
            let file_path: String = row.get(2)?;
            let path = PathBuf::from(&file_path);
            let cover_path: Option<String> = row.get(10)?;
            let cover_file = cover_path.as_ref().map(PathBuf::from);
            let file_name = path
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|| {
                    let format: String = row.get(4).unwrap_or_else(|_| "epub".to_string());
                    format!("{}.{}", row.get::<_, String>(0).unwrap_or_default(), format)
                });
            let cover_file_name = cover_file.as_ref().and_then(|path| {
                path.file_name()
                    .map(|value| value.to_string_lossy().to_string())
            });
            let cover_data_base64 = cover_file
                .as_ref()
                .and_then(|path| encode_file_to_base64(path));

            Ok(ExportEbookItem {
                id: row.get(0)?,
                title: row.get(1)?,
                author: row.get(3)?,
                format: row.get(4)?,
                progress: row.get(5)?,
                cfi_position: row.get(6)?,
                last_read_at: row.get(7)?,
                created_at: row.get(8)?,
                source_hash: row.get(9)?,
                file_name,
                file_data_base64: encode_file_to_base64(&path),
                cover_file_name,
                cover_data_base64,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let images_dir = resolve_images_dir()?;
    let assets = referenced_image_files
        .into_iter()
        .filter_map(|file_name| {
            let path = images_dir.join(&file_name);
            encode_file_to_base64(&path).map(|data_base64| ExportAssetItem {
                relative_path: format!("images/{}", file_name),
                data_base64,
            })
        })
        .collect::<Vec<_>>();

    Ok(ExportData {
        vocabulary,
        sentences,
        settings,
        articles,
        ebooks,
        assets,
        translations,
    })
}

fn apply_import_data(
    conn: &mut rusqlite::Connection,
    data: ExportData,
    mode: &str,
) -> Result<String, String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    if mode == "replace" {
        tx.execute_batch("DELETE FROM vocabulary; DELETE FROM sentences; DELETE FROM settings; DELETE FROM articles; DELETE FROM ebooks; DELETE FROM document_translations;")
            .map_err(|e| e.to_string())?;
        delete_setting_value(&tx, "api_key")?;
        delete_setting_value(&tx, "ocr_api_token")?;
        clear_directory(&resolve_ebooks_dir()?)?;
        clear_directory(&resolve_images_dir()?)?;
    }

    let mut vocab_count = 0;
    for item in &data.vocabulary {
        let result = tx.execute(
            "INSERT OR IGNORE INTO vocabulary (
                id, word, meaning, context, article_path, ebook_id, ebook_cfi, ebook_href,
                document_kind, document_id,
                review_count, last_reviewed_at, created_at, srs_due, srs_stability, srs_difficulty,
                srs_state, srs_lapses, srs_reps, srs_last_review
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
            rusqlite::params![
                item.id,
                item.word,
                item.meaning,
                item.context,
                item.article_path,
                item.ebook_id,
                item.ebook_cfi,
                item.ebook_href,
                item.document_kind,
                item.document_id,
                item.review_count,
                item.last_reviewed_at,
                item.created_at,
                item.srs_due,
                item.srs_stability,
                item.srs_difficulty,
                item.srs_state,
                item.srs_lapses,
                item.srs_reps,
                item.srs_last_review
            ],
        );
        if let Ok(changed) = result {
            vocab_count += changed;
        }
    }

    let mut sentence_count = 0;
    for item in &data.sentences {
        let result = tx.execute(
            "INSERT OR IGNORE INTO sentences (
                id, sentence, explanation, article_path, ebook_id, ebook_cfi, ebook_href,
                document_kind, document_id,
                review_count, last_reviewed_at, created_at, srs_due, srs_stability, srs_difficulty,
                srs_state, srs_lapses, srs_reps, srs_last_review
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            rusqlite::params![
                item.id,
                item.sentence,
                item.explanation,
                item.article_path,
                item.ebook_id,
                item.ebook_cfi,
                item.ebook_href,
                item.document_kind,
                item.document_id,
                item.review_count,
                item.last_reviewed_at,
                item.created_at,
                item.srs_due,
                item.srs_stability,
                item.srs_difficulty,
                item.srs_state,
                item.srs_lapses,
                item.srs_reps,
                item.srs_last_review
            ],
        );
        if let Ok(changed) = result {
            sentence_count += changed;
        }
    }

    for item in &data.settings {
        tx.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
            rusqlite::params![item.key, item.value],
        )
        .ok();
    }

    let mut article_count = 0;
    for item in &data.articles {
        let restored_content = restore_article_content_assets(&item.content)?;
        let result = tx.execute(
            "INSERT OR IGNORE INTO articles (
                id, title, content, author, category, description, word_count, created_at,
                content_kind, source_kind, source_document_id, source_document_title,
                source_href, source_cfi, source_anchor, import_source, published_at, mindmap_markdown
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            rusqlite::params![
                item.id,
                item.title,
                restored_content,
                item.author,
                item.category,
                item.description,
                item.word_count,
                item.created_at,
                item.content_kind,
                item.source_kind,
                item.source_document_id,
                item.source_document_title,
                item.source_href,
                item.source_cfi,
                item.source_anchor,
                item.import_source,
                item.published_at,
                item.mindmap_markdown
            ],
        );
        if let Ok(changed) = result {
            article_count += changed;
        }
    }

    let images_dir = resolve_images_dir()?;
    for asset in &data.assets {
        let relative = asset.relative_path.replace('\\', "/");
        let image_name = relative
            .strip_prefix("images/")
            .unwrap_or(relative.as_str());
        if image_name.trim().is_empty() {
            continue;
        }

        let output_path = images_dir.join(image_name);
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("创建图片目录失败: {}", e))?;
        }
        let bytes = decode_base64_to_bytes(&asset.data_base64)?;
        fs::write(&output_path, bytes).map_err(|e| format!("写入图片失败: {}", e))?;
    }

    let ebooks_dir = resolve_ebooks_dir()?;
    let mut ebook_count = 0;
    for item in &data.ebooks {
        if mode != "replace" && ebook_exists(&tx, item)? {
            continue;
        }

        let output_path = ebooks_dir.join(&item.file_name);
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("创建图书目录失败: {}", e))?;
        }

        if let Some(file_data) = &item.file_data_base64 {
            let bytes = decode_base64_to_bytes(file_data)?;
            fs::write(&output_path, bytes).map_err(|e| format!("写入图书文件失败: {}", e))?;
        } else if !output_path.exists() {
            continue;
        }

        let cover_path = if let (Some(cover_file_name), Some(cover_data)) =
            (&item.cover_file_name, &item.cover_data_base64)
        {
            let cover_output_path = resolve_images_dir()?.join(cover_file_name);
            if let Some(parent) = cover_output_path.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("创建封面目录失败: {}", e))?;
            }
            let bytes = decode_base64_to_bytes(cover_data)?;
            fs::write(&cover_output_path, bytes).map_err(|e| format!("写入封面文件失败: {}", e))?;
            Some(normalize_path(&cover_output_path))
        } else {
            None
        };

        let result = tx.execute(
            "INSERT OR IGNORE INTO ebooks (id, title, file_path, author, format, progress, cfi_position, last_read_at, created_at, source_hash, cover_path) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                item.id,
                item.title,
                normalize_path(&output_path),
                item.author,
                item.format,
                item.progress,
                item.cfi_position,
                item.last_read_at,
                item.created_at,
                item.source_hash,
                cover_path
            ],
        );
        if let Ok(changed) = result {
            ebook_count += changed;
        }
    }

    let mut translation_count = 0;
    for item in &data.translations {
        let result = tx.execute(
            "INSERT OR REPLACE INTO document_translations (
                document_kind, document_id, anchor, segment_index, source_hash, translation, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                item.document_kind,
                item.document_id,
                item.anchor,
                item.segment_index,
                item.source_hash,
                item.translation,
                item.updated_at,
            ],
        );
        if let Ok(changed) = result {
            translation_count += changed;
        }
    }

    tx.commit().map_err(|e| e.to_string())?;

    Ok(format!(
        "导入完成：{} 个生词，{} 个句子，{} 个设置项，{} 篇文章，{} 本图书，{} 条翻译，{} 个资源文件",
        vocab_count,
        sentence_count,
        data.settings.len(),
        article_count,
        ebook_count,
        translation_count,
        data.assets.len()
    ))
}

#[tauri::command]
pub fn export_all_data(db: State<Database>) -> Result<ExportData, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    collect_export_data(&conn)
}

#[tauri::command]
pub fn export_data_to_file(db: State<Database>, file_path: String) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let data = collect_export_data(&conn)?;
    let json =
        serde_json::to_string_pretty(&data).map_err(|e| format!("序列化备份数据失败: {}", e))?;

    fs::write(&file_path, json).map_err(|e| format!("写入备份文件失败: {}", e))?;
    Ok(file_path)
}

#[tauri::command]
pub fn import_data(db: State<Database>, data: ExportData, mode: String) -> Result<String, String> {
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    apply_import_data(&mut conn, data, &mode)
}

#[tauri::command]
pub fn import_data_from_file(
    db: State<Database>,
    file_path: String,
    mode: String,
) -> Result<String, String> {
    let content = fs::read_to_string(&file_path).map_err(|e| format!("读取备份文件失败: {}", e))?;
    let data: ExportData =
        serde_json::from_str(&content).map_err(|e| format!("解析备份文件失败: {}", e))?;

    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;
    apply_import_data(&mut conn, data, &mode)
}

#[cfg(test)]
mod tests {
    use super::{normalize_article_content_for_export, should_export_setting, IMAGE_ASSET_SCHEME};

    #[test]
    fn filters_sensitive_settings_from_export() {
        assert!(should_export_setting("theme"));
        assert!(should_export_setting("api_model"));
        assert!(!should_export_setting("api_key"));
        assert!(!should_export_setting("ocr_api_token"));
        assert!(!should_export_setting("db_password"));
        assert!(!should_export_setting("session_token"));
        assert!(!should_export_setting("some_secret"));
        assert!(!should_export_setting("custom_key"));
    }

    #[test]
    fn rewrites_local_image_paths_into_backup_scheme() {
        let content = concat!(
            "![cover](C:/Users/demo/.shiyu/images/cover.png)\n",
            "<img src=\"file:///C:/Users/demo/.shiyu/images/inline-photo.jpg\" />\n",
            "<img src=\"C:/Users/demo/.shiyu/images/cover.png\" />"
        );

        let (normalized, files) = normalize_article_content_for_export(content);

        assert_eq!(files, vec!["cover.png", "inline-photo.jpg"]);
        assert!(normalized.contains(&format!("{IMAGE_ASSET_SCHEME}cover.png")));
        assert!(normalized.contains(&format!("{IMAGE_ASSET_SCHEME}inline-photo.jpg")));
        assert!(!normalized.contains(".shiyu/images/"));
    }
}
