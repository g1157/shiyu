mod commands;
mod db;
mod models;
mod repositories;
mod secure_settings;

use commands::ai::{test_api_connection, translate_article_stream, translate_text};
use commands::articles::{
    add_article, delete_article, get_article, get_article_mindmap, get_articles,
    save_article_mindmap, update_article,
};
use commands::config_pack::import_config_pack;
use commands::data::{export_all_data, export_data_to_file, import_data, import_data_from_file};
use commands::ebooks::{
    delete_ebook, get_ebook, get_ebooks, import_epub_as_book, update_ebook_progress,
};
use commands::epub_tools::{extract_epub_chapter, extract_epub_chapters, parse_epub_toc};
use commands::ocr::{ocr_extract_pages, ocr_refine_with_ai};
use commands::sentences::{
    add_sentence, delete_sentence, get_due_sentences, get_sentences, get_sentences_by_article,
    get_sentences_by_ebook, update_sentence_review, update_sentence_srs,
};
use commands::settings::{delete_setting, get_all_settings, get_setting, set_setting};
use commands::vocabulary::{
    add_vocabulary, delete_vocabulary, get_due_vocabulary, get_vocabulary,
    get_vocabulary_by_article, get_vocabulary_by_ebook, get_vocabulary_grouped,
    update_vocabulary_review, update_vocabulary_srs,
};
use db::Database;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let database = Database::new().expect("Failed to initialize database");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(database)
        .invoke_handler(tauri::generate_handler![
            // Vocabulary
            get_vocabulary,
            get_vocabulary_by_article,
            get_vocabulary_by_ebook,
            get_vocabulary_grouped,
            add_vocabulary,
            delete_vocabulary,
            update_vocabulary_review,
            get_due_vocabulary,
            update_vocabulary_srs,
            // Sentences
            get_sentences,
            get_sentences_by_article,
            get_sentences_by_ebook,
            add_sentence,
            delete_sentence,
            update_sentence_review,
            get_due_sentences,
            update_sentence_srs,
            // Settings
            get_setting,
            set_setting,
            get_all_settings,
            delete_setting,
            // AI
            translate_text,
            translate_article_stream,
            test_api_connection,
            // Data
            export_all_data,
            export_data_to_file,
            import_data,
            import_data_from_file,
            // Articles
            get_articles,
            get_article,
            add_article,
            update_article,
            delete_article,
            get_article_mindmap,
            save_article_mindmap,
            // Ebooks
            get_ebooks,
            get_ebook,
            import_epub_as_book,
            update_ebook_progress,
            delete_ebook,
            // EPUB Tools
            parse_epub_toc,
            extract_epub_chapter,
            extract_epub_chapters,
            // OCR
            ocr_extract_pages,
            ocr_refine_with_ai,
            // Config Pack
            import_config_pack,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
