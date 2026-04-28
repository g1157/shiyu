// 数据模型定义（前后端共享接口）
// 修改结构体字段时，必须同步更新 src/services/api.ts 中的 TypeScript 接口
use serde::{Deserialize, Serialize};

fn default_article_content_kind() -> String {
    "article".to_string()
}

// ── Vocabulary ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocabularyItem {
    pub id: String,
    pub word: String,
    pub meaning: String,
    pub context: Option<String>,
    pub article_path: Option<String>,
    pub ebook_id: Option<String>,
    pub ebook_cfi: Option<String>,
    pub ebook_href: Option<String>,
    #[serde(default)]
    pub document_kind: Option<String>,
    #[serde(default)]
    pub document_id: Option<String>,
    pub review_count: i64,
    pub last_reviewed_at: Option<i64>,
    pub created_at: i64,
    // FSRS SRS 字段
    pub srs_due: Option<i64>,
    pub srs_stability: f64,
    pub srs_difficulty: f64,
    pub srs_state: i64,
    pub srs_lapses: i64,
    pub srs_reps: i64,
    pub srs_last_review: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct AddVocabularyRequest {
    pub word: String,
    pub meaning: String,
    pub context: Option<String>,
    pub article_path: Option<String>,
    pub ebook_id: Option<String>,
    pub ebook_cfi: Option<String>,
    pub ebook_href: Option<String>,
    #[serde(default)]
    pub document_kind: Option<String>,
    #[serde(default)]
    pub document_id: Option<String>,
}

/// 全局聚合视图：按 word 分组，展示所有释义和来源文章
#[derive(Debug, Clone, Serialize)]
pub struct VocabularyGrouped {
    pub word: String,
    pub primary_meaning: String,
    pub article_count: u32,
    pub total_review_count: i64,
    pub last_reviewed_at: Option<i64>,
    pub entries: Vec<VocabularyItem>,
}

// ── Sentences ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentenceItem {
    pub id: String,
    pub sentence: String,
    pub explanation: String,
    pub article_path: Option<String>,
    pub ebook_id: Option<String>,
    pub ebook_cfi: Option<String>,
    pub ebook_href: Option<String>,
    #[serde(default)]
    pub document_kind: Option<String>,
    #[serde(default)]
    pub document_id: Option<String>,
    pub review_count: i64,
    pub last_reviewed_at: Option<i64>,
    pub created_at: i64,
    // FSRS SRS 字段
    pub srs_due: Option<i64>,
    pub srs_stability: f64,
    pub srs_difficulty: f64,
    pub srs_state: i64,
    pub srs_lapses: i64,
    pub srs_reps: i64,
    pub srs_last_review: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct AddSentenceRequest {
    pub sentence: String,
    pub explanation: String,
    pub article_path: Option<String>,
    pub ebook_id: Option<String>,
    pub ebook_cfi: Option<String>,
    pub ebook_href: Option<String>,
    #[serde(default)]
    pub document_kind: Option<String>,
    #[serde(default)]
    pub document_id: Option<String>,
}

// ── SRS Update ──────────────────────────────────────────

/// 前端传入的 SRS 状态更新请求
#[derive(Debug, Deserialize)]
pub struct UpdateSrsRequest {
    pub id: String,
    pub srs_due: Option<i64>,
    pub srs_stability: f64,
    pub srs_difficulty: f64,
    pub srs_state: i64,
    pub srs_lapses: i64,
    pub srs_reps: i64,
    pub srs_last_review: Option<i64>,
}

// ── Settings ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingItem {
    pub key: String,
    pub value: String,
}

// ── AI Translation ──────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TranslateRequest {
    pub text: String,
    pub prompt_type: String, // "word", "sentence", "complex_sentence"
}

#[derive(Debug, Serialize)]
pub struct TranslateResponse {
    pub result: String,
}

#[derive(Debug, Deserialize)]
pub struct ParagraphItem {
    pub index: usize,
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct ArticleTranslateRequest {
    pub title: Option<String>,
    pub paragraphs: Vec<ParagraphItem>,
}

// ── Ebooks ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EbookItem {
    pub id: String,
    pub title: String,
    pub file_path: String,
    pub author: Option<String>,
    pub format: String,
    pub progress: f64,
    pub cfi_position: Option<String>,
    pub last_read_at: Option<i64>,
    pub created_at: i64,
    pub source_hash: Option<String>,
    pub cover_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEbookProgressRequest {
    pub id: String,
    pub progress: f64,
    pub cfi_position: Option<String>,
}

// ── Articles ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleItem {
    pub id: String,
    pub title: String,
    pub content: String,
    pub author: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub word_count: i64,
    pub created_at: i64,
    #[serde(default = "default_article_content_kind")]
    pub content_kind: String,
    #[serde(default)]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub source_document_id: Option<String>,
    #[serde(default)]
    pub source_document_title: Option<String>,
    #[serde(default)]
    pub source_href: Option<String>,
    #[serde(default)]
    pub source_cfi: Option<String>,
    #[serde(default)]
    pub source_anchor: Option<String>,
    #[serde(default)]
    pub import_source: Option<String>,
    #[serde(default)]
    pub published_at: Option<i64>,
    #[serde(default)]
    pub mindmap_markdown: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddArticleRequest {
    pub title: String,
    pub content: String,
    pub author: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub content_kind: Option<String>,
    #[serde(default)]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub source_document_id: Option<String>,
    #[serde(default)]
    pub source_document_title: Option<String>,
    #[serde(default)]
    pub source_href: Option<String>,
    #[serde(default)]
    pub source_cfi: Option<String>,
    #[serde(default)]
    pub source_anchor: Option<String>,
    #[serde(default)]
    pub import_source: Option<String>,
    #[serde(default)]
    pub published_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateArticleContentRequest {
    pub id: String,
    pub title: String,
    pub content: String,
}

// ── Document Translations ───────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentTranslationItem {
    pub document_kind: String,
    pub document_id: String,
    pub anchor: String,
    pub segment_index: i64,
    pub source_hash: String,
    pub translation: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveDocumentTranslationEntryRequest {
    pub segment_index: i64,
    pub source_hash: String,
    pub translation: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SaveDocumentTranslationsRequest {
    pub document_kind: String,
    pub document_id: String,
    pub anchor: Option<String>,
    pub entries: Vec<SaveDocumentTranslationEntryRequest>,
}

// ── Data Import/Export ───────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportEbookItem {
    pub id: String,
    pub title: String,
    pub author: Option<String>,
    pub format: String,
    pub progress: f64,
    pub cfi_position: Option<String>,
    pub last_read_at: Option<i64>,
    pub created_at: i64,
    pub source_hash: Option<String>,
    pub file_name: String,
    #[serde(default)]
    pub file_data_base64: Option<String>,
    #[serde(default)]
    pub cover_file_name: Option<String>,
    #[serde(default)]
    pub cover_data_base64: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportAssetItem {
    pub relative_path: String,
    pub data_base64: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportData {
    pub vocabulary: Vec<VocabularyItem>,
    pub sentences: Vec<SentenceItem>,
    pub settings: Vec<SettingItem>,
    pub articles: Vec<ArticleItem>,
    #[serde(default)]
    pub ebooks: Vec<ExportEbookItem>,
    #[serde(default)]
    pub assets: Vec<ExportAssetItem>,
    #[serde(default)]
    pub translations: Vec<DocumentTranslationItem>,
}
