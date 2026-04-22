type SanitizerMode = 'rich' | 'parsed'

const DROP_WITH_CONTENT = new Set([
  'script',
  'style',
  'iframe',
  'object',
  'embed',
  'form',
  'button',
  'input',
  'textarea',
  'select',
  'option',
  'link',
  'meta',
  'base',
])

const RICH_TAGS = new Set([
  'a',
  'blockquote',
  'br',
  'code',
  'del',
  'div',
  'em',
  'h1',
  'h2',
  'h3',
  'h4',
  'h5',
  'h6',
  'hr',
  'img',
  'li',
  'ol',
  'p',
  'pre',
  'span',
  'strong',
  'sub',
  'sup',
  'table',
  'tbody',
  'td',
  'th',
  'thead',
  'tr',
  'ul',
])

const PARSED_TAGS = new Set(['br', 'span'])

const GLOBAL_ATTRS = new Set(['class', 'id', 'title'])

const TAG_ATTRS: Record<string, Set<string>> = {
  a: new Set(['href', 'target', 'rel', 'title']),
  img: new Set(['src', 'alt', 'title']),
  ol: new Set(['start']),
  td: new Set(['colspan', 'rowspan']),
  th: new Set(['colspan', 'rowspan']),
}

function getAllowedTags(mode: SanitizerMode): Set<string> {
  return mode === 'parsed' ? PARSED_TAGS : RICH_TAGS
}

function sanitizeId(value: string): string {
  return value.replace(/[^A-Za-z0-9:_-]/g, '').slice(0, 120)
}

function sanitizeClassList(value: string, mode: SanitizerMode): string {
  const items = value
    .split(/\s+/)
    .map((item) => item.trim())
    .filter(Boolean)

  const filtered = mode === 'parsed'
    ? items.filter((item) => /^ps-[a-z0-9-]+$/i.test(item))
    : items.filter((item) => /^[A-Za-z0-9:_-]+$/.test(item))

  return filtered.join(' ')
}

function isSafeUrl(value: string, attrName: 'href' | 'src'): boolean {
  const trimmed = value.trim()
  if (!trimmed) return false

  if (attrName === 'href' && trimmed.startsWith('#')) {
    return true
  }

  if (/^(https?:|mailto:|tel:|asset:|tauri:|blob:|about:blank)/i.test(trimmed)) {
    return true
  }

  if (attrName === 'src' && /^data:image\//i.test(trimmed)) {
    return true
  }

  if (/^(\/|\.\/|\.\.\/)/.test(trimmed)) {
    return true
  }

  return false
}

function sanitizeElementAttributes(element: Element, mode: SanitizerMode) {
  const tag = element.tagName.toLowerCase()
  const tagAttrs = TAG_ATTRS[tag] || new Set<string>()
  const attrs = Array.from(element.attributes)

  for (const attr of attrs) {
    const name = attr.name.toLowerCase()
    const value = attr.value

    if (name.startsWith('on') || name === 'style') {
      element.removeAttribute(attr.name)
      continue
    }

    if (!GLOBAL_ATTRS.has(name) && !tagAttrs.has(name)) {
      element.removeAttribute(attr.name)
      continue
    }

    if (name === 'class') {
      const sanitized = sanitizeClassList(value, mode)
      if (sanitized) {
        element.setAttribute('class', sanitized)
      } else {
        element.removeAttribute('class')
      }
      continue
    }

    if (name === 'id') {
      const sanitized = sanitizeId(value)
      if (sanitized) {
        element.setAttribute('id', sanitized)
      } else {
        element.removeAttribute('id')
      }
      continue
    }

    if (name === 'href' || name === 'src') {
      if (!isSafeUrl(value, name)) {
        element.removeAttribute(attr.name)
      } else {
        element.setAttribute(attr.name, value.trim())
      }
      continue
    }

    if ((name === 'colspan' || name === 'rowspan' || name === 'start') && !/^\d+$/.test(value.trim())) {
      element.removeAttribute(attr.name)
      continue
    }

    if (name === 'target') {
      if (!['_blank', '_self'].includes(value.trim())) {
        element.removeAttribute(attr.name)
      }
      continue
    }
  }

  if (tag === 'a' && element.getAttribute('target') === '_blank') {
    element.setAttribute('rel', 'noopener noreferrer')
  }
}

function unwrapElement(element: Element) {
  const parent = element.parentNode
  if (!parent) return

  while (element.firstChild) {
    parent.insertBefore(element.firstChild, element)
  }
  parent.removeChild(element)
}

function sanitizeNodeChildren(root: ParentNode, mode: SanitizerMode) {
  const allowedTags = getAllowedTags(mode)
  const children = Array.from(root.childNodes)

  for (const child of children) {
    if (child.nodeType === Node.COMMENT_NODE) {
      child.parentNode?.removeChild(child)
      continue
    }

    if (child.nodeType !== Node.ELEMENT_NODE) {
      continue
    }

    const element = child as Element
    const tag = element.tagName.toLowerCase()

    if (!allowedTags.has(tag)) {
      if (DROP_WITH_CONTENT.has(tag)) {
        element.remove()
      } else {
        unwrapElement(element)
      }
      continue
    }

    sanitizeElementAttributes(element, mode)
    sanitizeNodeChildren(element, mode)
  }
}

function sanitizeHtmlInternal(html: string, mode: SanitizerMode): string {
  const raw = (html || '').trim()
  if (!raw) return ''
  if (typeof DOMParser === 'undefined') return raw

  const doc = new DOMParser().parseFromString(raw, 'text/html')
  sanitizeNodeChildren(doc.body, mode)
  return doc.body.innerHTML
}

export function sanitizeRichHtml(html: string): string {
  return sanitizeHtmlInternal(html, 'rich')
}

export function sanitizeParsedSentenceHtml(html: string): string {
  return sanitizeHtmlInternal(html, 'parsed')
}
