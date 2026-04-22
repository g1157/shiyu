import { sanitizeParsedSentenceHtml } from './sanitizeHtml'

export interface InlineSentenceTranslationPayload {
  translation: string
  parsedHtml?: string
  structureNote?: string
}

export const INLINE_SENTENCE_TRANSLATION_ATTR = 'data-inline-sentence-translation'
export const INLINE_SENTENCE_TRANSLATION_VALUE = 'quick'
export const INLINE_SENTENCE_TRANSLATION_SELECTOR =
  `[${INLINE_SENTENCE_TRANSLATION_ATTR}="${INLINE_SENTENCE_TRANSLATION_VALUE}"]`

const BLOCK_SELECTOR = 'p, li, blockquote, h1, h2, h3, h4, h5, h6, div'

function resolveNodeElement(node: Node | null): Element | null {
  if (!node) return null
  if (node.nodeType === Node.ELEMENT_NODE) {
    return node as Element
  }
  return node.parentElement
}

function resolveBlockAnchor(element: Element | null): HTMLElement | null {
  if (!element) return null

  const directBlock = element.closest(BLOCK_SELECTOR)
  if (directBlock && directBlock.nodeType === Node.ELEMENT_NODE) {
    return directBlock as HTMLElement
  }

  let current: Element | null = element
  while (current) {
    const parent: HTMLElement | null = current.parentElement
    if (!parent || parent.tagName === 'BODY') {
      return current as HTMLElement
    }
    current = parent
  }

  return null
}

export function resolveInlineSentenceAnchor(range: Range | null): HTMLElement | null {
  if (!range) return null

  const candidates = [
    resolveBlockAnchor(resolveNodeElement(range.startContainer)),
    resolveBlockAnchor(resolveNodeElement(range.endContainer)),
    resolveBlockAnchor(resolveNodeElement(range.commonAncestorContainer)),
  ]

  for (const candidate of candidates) {
    if (candidate) {
      return candidate
    }
  }

  return null
}

export function isInlineSentenceTranslationBlock(node: Element | null): node is HTMLElement {
  return (
    !!node
    && node.nodeType === Node.ELEMENT_NODE
    && node.getAttribute(INLINE_SENTENCE_TRANSLATION_ATTR) === INLINE_SENTENCE_TRANSLATION_VALUE
  )
}

export function buildInlineSentenceTranslationBlock(
  doc: Document,
  payload: InlineSentenceTranslationPayload,
): HTMLElement {
  const block = doc.createElement('section')
  block.className = 'inline-sentence-translation'
  block.setAttribute(INLINE_SENTENCE_TRANSLATION_ATTR, INLINE_SENTENCE_TRANSLATION_VALUE)

  const header = doc.createElement('div')
  header.className = 'inline-sentence-translation__header'

  const badge = doc.createElement('span')
  badge.className = 'inline-sentence-translation__badge'
  badge.textContent = '句下快译'
  header.appendChild(badge)

  const closeButton = doc.createElement('button')
  closeButton.type = 'button'
  closeButton.className = 'inline-sentence-translation__close'
  closeButton.textContent = '收起'
  closeButton.setAttribute('aria-label', '收起句下快译')
  header.appendChild(closeButton)

  block.appendChild(header)

  const translation = doc.createElement('div')
  translation.className = 'inline-sentence-translation__translation'
  translation.textContent = payload.translation.trim()
  block.appendChild(translation)

  if (payload.parsedHtml?.trim()) {
    const parsed = doc.createElement('div')
    parsed.className = 'inline-sentence-translation__parsed parsed-html-content'
    parsed.innerHTML = sanitizeParsedSentenceHtml(payload.parsedHtml)
    block.appendChild(parsed)
  }

  if (payload.structureNote?.trim()) {
    const note = doc.createElement('div')
    note.className = 'inline-sentence-translation__note'
    note.textContent = payload.structureNote.trim()
    block.appendChild(note)
  }

  return block
}
