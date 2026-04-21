<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { convertFileSrc } from '@tauri-apps/api/core'
import ePub from 'epubjs'
import {
  addSentence,
  addVocabulary,
  getEbook,
  getSentencesByEbook,
  getVocabularyByEbook,
  updateEbookProgress,
  type EbookItem,
  type SentenceItem,
  type VocabularyItem,
} from '../services/api'
import AnnotationForm from './AnnotationForm.vue'
import QuickLookupPanel from './QuickLookupPanel.vue'
import SelectionPopover from './SelectionPopover.vue'
import { useAnnotationInteraction } from '../composables/useAnnotationInteraction'
import { useGlobalToast } from '../composables/useGlobalToast'
import type { PopoverPosition, SelectionState } from '../composables/useTextSelection'
import type { HighlightType } from '../composables/useRouteQuery'
import { preCacheText } from '../services/ttsCache'
import { useAppStore } from '../stores/appStore'

interface TocNode {
  id?: string
  label: string
  href: string
  subitems: TocNode[]
}

interface FlatTocNode extends TocNode {
  depth: number
}

const props = defineProps<{
  ebook: EbookItem
  focusCfi?: string | null
  highlightId?: string | null
  highlightType?: HighlightType | null
}>()

const emit = defineEmits<{
  close: []
  updated: [ebook: EbookItem]
}>()

const router = useRouter()
const toast = useGlobalToast()
const appStore = useAppStore()
const readerHostRef = ref<HTMLElement | null>(null)
const currentEbook = ref<EbookItem | null>(null)
const loading = ref(true)
const error = ref<string | null>(null)
const toc = ref<TocNode[]>([])
const showToc = ref(true)
const currentChapter = ref('')
const annotationEnabled = ref(true)
const vocabulary = ref<VocabularyItem[]>([])
const sentences = ref<SentenceItem[]>([])
const toastRef = ref<{ show: (message: string) => void } | null>({
  show(message: string) {
    toast.success(message)
  },
})
const selection = ref<SelectionState>({
  text: '',
  type: null,
  range: null,
  rect: null,
})
const popoverPosition = ref<PopoverPosition>({
  top: 0,
  left: 0,
  visible: false,
})
const selectedContextText = ref('')
const selectedCfi = ref<string | null>(null)
const selectedHref = ref<string | null>(null)
const {
  showAnnotationForm,
  annotationType,
  cachedSelectedText,
  cachedContextText,
  annotationDraft,
  quickLookupVisible,
  quickLookupType,
  quickLookupAnchor,
  quickLookupSelectedText,
  quickLookupContextText,
  quickLookupWordPos,
  quickLookupMeaning,
  quickLookupTranslation,
  quickLookupParsedHtml,
  quickLookupStructureNote,
  quickLookupLoading,
  quickLookupDeepLoading,
  quickLookupSaving,
  quickLookupError,
  quickLookupDeepError,
  handleAddWord,
  handleAddSentence,
  handleCloseForm,
  handleSaveAnnotation,
  closeQuickLookup,
  retryQuickLookup,
  requestSentenceDeepAnalysis,
  openQuickLookupEditor,
  saveQuickLookup,
} = useAnnotationInteraction(
  router,
  toastRef,
  { selection, clearSelection, getContext, popoverPosition },
  { annotationEnabled, findWordById, findSentenceById, saveWord, saveSentence },
)

let bookInstance: any = null
let rendition: any = null
let progressTimer: ReturnType<typeof setTimeout> | null = null
let pendingProgress = 0
let pendingCfi: string | undefined
let contentCleanupFns: Array<() => void> = []
let renderedAnnotations: Array<{ cfi: string; type: 'highlight' | 'underline' }> = []
const activeHighlightId = ref<string | null>(null)
const activeHighlightType = ref<HighlightType | null>(null)
let highlightResetTimer: ReturnType<typeof setTimeout> | null = null

const progressPercent = computed(() =>
  Math.round(((currentEbook.value?.progress || 0) * 1000)) / 10
)

const flatToc = computed<FlatTocNode[]>(() => {
  const walk = (nodes: TocNode[], depth = 0): FlatTocNode[] =>
    nodes.flatMap((node) => [
      { ...node, depth },
      ...walk(node.subitems, depth + 1),
    ])

  return walk(toc.value)
})

function findWordById(id: string) {
  return vocabulary.value.find((item) => item.id === id)
}

function findSentenceById(id: string) {
  return sentences.value.find((item) => item.id === id)
}

function clearActiveHighlight(resetTimer = true) {
  activeHighlightId.value = null
  activeHighlightType.value = null
  if (resetTimer && highlightResetTimer) {
    clearTimeout(highlightResetTimer)
    highlightResetTimer = null
  }
}

function setActiveHighlight(id?: string | null, type?: HighlightType | null) {
  clearActiveHighlight()
  if (!id || !type) return

  activeHighlightId.value = id
  activeHighlightType.value = type
  highlightResetTimer = setTimeout(() => {
    clearActiveHighlight(false)
    syncBookAnnotations()
  }, 3200)
}

async function loadBookAnnotations() {
  if (!currentEbook.value) {
    vocabulary.value = []
    sentences.value = []
    return
  }

  try {
    const [words, sentenceItems] = await Promise.all([
      getVocabularyByEbook(currentEbook.value.id),
      getSentencesByEbook(currentEbook.value.id),
    ])
    vocabulary.value = words
    sentences.value = sentenceItems
  } catch (e) {
    console.error('加载图书标注失败:', e)
    vocabulary.value = []
    sentences.value = []
  }
}

async function saveWord(word: string, meaning: string, context: string) {
  const item = await addVocabulary({
    word,
    meaning,
    context: context || undefined,
    ebook_id: currentEbook.value?.id,
    ebook_cfi: selectedCfi.value || undefined,
    ebook_href: selectedHref.value || undefined,
  })
  vocabulary.value.unshift(item)
  appStore.addVocabularyItem(item)
  preCacheText(word, '-10%')
  setActiveHighlight(item.id, 'word')
  return item
}

async function saveSentence(sentence: string, explanation: string) {
  const item = await addSentence({
    sentence,
    explanation,
    ebook_id: currentEbook.value?.id,
    ebook_cfi: selectedCfi.value || undefined,
    ebook_href: selectedHref.value || undefined,
  })
  sentences.value.unshift(item)
  appStore.addSentenceItem(item)
  preCacheText(sentence, '+0%')
  setActiveHighlight(item.id, 'sentence')
  return item
}

function getRenderedContents(): any[] {
  const contents = rendition?.getContents?.()
  if (Array.isArray(contents)) return contents
  return contents ? [contents] : []
}

function clearNativeSelections() {
  for (const contents of getRenderedContents()) {
    try {
      contents?.window?.getSelection?.()?.removeAllRanges?.()
    } catch {}
  }
}

function clearSelection() {
  selection.value = {
    text: '',
    type: null,
    range: null,
    rect: null,
  }
  selectedContextText.value = ''
  popoverPosition.value.visible = false
  clearNativeSelections()
}

function getContext() {
  return selectedContextText.value
}

function normalizeWhitespace(text: string): string {
  return text.replace(/\s+/g, ' ').trim()
}

function detectSelectionType(text: string): 'word' | 'sentence' {
  const trimmed = text.trim()
  const wordCount = trimmed.split(/\s+/).length
  const hasSentenceEnder = /[.!?。！？]/.test(trimmed)
  if (wordCount <= 3 && !hasSentenceEnder) {
    return 'word'
  }
  return 'sentence'
}

function isSentenceBoundaryChar(char: string): boolean {
  return /[.!?。！？]/.test(char) || char === '\n'
}

function getSentenceRange(paragraphText: string, selectionStart: number, selectionEnd: number) {
  let start = 0
  for (let i = Math.max(0, selectionStart - 1); i >= 0; i -= 1) {
    if (isSentenceBoundaryChar(paragraphText[i])) {
      start = i + 1
      break
    }
  }

  let end = paragraphText.length
  for (let i = Math.max(selectionEnd, start); i < paragraphText.length; i += 1) {
    if (isSentenceBoundaryChar(paragraphText[i])) {
      end = i + 1
      break
    }
  }

  return { start, end }
}

function getSelectionOffsetsInBlock(range: Range, block: Element) {
  try {
    const startRange = range.cloneRange()
    startRange.selectNodeContents(block)
    startRange.setEnd(range.startContainer, range.startOffset)
    const start = startRange.toString().length

    const endRange = range.cloneRange()
    endRange.selectNodeContents(block)
    endRange.setEnd(range.endContainer, range.endOffset)
    const end = endRange.toString().length

    if (!Number.isFinite(start) || !Number.isFinite(end)) {
      return null
    }
    return { start, end }
  } catch {
    return null
  }
}

function findContextBlock(range: Range): Element | null {
  const anchor =
    range.commonAncestorContainer.nodeType === Node.ELEMENT_NODE
      ? (range.commonAncestorContainer as Element)
      : range.commonAncestorContainer.parentElement
  if (!anchor?.closest) return null
  return anchor.closest('p, li, blockquote, h1, h2, h3, h4, h5, h6, div')
}

function extractContext(range: Range, fallbackText: string): string {
  const block = findContextBlock(range)
  if (!block) return fallbackText

  const blockText = block.textContent || ''
  if (!blockText.trim()) return fallbackText

  const offsets = getSelectionOffsetsInBlock(range, block)
  if (!offsets) {
    return normalizeWhitespace(blockText) || fallbackText
  }

  const sentenceRange = getSentenceRange(blockText, offsets.start, offsets.end)
  const sentence = normalizeWhitespace(blockText.slice(sentenceRange.start, sentenceRange.end))
  return sentence || normalizeWhitespace(blockText) || fallbackText
}

function setPopoverPosition(range: Range, contents: any) {
  const rect = range.getBoundingClientRect()
  const frameElement = contents?.document?.defaultView?.frameElement as HTMLElement | null
  const frameRect = frameElement?.getBoundingClientRect()
  const top = frameRect ? frameRect.top + rect.top : rect.top
  const bottom = frameRect ? frameRect.top + rect.bottom : rect.bottom
  const left = frameRect ? frameRect.left + rect.left : rect.left

  let topPosition = top - 50
  if (topPosition < 60) {
    topPosition = bottom + 10
  }

  const rawLeft = left + rect.width / 2
  const clampedLeft = Math.max(32, Math.min(window.innerWidth - 32, rawLeft))

  popoverPosition.value = {
    top: topPosition,
    left: clampedLeft,
    visible: true,
  }
}

function handleSelected(_cfiRange: string, contents: any) {
  const nativeSelection = contents?.window?.getSelection?.()
  if (!nativeSelection || nativeSelection.isCollapsed || nativeSelection.rangeCount === 0) {
    clearSelection()
    return
  }

  const range = nativeSelection.getRangeAt(0).cloneRange()
  const text = nativeSelection.toString().trim()
  if (!text) {
    clearSelection()
    return
  }

  selection.value = {
    text,
    type: detectSelectionType(text),
    range,
    rect: range.getBoundingClientRect(),
  }
  selectedCfi.value = _cfiRange || null
  selectedHref.value = contents?.location?.start?.href || currentLocationHref() || null
  selectedContextText.value = extractContext(range, normalizeWhitespace(text))
  setPopoverPosition(range, contents)
}

function attachContentInteraction(contents: any) {
  const dismissIfCollapsed = () => {
    setTimeout(() => {
      const nativeSelection = contents?.window?.getSelection?.()
      const hasSelection =
        nativeSelection && !nativeSelection.isCollapsed && nativeSelection.toString().trim()
      if (!hasSelection && popoverPosition.value.visible) {
        clearSelection()
      }
    }, 10)
  }

  const onPointerDown = () => {
    if (popoverPosition.value.visible) {
      clearSelection()
    }
  }

  contents.document.addEventListener('mouseup', dismissIfCollapsed)
  contents.document.addEventListener('touchend', dismissIfCollapsed)
  contents.document.addEventListener('mousedown', onPointerDown)
  contents.document.addEventListener('touchstart', onPointerDown)

  contentCleanupFns.push(() => {
    try {
      contents.document.removeEventListener('mouseup', dismissIfCollapsed)
      contents.document.removeEventListener('touchend', dismissIfCollapsed)
      contents.document.removeEventListener('mousedown', onPointerDown)
      contents.document.removeEventListener('touchstart', onPointerDown)
    } catch {}
  })
}

function normalizeHref(href?: string): string {
  return (href || '').split('#')[0]
}

function normalizeToc(nodes: any[] = []): TocNode[] {
  return nodes.map((node) => ({
    id: node.id,
    label: node.label || node.title || '未命名章节',
    href: node.href || '',
    subitems: normalizeToc(node.subitems || node.children || []),
  }))
}

function flattenToc(nodes: TocNode[]): TocNode[] {
  return nodes.flatMap((node) => [node, ...flattenToc(node.subitems)])
}

function resolveChapterLabel(href?: string): string {
  const target = normalizeHref(href)
  if (!target) return ''

  const matched = flattenToc(toc.value).find((node) => {
    const nodeHref = normalizeHref(node.href)
    return nodeHref === target || nodeHref.endsWith(target) || target.endsWith(nodeHref)
  })

  return matched?.label || ''
}

function currentLocationHref() {
  const location = rendition?.currentLocation?.()
  if (location?.start?.href) {
    return location.start.href as string
  }
  return null
}

function syncBookUpdate(updated: EbookItem) {
  currentEbook.value = updated
  emit('updated', updated)
}

function clearRenderedAnnotations() {
  if (!rendition?.annotations) {
    renderedAnnotations = []
    return
  }

  for (const entry of renderedAnnotations) {
    try {
      rendition.annotations.remove(entry.cfi, entry.type)
    } catch {}
  }
  renderedAnnotations = []
}

function syncBookAnnotations() {
  if (!rendition?.annotations || !currentEbook.value) return

  clearRenderedAnnotations()

  const seen = new Set<string>()

  for (const item of vocabulary.value) {
    if (!item.ebook_cfi) continue
    const key = `highlight:${item.ebook_cfi}`
    if (seen.has(key)) continue
    seen.add(key)
    const isFocused = activeHighlightId.value === item.id && activeHighlightType.value === 'word'
    try {
      rendition.annotations.highlight(
        item.ebook_cfi,
        { id: item.id, type: 'word' },
        undefined,
        isFocused ? 'book-word-highlight book-word-highlight--focus' : 'book-word-highlight',
        {
          fill: isFocused ? '#16a34a' : '#22c55e',
          'fill-opacity': isFocused ? '0.42' : '0.22',
          'mix-blend-mode': 'multiply',
          'pointer-events': 'none',
        },
      )
      renderedAnnotations.push({ cfi: item.ebook_cfi, type: 'highlight' })
    } catch (e) {
      console.warn('恢复图书单词高亮失败:', e)
    }
  }

  for (const item of sentences.value) {
    if (!item.ebook_cfi) continue
    const key = `underline:${item.ebook_cfi}`
    if (seen.has(key)) continue
    seen.add(key)
    const isFocused = activeHighlightId.value === item.id && activeHighlightType.value === 'sentence'
    try {
      rendition.annotations.underline(
        item.ebook_cfi,
        { id: item.id, type: 'sentence' },
        undefined,
        isFocused ? 'book-sentence-highlight book-sentence-highlight--focus' : 'book-sentence-highlight',
        {
          stroke: isFocused ? '#7c3aed' : '#8b5cf6',
          'stroke-opacity': '0.92',
          'stroke-width': isFocused ? '3.2' : '1.8',
          'pointer-events': 'none',
        },
      )
      renderedAnnotations.push({ cfi: item.ebook_cfi, type: 'underline' })
    } catch (e) {
      console.warn('恢复图书句子高亮失败:', e)
    }
  }
}

function queueProgressSave(progress: number, cfiPosition?: string) {
  if (!currentEbook.value) return

  pendingProgress = progress
  pendingCfi = cfiPosition

  if (progressTimer) clearTimeout(progressTimer)
  progressTimer = setTimeout(async () => {
    if (!currentEbook.value) return
    try {
      const updated = await updateEbookProgress({
        id: currentEbook.value.id,
        progress: pendingProgress,
        cfi_position: pendingCfi,
      })
      syncBookUpdate(updated)
    } catch (e) {
      console.error('保存图书进度失败:', e)
    }
  }, 500)
}

function handleRelocated(location: any) {
  const start = location?.start
  const progress = typeof start?.percentage === 'number' ? start.percentage : 0
  currentChapter.value = resolveChapterLabel(start?.href)
  queueProgressSave(progress, start?.cfi || undefined)
}

function cleanupReader() {
  if (progressTimer) {
    clearTimeout(progressTimer)
    progressTimer = null
  }
  clearActiveHighlight()

  clearSelection()
  selectedCfi.value = null
  selectedHref.value = null
  closeQuickLookup()
  if (showAnnotationForm.value) {
    handleCloseForm()
  }

  for (const cleanup of contentCleanupFns) {
    try {
      cleanup()
    } catch {}
  }
  contentCleanupFns = []
  clearRenderedAnnotations()

  try {
    rendition?.off?.('relocated', handleRelocated)
  } catch {}

  try {
    rendition?.off?.('selected', handleSelected)
  } catch {}

  try {
    rendition?.destroy?.()
  } catch {}
  rendition = null

  try {
    bookInstance?.destroy?.()
  } catch {}
  bookInstance = null

  if (readerHostRef.value) {
    readerHostRef.value.innerHTML = ''
  }
}

async function initReader() {
  loading.value = true
  error.value = null
  toc.value = []
  currentChapter.value = ''
  cleanupReader()

  try {
    const latest = await getEbook(props.ebook.id).catch(() => props.ebook)
    currentEbook.value = latest
    await loadBookAnnotations()

    await nextTick()
    if (!readerHostRef.value || !currentEbook.value) {
      throw new Error('阅读容器未初始化')
    }

    const assetUrl = convertFileSrc(currentEbook.value.file_path)
    bookInstance = ePub(assetUrl)

    const navigation = await bookInstance.loaded.navigation
    toc.value = normalizeToc(navigation?.toc || [])
    currentChapter.value = toc.value[0]?.label || ''

    rendition = bookInstance.renderTo(readerHostRef.value, {
      width: '100%',
      height: '100%',
      flow: 'paginated',
      spread: 'none',
      allowScriptedContent: true,
    })

    rendition.hooks.content.register((contents: any) => {
      attachContentInteraction(contents)
    })
    rendition.on('relocated', handleRelocated)
    rendition.on('selected', handleSelected)
    await rendition.display(props.focusCfi || currentEbook.value.cfi_position || undefined)
    setActiveHighlight(props.highlightId, props.highlightType)
    syncBookAnnotations()
  } catch (e: any) {
    error.value = e?.message || String(e)
    toast.error('打开图书失败: ' + error.value)
  } finally {
    loading.value = false
  }
}

async function openChapter(item: TocNode) {
  if (!rendition || !item.href) return
  clearSelection()
  await rendition.display(item.href)
  currentChapter.value = item.label
}

function prevPage() {
  clearSelection()
  rendition?.prev?.()
}

function nextPage() {
  clearSelection()
  rendition?.next?.()
}

onMounted(() => {
  void initReader()
})

onUnmounted(() => {
  cleanupReader()
})

watch(() => props.ebook.id, () => {
  void initReader()
})

watch(
  () => [
    vocabulary.value,
    sentences.value,
    currentEbook.value?.id,
    activeHighlightId.value,
    activeHighlightType.value,
  ] as const,
  () => {
    syncBookAnnotations()
  },
  { deep: true },
)

watch(
  () => [props.highlightId, props.highlightType] as const,
  ([newId, newType], [oldId, oldType]) => {
    if (newId === oldId && newType === oldType) return
    setActiveHighlight(newId, newType)
  },
)

watch(
  () => props.focusCfi,
  (newCfi, oldCfi) => {
    if (!newCfi || newCfi === oldCfi || !rendition) return
    clearSelection()
    setActiveHighlight(props.highlightId, props.highlightType)
    void rendition.display(newCfi)
  },
)
</script>

<template>
  <section class="book-reader">
    <header class="reader-header">
      <div class="reader-title-block">
        <button class="header-btn" @click="emit('close')">← 返回书架</button>
        <div class="reader-title-wrap">
          <h1 class="reader-title">{{ currentEbook?.title || ebook.title }}</h1>
          <p class="reader-subtitle">
            <span v-if="currentEbook?.author">{{ currentEbook.author }}</span>
            <span>{{ currentChapter || '正在加载章节' }}</span>
            <span>{{ progressPercent }}%</span>
          </p>
        </div>
      </div>
      <div class="reader-actions">
        <button class="header-btn" @click="showToc = !showToc">{{ showToc ? '隐藏目录' : '显示目录' }}</button>
        <button class="header-btn" @click="prevPage">上一页</button>
        <button class="header-btn primary" @click="nextPage">下一页</button>
      </div>
    </header>

    <div class="reader-layout">
      <aside v-if="showToc" class="reader-toc glass-card">
        <div class="toc-header">
          <h3>目录</h3>
          <span>{{ toc.length }} 项</span>
        </div>
        <div v-if="toc.length === 0" class="toc-empty">该图书未提供可用目录</div>
        <div v-else class="toc-tree">
          <button
            v-for="node in flatToc"
            :key="`${node.href}-${node.label}-${node.depth}`"
            class="toc-node"
            :style="{ paddingLeft: `${12 + node.depth * 16}px` }"
            @click="openChapter(node)"
          >
            {{ node.label }}
          </button>
        </div>
      </aside>

      <section class="reader-stage glass-card">
        <div ref="readerHostRef" class="reader-host"></div>
        <div v-if="loading" class="stage-state">正在加载 EPUB 图书...</div>
        <div v-else-if="error" class="stage-state stage-state--error">{{ error }}</div>
      </section>
    </div>

    <SelectionPopover
      :position="popoverPosition"
      :selection-type="selection.type"
      :selection-length="selection.text?.length || 0"
      @add-word="handleAddWord"
      @add-sentence="handleAddSentence"
    />

    <AnnotationForm
      v-if="showAnnotationForm"
      :type="annotationType"
      :selected-text="cachedSelectedText"
      :context-text="cachedContextText"
      :initial-meaning="annotationDraft.meaning"
      :initial-sentence-translation="annotationDraft.sentenceTranslation"
      :initial-structure-parsed="annotationDraft.structureParsed"
      :initial-structure-note="annotationDraft.structureNote"
      @save="handleSaveAnnotation"
      @cancel="handleCloseForm"
    />

    <QuickLookupPanel
      :visible="quickLookupVisible"
      :type="quickLookupType"
      :position="quickLookupAnchor"
      :selected-text="quickLookupSelectedText"
      :context-text="quickLookupContextText"
      :loading="quickLookupLoading"
      :deep-loading="quickLookupDeepLoading"
      :saving="quickLookupSaving"
      :error="quickLookupError"
      :deep-error="quickLookupDeepError"
      :word-pos="quickLookupWordPos"
      :meaning="quickLookupMeaning"
      :translation="quickLookupTranslation"
      :parsed-html="quickLookupParsedHtml"
      :structure-note="quickLookupStructureNote"
      @close="closeQuickLookup"
      @retry="retryQuickLookup"
      @deepen="requestSentenceDeepAnalysis"
      @edit="openQuickLookupEditor"
      @save="saveQuickLookup"
    />
  </section>
</template>

<style scoped>
.book-reader {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 20px 24px 28px;
}

.reader-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  flex-wrap: wrap;
}

.reader-title-block {
  display: flex;
  align-items: flex-start;
  gap: 12px;
}

.reader-title-wrap {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.reader-title {
  margin: 0;
  font-size: 1.4rem;
  color: var(--c-text);
}

.reader-subtitle {
  margin: 0;
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  font-size: 0.9rem;
  color: var(--c-text-lighter);
}

.reader-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.header-btn {
  padding: 8px 14px;
  border-radius: 10px;
  border: 1px solid var(--c-border);
  background: var(--c-bg-light);
  color: var(--c-text);
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
}

.header-btn:hover {
  border-color: var(--c-primary);
  color: var(--c-primary);
}

.header-btn.primary {
  background: linear-gradient(135deg, #007AFF, #409CFF);
  border-color: transparent;
  color: #fff;
}

.reader-layout {
  display: grid;
  grid-template-columns: 280px minmax(0, 1fr);
  gap: 16px;
  min-height: calc(100vh - 180px);
}

.glass-card {
  background: rgba(255, 255, 255, 0.72);
  border: 1px solid rgba(255, 255, 255, 0.55);
  box-shadow: 0 18px 40px rgba(15, 23, 42, 0.08);
  backdrop-filter: blur(14px);
  -webkit-backdrop-filter: blur(14px);
}

.reader-toc {
  border-radius: 18px;
  padding: 16px;
  overflow: auto;
}

.toc-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 12px;
}

.toc-header h3 {
  margin: 0;
  font-size: 1rem;
}

.toc-header span,
.toc-empty {
  color: var(--c-text-lighter);
  font-size: 0.85rem;
}

.toc-tree {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.toc-node {
  padding: 10px 12px;
  border-radius: 10px;
  border: 1px solid var(--c-border);
  background: var(--c-bg-light);
  color: var(--c-text);
  text-align: left;
  cursor: pointer;
  transition: all 0.15s ease;
}

.toc-node:hover {
  border-color: #bfdbfe;
  background: #eff6ff;
}

.reader-stage {
  position: relative;
  border-radius: 20px;
  padding: 16px;
  min-height: calc(100vh - 180px);
  overflow: hidden;
}

.reader-host {
  width: 100%;
  height: calc(100vh - 212px);
  border-radius: 14px;
  overflow: hidden;
  background: #fff;
}

.stage-state {
  position: absolute;
  inset: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--c-text-lighter);
  font-size: 0.95rem;
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.92);
  backdrop-filter: blur(2px);
  -webkit-backdrop-filter: blur(2px);
}

.stage-state--error {
  color: #dc2626;
}

@media (max-width: 960px) {
  .reader-layout {
    grid-template-columns: 1fr;
  }

  .reader-toc {
    max-height: 240px;
  }
}
</style>
