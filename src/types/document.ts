export type DocumentKind = 'article' | 'ebook'

export type ArticleContentKind = 'article' | 'epub_excerpt' | 'epub_compilation' | 'ocr_excerpt'

export interface DocumentRef {
  kind: DocumentKind
  id: string
}

export interface DocumentSourceSummary {
  kind: DocumentKind
  id: string
  title?: string
  label: string
  sourceKind?: string
  sourceDocumentId?: string
  sourceDocumentTitle?: string
  sourceHref?: string
  sourceCfi?: string
  sourceAnchor?: string
}

export interface DocumentTranslationItem {
  document_kind: DocumentKind
  document_id: string
  anchor: string
  segment_index: number
  source_hash: string
  translation: string
  updated_at: number
}

export interface SaveDocumentTranslationEntryRequest {
  segment_index: number
  source_hash: string
  translation: string
}

export interface SaveDocumentTranslationsRequest {
  document_kind: DocumentKind
  document_id: string
  anchor?: string
  entries: SaveDocumentTranslationEntryRequest[]
}
