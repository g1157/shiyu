import type { ArticleItem, EbookItem, SentenceItem, VocabularyItem } from '../services/api'
import type { DocumentSourceSummary } from '../types/document'

function isNonEmpty(value?: string | null): value is string {
  return typeof value === 'string' && value.trim().length > 0
}

export function getDocumentRef(item: Pick<VocabularyItem | SentenceItem, 'document_kind' | 'document_id' | 'article_path' | 'ebook_id'>) {
  if (isNonEmpty(item.document_kind) && isNonEmpty(item.document_id)) {
    return { kind: item.document_kind, id: item.document_id }
  }
  if (isNonEmpty(item.ebook_id)) {
    return { kind: 'ebook' as const, id: item.ebook_id }
  }
  if (isNonEmpty(item.article_path)) {
    return { kind: 'article' as const, id: item.article_path }
  }
  return null
}

export function resolveDocumentSourceSummary(
  item: Pick<VocabularyItem | SentenceItem, 'document_kind' | 'document_id' | 'article_path' | 'ebook_id'>,
  articles: ArticleItem[],
  ebooks: EbookItem[],
): DocumentSourceSummary | null {
  const ref = getDocumentRef(item)
  if (!ref) return null

  if (ref.kind === 'ebook') {
    const ebook = ebooks.find((entry) => entry.id === ref.id)
    return {
      kind: 'ebook',
      id: ref.id,
      title: ebook?.title,
      label: ebook?.title || '原文图书',
    }
  }

  const article = articles.find((entry) => entry.id === ref.id)
  const sourceLabel = article?.source_document_title || article?.title || '精读材料'
  return {
    kind: 'article',
    id: ref.id,
    title: article?.title,
    label: sourceLabel,
    sourceKind: article?.source_kind,
    sourceDocumentId: article?.source_document_id,
    sourceDocumentTitle: article?.source_document_title,
    sourceHref: article?.source_href,
    sourceCfi: article?.source_cfi,
    sourceAnchor: article?.source_anchor,
  }
}
