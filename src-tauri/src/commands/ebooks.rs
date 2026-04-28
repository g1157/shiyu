use crate::db::Database;
use crate::models::{EbookItem, UpdateEbookProgressRequest};
use crate::repositories::document_translation_repository::DocumentTranslationRepository;
use crate::repositories::ebook_repository::EbookRepository;
use crate::repositories::sentence_repository::SentenceRepository;
use crate::repositories::vocabulary_repository::VocabularyRepository;
use epub::doc::EpubDoc;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tauri::State;

fn resolve_ebooks_dir() -> Result<PathBuf, String> {
    let mut path = dirs::home_dir().ok_or_else(|| "无法定位用户目录".to_string())?;
    path.push(".shiyu");
    path.push("ebooks");
    fs::create_dir_all(&path).map_err(|e| format!("创建图书目录失败: {}", e))?;
    Ok(path)
}

fn resolve_images_dir() -> Result<PathBuf, String> {
    let mut path = dirs::home_dir().ok_or_else(|| "无法定位用户目录".to_string())?;
    path.push(".shiyu");
    path.push("images");
    fs::create_dir_all(&path).map_err(|e| format!("创建图片目录失败: {}", e))?;
    Ok(path)
}

fn normalize_file_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn compute_sha256(file_path: &Path) -> Result<String, String> {
    let mut file = File::open(file_path).map_err(|e| format!("打开文件失败: {}", e))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| format!("读取文件失败: {}", e))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn image_extension(mime: &str) -> &'static str {
    match mime {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/svg+xml" => "svg",
        "image/webp" => "webp",
        _ => "png",
    }
}

fn save_epub_cover(file_path: &Path) -> Result<Option<String>, String> {
    let path_str = file_path.to_string_lossy().to_string();
    let mut doc = EpubDoc::new(&path_str).map_err(|e| format!("无法读取 EPUB 封面: {}", e))?;
    let Some((data, mime)) = doc.get_cover() else {
        return Ok(None);
    };

    let hash = Sha256::digest(&data);
    let hash_hex: String = hash.iter().take(8).map(|b| format!("{:02x}", b)).collect();
    let file_name = format!("ebook-cover-{}.{}", hash_hex, image_extension(&mime));
    let cover_path = resolve_images_dir()?.join(file_name);

    if !cover_path.exists() {
        let mut file =
            File::create(&cover_path).map_err(|e| format!("保存 EPUB 封面失败: {}", e))?;
        file.write_all(&data)
            .map_err(|e| format!("写入 EPUB 封面失败: {}", e))?;
    }

    Ok(Some(normalize_file_path(&cover_path)))
}

fn read_epub_metadata(file_path: &Path) -> Result<(String, Option<String>), String> {
    let path_str = file_path.to_string_lossy().to_string();
    let doc = EpubDoc::new(&path_str).map_err(|e| format!("无法读取 EPUB 元数据: {}", e))?;

    let fallback_title = file_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "未命名图书".to_string());

    let title = doc.get_title().unwrap_or_default().trim().to_string();
    let author = doc
        .mdata("creator")
        .map(|item| item.value.trim().to_string())
        .unwrap_or_default();

    Ok((
        if title.is_empty() {
            fallback_title
        } else {
            title
        },
        if author.is_empty() {
            None
        } else {
            Some(author)
        },
    ))
}

#[tauri::command]
pub fn get_ebooks(db: State<Database>) -> Result<Vec<EbookItem>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = EbookRepository::new();
    let items = repo.find_all(&conn).map_err(|e| e.to_string())?;

    items
        .into_iter()
        .map(|item| {
            if item.cover_path.is_some() || item.file_path.is_empty() {
                return Ok(item);
            }

            match save_epub_cover(Path::new(&item.file_path)) {
                Ok(Some(cover_path)) => repo
                    .update_cover_path(&conn, &item.id, Some(cover_path))
                    .map_err(|e| e.to_string()),
                Ok(None) | Err(_) => Ok(item),
            }
        })
        .collect()
}

#[tauri::command]
pub fn get_ebook(id: String, db: State<Database>) -> Result<EbookItem, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = EbookRepository::new();
    repo.find_by_id(&conn, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Ebook not found".to_string())
}

#[tauri::command]
pub fn import_epub_as_book(file_path: String, db: State<Database>) -> Result<EbookItem, String> {
    let source_path = PathBuf::from(&file_path);
    if !source_path.exists() {
        return Err("EPUB 文件不存在".to_string());
    }

    let source_hash = compute_sha256(&source_path)?;

    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = EbookRepository::new();

    if let Some(existing) = repo
        .find_by_source_hash(&conn, &source_hash)
        .map_err(|e| e.to_string())?
    {
        return Ok(existing);
    }

    let (title, author) = read_epub_metadata(&source_path)?;
    let cover_path = save_epub_cover(&source_path)?;
    let books_dir = resolve_ebooks_dir()?;
    let file_name = format!("{}.epub", uuid::Uuid::new_v4());
    let managed_path = books_dir.join(file_name);

    fs::copy(&source_path, &managed_path).map_err(|e| format!("复制 EPUB 文件失败: {}", e))?;

    repo.create(
        &conn,
        title,
        normalize_file_path(&managed_path),
        author,
        "epub".to_string(),
        Some(source_hash),
        cover_path,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_ebook_progress(
    req: UpdateEbookProgressRequest,
    db: State<Database>,
) -> Result<EbookItem, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = EbookRepository::new();
    repo.update_progress(&conn, req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_ebook(id: String, db: State<Database>) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let repo = EbookRepository::new();
    let vocab_repo = VocabularyRepository::new();
    let sentence_repo = SentenceRepository::new();
    let translation_repo = DocumentTranslationRepository::new();
    let existing = repo.find_by_id(&conn, &id).map_err(|e| e.to_string())?;

    if let Some(ebook) = existing {
        vocab_repo
            .delete_by_ebook(&conn, &id)
            .map_err(|e| e.to_string())?;
        sentence_repo
            .delete_by_ebook(&conn, &id)
            .map_err(|e| e.to_string())?;
        translation_repo
            .delete_by_document(&conn, "ebook", &id)
            .map_err(|e| e.to_string())?;
        repo.delete(&conn, &id).map_err(|e| e.to_string())?;
        if !ebook.file_path.is_empty() {
            let path = PathBuf::from(&ebook.file_path);
            if path.exists() {
                fs::remove_file(path).map_err(|e| format!("删除图书文件失败: {}", e))?;
            }
        }
        Ok(())
    } else {
        Err("Ebook not found".to_string())
    }
}
