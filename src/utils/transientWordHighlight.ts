const ACTIVE_WORD_HIGHLIGHT_CLASS = 'active-lookup-word'
const ACTIVE_WORD_HIGHLIGHT_SUBTLE_CLASS = 'active-lookup-word-subtle'

function escapeRegex(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

function normalizeLookupWord(word: string): string {
  return word.trim().replace(/^[\'"“”‘’]+|[\'"“”‘’.,!?;:()\[\]{}]+$/g, '')
}

function getOwnerDocument(root: ParentNode): Document {
  if (root instanceof Document) return root
  return (root as Node).ownerDocument || document
}

function unwrapMarks(root: ParentNode, selector: string) {
  root.querySelectorAll(selector).forEach((el) => {
    const parent = el.parentNode
    if (!parent) return
    parent.replaceChild(el.ownerDocument.createTextNode(el.textContent || ''), el)
    parent.normalize()
  })
}

export function clearTransientWordHighlights(root: ParentNode | null | undefined) {
  if (!root) return
  unwrapMarks(root, `.${ACTIVE_WORD_HIGHLIGHT_CLASS}, .${ACTIVE_WORD_HIGHLIGHT_SUBTLE_CLASS}`)
}

export function highlightTransientWord(root: ParentNode | null | undefined, word: string) {
  if (!root) return
  clearTransientWordHighlights(root)

  const normalized = normalizeLookupWord(word)
  if (!normalized || !/^[a-z][a-z'-]*$/i.test(normalized)) return

  const doc = getOwnerDocument(root)
  const regex = new RegExp(`\\b(${escapeRegex(normalized)})\\b`, 'gi')
  const walker = doc.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
    acceptNode: (node) => {
      const parent = node.parentElement
      if (!parent || !node.textContent?.trim()) return NodeFilter.FILTER_REJECT
      if (['SCRIPT', 'STYLE', 'TEXTAREA', 'INPUT'].includes(parent.nodeName)) return NodeFilter.FILTER_REJECT
      if (parent.closest('mark, .annotated-word, .annotated-word-subtle, .annotated-sentence, .annotated-sentence-subtle, button, input, textarea, .quick-lookup-panel, .annotation-tooltip')) {
        return NodeFilter.FILTER_REJECT
      }
      regex.lastIndex = 0
      return regex.test(node.textContent) ? NodeFilter.FILTER_ACCEPT : NodeFilter.FILTER_REJECT
    },
  })

  const replacements: Array<{ node: Text; fragment: DocumentFragment }> = []
  let seenFirst = false
  let node: Node | null

  while ((node = walker.nextNode())) {
    const textNode = node as Text
    const text = textNode.textContent || ''
    const fragment = doc.createDocumentFragment()
    let lastIndex = 0
    let hasMatch = false
    let match: RegExpExecArray | null

    regex.lastIndex = 0
    while ((match = regex.exec(text))) {
      const matchedText = match[0]
      if (match.index > lastIndex) {
        fragment.appendChild(doc.createTextNode(text.slice(lastIndex, match.index)))
      }

      const mark = doc.createElement('mark')
      mark.className = seenFirst ? ACTIVE_WORD_HIGHLIGHT_SUBTLE_CLASS : ACTIVE_WORD_HIGHLIGHT_CLASS
      mark.textContent = matchedText
      fragment.appendChild(mark)

      seenFirst = true
      hasMatch = true
      lastIndex = match.index + matchedText.length
    }

    if (!hasMatch) continue
    if (lastIndex < text.length) {
      fragment.appendChild(doc.createTextNode(text.slice(lastIndex)))
    }
    replacements.push({ node: textNode, fragment })
  }

  for (const { node: textNode, fragment } of replacements) {
    const parent = textNode.parentNode
    if (!parent?.contains(textNode)) continue
    parent.replaceChild(fragment, textNode)
  }
}
