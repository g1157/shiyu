<script setup lang="ts">
import { defineAsyncComponent, ref, computed, onMounted, onUnmounted, onActivated, nextTick, watch } from 'vue'
import { useRouter } from 'vue-router'
import { getArticle, type ArticleItem } from '../services/api'
import { useSettingsStore } from '../stores/settingsStore'
import { marked } from 'marked'
import { useTextSelection } from '../composables/useTextSelection'
import { useAnnotation } from '../composables/useAnnotation'
import { useAnnotationInteraction } from '../composables/useAnnotationInteraction'
import { useTranslation } from '../composables/useTranslation'
import type { HighlightType } from '../composables/useRouteQuery'
import {
  buildInlineSentenceTranslationBlock,
  isInlineSentenceTranslationBlock,
  resolveInlineSentenceAnchor,
} from '../utils/inlineSentenceTranslation'
import { sanitizeRichHtml } from '../utils/sanitizeHtml'
import { clearTransientWordHighlights, highlightTransientWord } from '../utils/transientWordHighlight'

import SelectionPopover from './SelectionPopover.vue'
import AnnotationForm from './AnnotationForm.vue'
import AnnotationTooltip from './AnnotationTooltip.vue'
import QuickLookupPanel from './QuickLookupPanel.vue'
import Toast from './Toast.vue'
// ⚠️ CSS 导入顺序固定：annotation-highlight 必须最后加载
import '../styles/reader-typography.css'
import '../styles/article-reader.css'
import '../styles/annotation-highlight.css'

const ContentEditorModal = defineAsyncComponent(() => import('./ContentEditorModal.vue'))
const MindMapPanel = defineAsyncComponent(() => import('./MindMapPanel.vue'))
const ArticleWordList = defineAsyncComponent(() => import('./ArticleWordList.vue'))

marked.setOptions({ gfm: true, breaks: true })

const props = defineProps<{
  article: ArticleItem
  highlightId?: string | null
  highlightType?: HighlightType | null
  autoOpenEditor?: boolean
}>()

const emit = defineEmits<{
  close: []
  updated: [article: ArticleItem]
}>()

const router = useRouter()
const settingsStore = useSettingsStore()

type ReaderFontSize = 'small' | 'medium' | 'large'
type ReaderWidth = 'narrow' | 'medium' | 'wide'
type ReaderDensity = 'compact' | 'balanced' | 'relaxed'

const READER_FONT_SETTING_KEY = 'article_reader_font_size'
const READER_WIDTH_SETTING_KEY = 'article_reader_width'
const READER_DENSITY_SETTING_KEY = 'article_reader_density'

interface ArticleTocItem {
  id: string
  text: string
  level: number
}

// Reader state
const fontSize = ref<ReaderFontSize>('medium')
const readerWidth = ref<ReaderWidth>('medium')
const readerDensity = ref<ReaderDensity>('balanced')
const readingProgress = ref(0)
const showFloatingHeader = ref(false)
const showArticleToc = ref(true)
const activeTocId = ref<string | null>(null)

const readerViewRef = ref<HTMLElement | null>(null)
const readerBodyRef = ref<HTMLElement | null>(null)
const currentArticle = ref<ArticleItem | null>(null)
const currentArticleId = computed(() => currentArticle.value?.id || null)
const toastRef = ref<InstanceType<typeof Toast> | null>(null)
const showEditor = ref(false)
const showWordList = ref(false)
const showMindMap = ref(false)
const quickLookupPanelPosition = ref<{
  top: number
  left: number
  sourceTop?: number
  sourceBottom?: number
} | null>(null)

function openEditor() {
  showEditor.value = true
}

function handleEditorSaved(updated: ArticleItem) {
  currentArticle.value = updated
  showEditor.value = false
  emit('updated', updated)
}

// 词句面板：删除后刷新标注
async function handleWordListDeleted(_type: 'word' | 'sentence', _id: string) {
  // 重新加载标注数据并刷新高亮
  await loadAnnotations()
  clearExistingAnnotations()
  await nextTick()
  highlightAnnotatedContent()
}

// 词句面板：定位到文中高亮
function handleWordListLocate(type: 'word' | 'sentence', id: string) {
  const dataAttr = type === 'word' ? 'word-id' : 'sentence-id'
  const el = readerBodyRef.value?.querySelector(`[data-${dataAttr}="${id}"]`) as HTMLElement | null
  if (!el) return
  el.scrollIntoView({ behavior: 'smooth', block: 'center' })
  el.classList.add('flashing')
  setTimeout(() => el.classList.remove('flashing'), 3200)
}

function updateQuickLookupPanelPosition() {
  if (!quickLookupVisible.value || !quickLookupRange.value) {
    quickLookupPanelPosition.value = quickLookupAnchor.value
    return
  }

  const rect = quickLookupRange.value.getBoundingClientRect()
  if (rect.width === 0 && rect.height === 0) {
    quickLookupPanelPosition.value = quickLookupAnchor.value
    return
  }

  const selectionType = quickLookupType.value || 'word'
  let top = selectionType === 'sentence' ? rect.top : rect.top - 50
  if (selectionType !== 'sentence' && top < 60) {
    top = rect.bottom + 10
  }

  const left = selectionType === 'sentence'
    ? Math.max(32, Math.min(window.innerWidth - 16, rect.right))
    : Math.max(32, Math.min(window.innerWidth - 32, rect.left + rect.width / 2))
  quickLookupPanelPosition.value = {
    top,
    left,
    sourceTop: rect.top,
    sourceBottom: rect.bottom,
  }
}

// Composables
const {
  selection,
  popoverPosition,
  clearSelection,
  getContext,
} = useTextSelection(readerBodyRef)

const {
  annotationEnabled,
  loadData: loadAnnotations,
  highlightAnnotatedContent,
  clearExistingAnnotations,
  saveWord,
  saveSentence,
  deleteWord,
  deleteSentence,
  findWordById,
  findSentenceById,
} = useAnnotation(readerBodyRef, currentArticleId)

const {
  showAnnotationForm,
  annotationType,
  cachedSelectedText,
  cachedContextText,
  annotationDraft,
  tooltipState,
  quickLookupVisible,
  quickLookupType,
  quickLookupAnchor,
  quickLookupRange,
  quickLookupSelectedText,
  quickLookupContextText,
  quickLookupWordPos,
  quickLookupPhonetic,
  quickLookupUsedContext,
  quickLookupMeaning,
  quickLookupBaseMeaning,
  quickLookupOtherMeanings,
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
  toggleAnnotations,
  handleAnnotationClick,
  handleHighlightHover,
  handleHighlightLeave,
  handleTooltipClose,
  handleTooltipEnter,
  handleTooltipLeave,
  handleTooltipRemove,
  handlePageClick,
} = useAnnotationInteraction(
  router,
  toastRef,
  { selection, clearSelection, getContext, popoverPosition },
  { annotationEnabled, findWordById, findSentenceById, saveWord, saveSentence, deleteWord, deleteSentence },
)

import { resolveLocalImages, resolveLocalImagesInMarkdown } from '../utils/imageResolver'

const articleMarkdownContent = computed(() => currentArticle.value?.content || null)
const articleTitleRef = computed(() => currentArticle.value?.title || null)
const articleIdRef = computed(() => currentArticle.value?.id || null)

const {
  translationEnabled,
  translating,
  titleTranslation,
  titleStreaming,
  toggleTranslation,
  cleanup: cleanupTranslation,
} = useTranslation(readerBodyRef, articleMarkdownContent, articleTitleRef, articleIdRef)

function slugifyHeading(text: string, fallbackIndex: number, usedIds: Set<string>) {
  const base = text
    .toLowerCase()
    .trim()
    .replace(/<[^>]+>/g, '')
    .replace(/[^\p{L}\p{N}\s-]/gu, '')
    .replace(/\s+/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '') || `section-${fallbackIndex}`

  let candidate = base
  let suffix = 2
  while (usedIds.has(candidate)) {
    candidate = `${base}-${suffix}`
    suffix += 1
  }
  usedIds.add(candidate)
  return candidate
}

function buildRenderedArticlePayload(html: string): { html: string; toc: ArticleTocItem[] } {
  if (typeof DOMParser === 'undefined') {
    return { html, toc: [] }
  }

  const parser = new DOMParser()
  const doc = parser.parseFromString(html, 'text/html')
  const headings = Array.from(doc.body.querySelectorAll('h1, h2, h3, h4'))
  const toc: ArticleTocItem[] = []
  const usedIds = new Set<string>()

  headings.forEach((heading, index) => {
    const text = (heading.textContent || '').trim()
    if (!text) return
    const level = Number(heading.tagName.slice(1))
    const existingId = heading.id?.trim()
    const id = existingId && !usedIds.has(existingId)
      ? existingId
      : slugifyHeading(text, index + 1, usedIds)

    usedIds.add(id)
    heading.id = id
    toc.push({ id, text, level })
  })

  return {
    html: doc.body.innerHTML,
    toc,
  }
}

const renderedArticle = computed(() => {
  if (!currentArticle.value) {
    return {
      html: '',
      toc: [] as ArticleTocItem[],
    }
  }
  const content = currentArticle.value.content
  // Resolve image paths before markdown parsing (prevents backslash issues)
  const resolved = resolveLocalImagesInMarkdown(content)
  const html = sanitizeRichHtml(resolveLocalImages(marked.parse(resolved) as string))
  return buildRenderedArticlePayload(html)
})

const parsedContent = computed(() => renderedArticle.value.html)
const articleToc = computed(() => renderedArticle.value.toc)

const fontSizeMap = {
  small: '0.95rem',
  medium: '1.1rem',
  large: '1.25rem',
}

const fontSizeLabel = {
  small: '小',
  medium: '中',
  large: '大',
}

const readerWidthMap = {
  narrow: '620px',
  medium: '680px',
  wide: '820px',
}

const readerDensityLineHeightMap = {
  compact: '1.72',
  balanced: '1.9',
  relaxed: '2.08',
}

const readerDensityImageWidthMap = {
  compact: '68%',
  balanced: '75%',
  relaxed: '82%',
}

const readerViewStyle = computed(() => ({
  '--reader-content-width': readerWidthMap[readerWidth.value],
  '--reader-line-height': readerDensityLineHeightMap[readerDensity.value],
  '--reader-image-max-width': readerDensityImageWidthMap[readerDensity.value],
}))

const readerBodyStyle = computed(() => ({
  fontSize: fontSizeMap[fontSize.value],
}))

const estimatedReadTime = computed(() => {
  if (!currentArticle.value) return 0
  return Math.max(1, Math.ceil(currentArticle.value.word_count / 400))
})

// Highlight logic
let pendingHighlightId = ref<string | null>(null)
let pendingHighlightType = ref<HighlightType | null>(null)
let highlightRetryCount = 0
const highlightRetryLimit = 8
let highlightTimer: ReturnType<typeof setTimeout> | null = null
let inlineSentenceBlockEl: HTMLElement | null = null
let inlineSentenceBlockCleanup: (() => void) | null = null

function removeInlineSentenceTranslation() {
  inlineSentenceBlockCleanup?.()
  inlineSentenceBlockCleanup = null
  inlineSentenceBlockEl?.remove()
  inlineSentenceBlockEl = null
}

function expandSentenceTranslationInline() {
  if (quickLookupType.value !== 'sentence' || !quickLookupTranslation.value.trim()) {
    return
  }

  const anchor = resolveInlineSentenceAnchor(quickLookupRange.value)
  if (!anchor || !readerBodyRef.value?.contains(anchor)) {
    toastRef.value?.show('未找到原句位置，请重新选句')
    return
  }

  removeInlineSentenceTranslation()

  const block = buildInlineSentenceTranslationBlock(anchor.ownerDocument, {
    translation: quickLookupTranslation.value,
    parsedHtml: quickLookupParsedHtml.value,
    structureNote: quickLookupStructureNote.value,
  })
  const closeButton = block.querySelector('.inline-sentence-translation__close') as HTMLButtonElement | null
  const handleClose = (event: Event) => {
    event.preventDefault()
    removeInlineSentenceTranslation()
  }
  closeButton?.addEventListener('click', handleClose)
  inlineSentenceBlockCleanup = () => closeButton?.removeEventListener('click', handleClose)

  const next = anchor.nextElementSibling
  if (isInlineSentenceTranslationBlock(next)) {
    next.replaceWith(block)
  } else {
    anchor.insertAdjacentElement('afterend', block)
  }

  inlineSentenceBlockEl = block
  block.scrollIntoView({ behavior: 'smooth', block: 'nearest' })
  closeQuickLookup()
}

function triggerHighlight() {
  if (!pendingHighlightId.value || !pendingHighlightType.value) return
  const selector = pendingHighlightType.value === 'word'
    ? `[data-word-id="${pendingHighlightId.value}"]`
    : `[data-sentence-id="${pendingHighlightId.value}"]`

  const element = document.querySelector(selector) as HTMLElement | null
  if (!element) {
    highlightRetryCount += 1
    if (highlightRetryCount >= highlightRetryLimit) {
      pendingHighlightId.value = null
      pendingHighlightType.value = null
      return
    }
    setTimeout(() => triggerHighlight(), 250)
    return
  }

  // 先清除所有正在闪烁的元素，确保同一时间只有一个在闪
  document.querySelectorAll('.flashing').forEach(el => el.classList.remove('flashing'))
  if (highlightTimer) clearTimeout(highlightTimer)

  element.scrollIntoView({ behavior: 'smooth', block: 'center' })
  element.classList.add('flashing')

  highlightTimer = setTimeout(() => {
    element.classList.remove('flashing')
  }, 3200)

  pendingHighlightId.value = null
  pendingHighlightType.value = null
  highlightRetryCount = 0
}

function cycleFontSize() {
  const sizes: ReaderFontSize[] = ['small', 'medium', 'large']
  const idx = sizes.indexOf(fontSize.value)
  setFontSize(sizes[(idx + 1) % sizes.length])
}

function setFontSize(next: ReaderFontSize) {
  if (fontSize.value === next) return
  fontSize.value = next
  void persistReaderPreference(READER_FONT_SETTING_KEY, fontSize.value)
}

function adjustFontSize(step: 1 | -1) {
  const sizes: ReaderFontSize[] = ['small', 'medium', 'large']
  const idx = sizes.indexOf(fontSize.value)
  const nextIdx = Math.max(0, Math.min(sizes.length - 1, idx + step))
  if (nextIdx === idx) return
  setFontSize(sizes[nextIdx])
}

function isReaderFontSize(value: string | null): value is ReaderFontSize {
  return value === 'small' || value === 'medium' || value === 'large'
}

function isReaderWidth(value: string | null): value is ReaderWidth {
  return value === 'narrow' || value === 'medium' || value === 'wide'
}

function isReaderDensity(value: string | null): value is ReaderDensity {
  return value === 'compact' || value === 'balanced' || value === 'relaxed'
}

async function loadReaderPreferences() {
  try {
    await settingsStore.loadSettings()
    const savedFontSize = settingsStore.getSettingImmediate(READER_FONT_SETTING_KEY)
    const savedWidth = settingsStore.getSettingImmediate(READER_WIDTH_SETTING_KEY)
    const savedDensity = settingsStore.getSettingImmediate(READER_DENSITY_SETTING_KEY)

    if (isReaderFontSize(savedFontSize)) {
      fontSize.value = savedFontSize
    }

    if (isReaderWidth(savedWidth)) {
      readerWidth.value = savedWidth
    }

    if (isReaderDensity(savedDensity)) {
      readerDensity.value = savedDensity
    }
  } catch (error) {
    console.error('加载阅读器偏好失败:', error)
  }
}

async function persistReaderPreference(key: string, value: string) {
  try {
    await settingsStore.setSetting(key, value)
  } catch (error) {
    console.error(`保存阅读器偏好失败: ${key}`, error)
  }
}

function scrollToHeading(id: string) {
  const target = readerBodyRef.value?.querySelector(`#${CSS.escape(id)}`) as HTMLElement | null
  if (!target) return
  activeTocId.value = id
  target.scrollIntoView({ behavior: 'smooth', block: 'start' })
}

function updateActiveToc() {
  if (!readerBodyRef.value || articleToc.value.length === 0) {
    activeTocId.value = null
    return
  }

  const headingElements = articleToc.value
    .map((item) => readerBodyRef.value?.querySelector(`#${CSS.escape(item.id)}`) as HTMLElement | null)
    .filter((item): item is HTMLElement => Boolean(item))

  if (headingElements.length === 0) {
    activeTocId.value = null
    return
  }

  const current = headingElements
    .filter((item) => item.getBoundingClientRect().top <= 140)
  const active = current.length > 0 ? current[current.length - 1] : headingElements[0]

  activeTocId.value = active.id
}

function handleReaderWheel(event: WheelEvent) {
  if (!event.ctrlKey) return
  event.preventDefault()
  adjustFontSize(event.deltaY < 0 ? 1 : -1)
}

let scrollRafId: number | null = null
let scrollContainer: HTMLElement | null = null

function getScrollContainer(): HTMLElement {
  if (!scrollContainer) {
    scrollContainer = document.querySelector('.app-main') as HTMLElement
  }
  return scrollContainer || document.documentElement
}

function dismissSelectionOverlaysOnScroll() {
  if (!popoverPosition.value.visible && !quickLookupVisible.value) return
  closeQuickLookup()
  quickLookupPanelPosition.value = null
  popoverPosition.value.visible = false
  popoverPosition.value.anchor = 'center'
}

function handleScroll() {
  dismissSelectionOverlaysOnScroll()
  if (scrollRafId !== null) return
  scrollRafId = requestAnimationFrame(() => {
    scrollRafId = null
    const el = getScrollContainer()
    const scrollTop = el.scrollTop
    const scrollHeight = el.scrollHeight - el.clientHeight
    readingProgress.value = scrollHeight > 0 ? Math.min((scrollTop / scrollHeight) * 100, 100) : 0
    showFloatingHeader.value = scrollTop > 200
    updateActiveToc()
  })
}

function scrollToTop() {
  getScrollContainer().scrollTo({ top: 0, behavior: 'smooth' })
}

function closeReader() {
  removeInlineSentenceTranslation()
  clearSelection()
  clearExistingAnnotations()
  cleanupTranslation()
  readingProgress.value = 0
  currentArticle.value = null
  emit('close')
}

// Initialize reader content
async function initReader() {
  removeInlineSentenceTranslation()

  // Fetch full article content
  try {
    const fullArticle = await getArticle(props.article.id)
    currentArticle.value = fullArticle
  } catch (e) {
    console.error('获取文章详情失败:', e)
    currentArticle.value = props.article
  }

  readingProgress.value = 0
  showFloatingHeader.value = false
  pendingHighlightId.value = props.highlightId ?? null
  pendingHighlightType.value = props.highlightType ?? null
  highlightRetryCount = 0

  nextTick(async () => {
    await loadAnnotations()
    await nextTick()
    await nextTick()
    updateActiveToc()
    setTimeout(() => {
      highlightAnnotatedContent()
      setTimeout(() => triggerHighlight(), 100)
    }, 50)

    // OCR 导入时自动打开编辑器
    if (props.autoOpenEditor) {
      showEditor.value = true
    }
  })
}

onMounted(() => {
  void loadReaderPreferences()
  initReader()
  readerViewRef.value?.addEventListener('wheel', handleReaderWheel, { passive: false })
  getScrollContainer().addEventListener('scroll', handleScroll, { passive: true })
  document.addEventListener('mousedown', handlePageClick)
  window.addEventListener('resize', updateQuickLookupPanelPosition)
})

onUnmounted(() => {
  removeInlineSentenceTranslation()
  readerViewRef.value?.removeEventListener('wheel', handleReaderWheel)
  getScrollContainer().removeEventListener('scroll', handleScroll)
  document.removeEventListener('mousedown', handlePageClick)
  window.removeEventListener('resize', updateQuickLookupPanelPosition)
  if (highlightTimer) clearTimeout(highlightTimer)
  if (scrollRafId !== null) cancelAnimationFrame(scrollRafId)
  scrollContainer = null
})

onActivated(() => {
  void loadAnnotations().then(async () => {
    clearExistingAnnotations()
    await nextTick()
    highlightAnnotatedContent()
  })
})

function currentTransientHighlightWord() {
  if (quickLookupVisible.value && quickLookupType.value === 'word') {
    return quickLookupSelectedText.value
  }
  if (selection.value.type === 'word') {
    return selection.value.text
  }
  return ''
}

function syncTransientWordHighlights() {
  const word = currentTransientHighlightWord()
  if (word) {
    highlightTransientWord(readerBodyRef.value, word)
  } else {
    clearTransientWordHighlights(readerBodyRef.value)
  }
}

// Watch for article change (e.g., route navigation while reader is open)
watch(() => props.article.id, () => {
  initReader()
})

watch(
  () => [selection.value.text, selection.value.type, quickLookupVisible.value, quickLookupType.value, quickLookupSelectedText.value] as const,
  () => {
    if (!quickLookupVisible.value) {
      quickLookupPanelPosition.value = null
    }
    nextTick(() => {
      if (quickLookupVisible.value) updateQuickLookupPanelPosition()
      syncTransientWordHighlights()
    })
  },
)

// Watch for highlight change on same article (e.g., vocab/sentences → 同一文章的不同标注)
watch(() => props.highlightId, (newId) => {
  if (!newId) return
  pendingHighlightId.value = newId
  pendingHighlightType.value = props.highlightType ?? null
  highlightRetryCount = 0
  nextTick(() => triggerHighlight())
})
</script>
<template>
  <div ref="readerViewRef" class="reader-view" :style="readerViewStyle">
    <!-- 阅读进度条 -->
    <div class="reading-progress-bar">
      <div class="reading-progress-fill" :style="{ width: readingProgress + '%' }"></div>
    </div>

    <!-- 浮动顶部栏 -->
    <transition name="slide-down">
      <header v-if="showFloatingHeader" class="floating-header">
        <button class="floating-back-btn" @click="closeReader">
          <svg viewBox="0 0 24 24" width="18" height="18" stroke="currentColor" stroke-width="2" fill="none">
            <path d="M19 12H5"/>
            <polyline points="12 19 5 12 12 5"/>
          </svg>
        </button>
        <span class="floating-title">{{ currentArticle?.title }}</span>
        <div class="floating-actions">
          <!-- 翻译开关按钮 -->
          <button 
            class="tool-btn translation-toggle-btn" 
            :class="{ 'active': translationEnabled, 'loading': translating }"
            @click="toggleTranslation" 
            :title="translating ? '翻译中...' : (translationEnabled ? '隐藏翻译' : '显示翻译')"
            :disabled="translating"
          >
            <svg v-if="!translating" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M5 8l6 6"/><path d="M4 14l6-6 2-3"/><path d="M2 5h12"/><path d="M7 2h1"/>
              <path d="M22 22l-5-10-5 10"/><path d="M14 18h6"/>
            </svg>
            <svg v-else class="spin-icon" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 12a9 9 0 1 1-6.219-8.56"/>
            </svg>
            <span class="tool-label">{{ translating ? '译中' : '译' }}</span>
          </button>

          <!-- 标注开关按钮 -->
          <button 
            class="tool-btn annotation-toggle-btn" 
            :class="{ 'active': annotationEnabled }"
            @click="toggleAnnotations" 
            :title="annotationEnabled ? '关闭标注' : '开启标注'"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12 20h9"/><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"/>
            </svg>
            <span class="tool-label">标注</span>
          </button>
          
          <button class="tool-btn" @click="cycleFontSize" :title="'字号: ' + fontSizeLabel[fontSize]">
            <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M4 7V4h16v3"/>
              <path d="M9 20h6"/>
              <path d="M12 4v16"/>
            </svg>
            <span class="tool-label">{{ fontSizeLabel[fontSize] }}</span>
          </button>
          <button
            v-if="articleToc.length"
            class="tool-btn"
            @click="showArticleToc = !showArticleToc"
            :title="showArticleToc ? '隐藏目录' : '显示目录'"
          >
            <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M8 6h12"/>
              <path d="M8 12h12"/>
              <path d="M8 18h12"/>
              <circle cx="4" cy="6" r="1"/>
              <circle cx="4" cy="12" r="1"/>
              <circle cx="4" cy="18" r="1"/>
            </svg>
            <span class="tool-label">目录</span>
          </button>
          <span class="progress-text">{{ Math.round(readingProgress) }}%</span>
        </div>
      </header>
    </transition>

    <!-- 文章头部 -->
    <header v-if="currentArticle" class="reader-header">
      <div class="reader-title-area">
        <div class="title-decoration"></div>
        <h1>{{ currentArticle.title }}</h1>
        <div v-if="translationEnabled && titleTranslation" class="translation-block title-translation" :class="{ 'streaming': titleStreaming }">
          {{ titleTranslation }}
        </div>
        <div class="reader-meta">
          <span v-if="currentArticle.author" class="meta-chip author">
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/>
              <circle cx="12" cy="7" r="4"/>
            </svg>
            {{ currentArticle.author }}
          </span>
          <span v-if="currentArticle.category" class="meta-chip category">
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
            </svg>
            {{ currentArticle.category }}
          </span>
          <span class="meta-chip">
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
              <polyline points="14 2 14 8 20 8"/>
              <line x1="16" y1="13" x2="8" y2="13"/>
              <line x1="16" y1="17" x2="8" y2="17"/>
            </svg>
            {{ currentArticle.word_count.toLocaleString() }} 字
          </span>
          <span class="meta-chip">
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="10"/>
              <polyline points="12 6 12 12 16 14"/>
            </svg>
            约 {{ estimatedReadTime }} 分钟
          </span>
        </div>
      </div>
    </header>

    <div v-if="currentArticle" class="reader-toolbar">
      <button class="tool-btn reader-setting-btn" @click="cycleFontSize" :title="'字号: ' + fontSizeLabel[fontSize]">
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M4 7V4h16v3"/>
          <path d="M9 20h6"/>
          <path d="M12 4v16"/>
        </svg>
        <span class="tool-label">字号 {{ fontSizeLabel[fontSize] }}</span>
      </button>
      <button
        v-if="articleToc.length"
        class="tool-btn reader-setting-btn"
        @click="showArticleToc = !showArticleToc"
        :title="showArticleToc ? '隐藏目录' : '显示目录'"
      >
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M8 6h12"/>
          <path d="M8 12h12"/>
          <path d="M8 18h12"/>
          <circle cx="4" cy="6" r="1"/>
          <circle cx="4" cy="12" r="1"/>
          <circle cx="4" cy="18" r="1"/>
        </svg>
        <span class="tool-label">{{ showArticleToc ? '隐藏目录' : '显示目录' }}</span>
      </button>
    </div>

    <div
      class="reader-content-shell"
      :class="{ 'reader-content-shell--toc-hidden': !showArticleToc || articleToc.length === 0 }"
    >
      <aside v-if="articleToc.length && showArticleToc" class="reader-toc-sidebar">
        <div class="reader-toc-header">
          <div>
            <h3>目录</h3>
            <p>{{ articleToc.length }} 节</p>
          </div>
          <button class="tool-btn reader-toc-toggle-btn" @click="showArticleToc = false">
            收起
          </button>
        </div>
        <nav class="reader-toc-nav">
          <button
            v-for="item in articleToc"
            :key="item.id"
            class="reader-toc-link"
            :class="[
              `reader-toc-link--level-${Math.min(item.level, 4)}`,
              { 'reader-toc-link--active': activeTocId === item.id },
            ]"
            @click="scrollToHeading(item.id)"
          >
            {{ item.text }}
          </button>
        </nav>
      </aside>

      <!-- 文章正文 -->
      <article
        ref="readerBodyRef"
        class="reader-body"
        :class="{ 'annotations-hidden': !annotationEnabled }"
        :style="readerBodyStyle"
        v-html="parsedContent"
        @click="handleAnnotationClick"
        @mouseover="handleHighlightHover"
        @mouseout="handleHighlightLeave"
      ></article>
    </div>



    <!-- 右下角浮动操作栏 -->
    <div v-if="currentArticle" class="fab-column">
        <!-- 返回列表 -->
        <button class="fab-btn" @click="closeReader" data-tooltip="返回列表">
          <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M19 12H5"/>
            <polyline points="12 19 5 12 12 5"/>
          </svg>
        </button>

        <!-- 编辑排版 -->
        <button class="fab-btn" @click="openEditor" data-tooltip="编辑排版">
          <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/>
            <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
          </svg>
        </button>

        <!-- 词句面板 -->
        <button class="fab-btn" :class="{ 'fab-active': showWordList }" @click="showWordList = true" data-tooltip="词句面板">
          <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/>
            <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"/>
          </svg>
        </button>

        <!-- 思维导图 -->
        <button class="fab-btn" :class="{ 'fab-active': showMindMap }" @click="showMindMap = true" data-tooltip="思维导图">
          <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="5" r="2.5"/><circle cx="5" cy="19" r="2.5"/><circle cx="19" cy="19" r="2.5"/>
            <path d="M12 7.5V12m0 0l-5.5 4.5M12 12l5.5 4.5"/>
          </svg>
        </button>

        <!-- 双语翻译 -->
        <button 
          class="fab-btn" 
          :class="{ 'fab-active': translationEnabled, 'fab-loading': translating }"
          @click="toggleTranslation"
          :data-tooltip="translating ? '翻译中...' : (translationEnabled ? '隐藏翻译' : '双语翻译')"
          :disabled="translating"
        >
          <svg v-if="!translating" xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M5 8l6 6"/><path d="M4 14l6-6 2-3"/><path d="M2 5h12"/><path d="M7 2h1"/>
            <path d="M22 22l-5-10-5 10"/><path d="M14 18h6"/>
          </svg>
          <svg v-else class="spin-icon" xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 12a9 9 0 1 1-6.219-8.56"/>
          </svg>
        </button>

        <!-- 回到顶部 -->
        <button 
          class="fab-btn fab-primary fab-scroll-top" 
          :class="{ 'fab-visible': showFloatingHeader }"
          @click="scrollToTop" 
          data-tooltip="回到顶部"
        >
          <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="18 15 12 9 6 15"/>
          </svg>
        </button>
    </div>

    <!-- 标注交互层 -->
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

    <AnnotationTooltip
      v-if="tooltipState.visible"
      :content="tooltipState.content"
      :type="tooltipState.type"
      :position="tooltipState.position"
      @close="handleTooltipClose"
      @hover-enter="handleTooltipEnter"
      @hover-leave="handleTooltipLeave"
      @remove="handleTooltipRemove"
    />

    <QuickLookupPanel
      :visible="quickLookupVisible"
      :type="quickLookupType"
      :position="quickLookupPanelPosition || quickLookupAnchor"
      :content-element="readerBodyRef"
      :selected-text="quickLookupSelectedText"
      :context-text="quickLookupContextText"
      :loading="quickLookupLoading"
      :deep-loading="quickLookupDeepLoading"
      :saving="quickLookupSaving"
      :error="quickLookupError"
      :deep-error="quickLookupDeepError"
      :word-pos="quickLookupWordPos"
      :phonetic="quickLookupPhonetic"
      :used-context="quickLookupUsedContext"
      :meaning="quickLookupMeaning"
      :base-meaning="quickLookupBaseMeaning"
      :other-meanings="quickLookupOtherMeanings"
      :translation="quickLookupTranslation"
      :parsed-html="quickLookupParsedHtml"
      :structure-note="quickLookupStructureNote"
      @close="closeQuickLookup"
      @retry="retryQuickLookup"
      @deepen="requestSentenceDeepAnalysis"
      @inline="expandSentenceTranslationInline"
      @edit="openQuickLookupEditor"
      @save="saveQuickLookup"
    />



    <!-- 内容编辑器 -->
    <ContentEditorModal
      v-if="currentArticle"
      :visible="showEditor"
      :article="currentArticle"
      @close="showEditor = false"
      @saved="handleEditorSaved"
    />

    <!-- 词句面板 -->
    <ArticleWordList
      v-if="currentArticle"
      :visible="showWordList"
      :article-id="currentArticle.id"
      :article-title="currentArticle.title"
      @close="showWordList = false"
      @deleted="handleWordListDeleted"
      @locate="handleWordListLocate"
    />

    <!-- 思维导图面板 -->
    <MindMapPanel
      v-if="currentArticle"
      :visible="showMindMap"
      :article-id="currentArticle.id"
      :article-content="currentArticle.content"
      :article-title="currentArticle.title"
      @close="showMindMap = false"
    />

    <Toast ref="toastRef" />
  </div>
</template>
