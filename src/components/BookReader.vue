<script setup lang="ts">
import { computed, nextTick, onActivated, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { convertFileSrc } from '@tauri-apps/api/core'
import ePub from 'epubjs'
import {
  addSentence,
  addVocabulary,
  deleteSentence as deleteSentenceApi,
  deleteVocabulary as deleteVocabularyApi,
  getEbook,
  getSentencesByEbook,
  getVocabularyByEbook,
  updateEbookProgress,
  type EbookItem,
  type SentenceItem,
  type VocabularyItem,
} from '../services/api'
import AnnotationForm from './AnnotationForm.vue'
import AnnotationTooltip from './AnnotationTooltip.vue'
import QuickLookupPanel from './QuickLookupPanel.vue'
import SelectionPopover from './SelectionPopover.vue'
import { useAnnotationInteraction } from '../composables/useAnnotationInteraction'
import { useGlobalToast } from '../composables/useGlobalToast'
import type { PopoverPosition, SelectionState } from '../composables/useTextSelection'
import type { HighlightType } from '../composables/useRouteQuery'
import { preCacheText } from '../services/ttsCache'
import { useAppStore } from '../stores/appStore'
import { useSettingsStore } from '../stores/settingsStore'
import {
  buildInlineSentenceTranslationBlock,
  isInlineSentenceTranslationBlock,
  resolveInlineSentenceAnchor,
} from '../utils/inlineSentenceTranslation'

interface TocNode {
  uid: string
  id?: string
  label: string
  href: string
  subitems: TocNode[]
}

interface VisibleTocNode extends TocNode {
  depth: number
  expanded: boolean
  hasChildren: boolean
}

interface BookAnnotationEntry {
  cfi: string
  type: 'highlight' | 'underline'
  annotationId: string
  annotationType: HighlightType
  annotation: {
    mark?: {
      element?: Element | null
      getBoundingClientRect?: () => DOMRect
      getClientRects?: () => DOMRect[]
    } | null
  } | null
  mark: {
    element?: Element | null
    getBoundingClientRect?: () => DOMRect
    getClientRects?: () => DOMRect[]
  } | null
  idleAttrs: Record<string, string>
  hoverAttrs: Record<string, string>
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
const settingsStore = useSettingsStore()
const TOC_PANEL_STORAGE_KEY = 'shiyu:book-reader:toc-visible:v2'
type ReaderFontSize = 'small' | 'medium' | 'large'
type ReaderWidth = 'narrow' | 'medium' | 'wide'
type ReaderDensity = 'compact' | 'balanced' | 'relaxed'

// Shared with ArticleReader so article/book reading stays visually consistent.
const READER_FONT_SETTING_KEY = 'article_reader_font_size'
const READER_WIDTH_SETTING_KEY = 'article_reader_width'
const READER_DENSITY_SETTING_KEY = 'article_reader_density'

const bookReaderFontSizeMap = {
  small: '1.02rem',
  medium: '1.13rem',
  large: '1.25rem',
}

const bookReaderFontSizeLabel = {
  small: '小',
  medium: '中',
  large: '大',
}

const bookReaderWidthMap = {
  narrow: '680px',
  medium: '760px',
  wide: '860px',
}

const bookReaderWidthLabel = {
  narrow: '窄栏',
  medium: '标准',
  wide: '宽栏',
}

const bookReaderDensityLineHeightMap = {
  compact: '1.82',
  balanced: '1.98',
  relaxed: '2.12',
}

const bookReaderDensityImageWidthMap = {
  compact: '68%',
  balanced: '75%',
  relaxed: '82%',
}

const bookReaderDensityLabel = {
  compact: '紧凑',
  balanced: '均衡',
  relaxed: '舒展',
}

const BOOK_TOOLBAR_SCROLL_THRESHOLD = 96

const readerShellRef = ref<HTMLElement | null>(null)
const readerHostRef = ref<HTMLElement | null>(null)
const currentEbook = ref<EbookItem | null>(null)
const loading = ref(true)
const error = ref<string | null>(null)
const toc = ref<TocNode[]>([])
const expandedTocKeys = ref<Record<string, boolean>>({})
const showToc = ref(readStoredTocVisibility())
const currentChapter = ref('')
const annotationEnabled = ref(true)
const fontSize = ref<ReaderFontSize>('medium')
const readerWidth = ref<ReaderWidth>('medium')
const readerDensity = ref<ReaderDensity>('balanced')
const showTopToolbar = ref(false)
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
const quickLookupPanelPosition = ref<{
  top: number
  left: number
  sourceTop?: number
  sourceBottom?: number
} | null>(null)
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
  handleTooltipClose,
  handleTooltipEnter,
  handleTooltipLeave,
  handleTooltipRemove,
  showTooltipForAnnotation,
} = useAnnotationInteraction(
  router,
  toastRef,
  { selection, clearSelection, getContext, popoverPosition },
  { annotationEnabled, findWordById, findSentenceById, saveWord, saveSentence, deleteWord, deleteSentence },
)

let bookInstance: any = null
let rendition: any = null
let progressTimer: ReturnType<typeof setTimeout> | null = null
let pendingProgress = 0
let pendingCfi: string | undefined
let contentCleanupFns: Array<() => void> = []
let renderedAnnotations: BookAnnotationEntry[] = []
let hoveredAnnotationKey: string | null = null
const activeHighlightId = ref<string | null>(null)
const activeHighlightType = ref<HighlightType | null>(null)
let highlightResetTimer: ReturnType<typeof setTimeout> | null = null
let inlineSentenceBlockEl: HTMLElement | null = null
let inlineSentenceBlockCleanup: (() => void) | null = null
let shellScrollContainer: HTMLElement | null = null
let locationsReady = false
let locationsGenerationPromise: Promise<void> | null = null

const progressPercent = computed(() =>
  Math.round(((currentEbook.value?.progress || 0) * 1000)) / 10
)
const currentTheme = computed<'light' | 'dark'>(() => (settingsStore.theme === 'dark' ? 'dark' : 'light'))
const readerShellStyle = computed(() => ({
  '--book-reader-host-width': bookReaderWidthMap[readerWidth.value],
}))
const readerToolbarTitle = computed(() => currentEbook.value?.title || props.ebook.title)
const readerToolbarSubtitle = computed(() => currentChapter.value || '正在加载章节')

const BOOK_READER_THEME_NAME = 'app-book-theme'
const registeredReaderThemes = new Set<string>()

function getBookReaderPalette(mode: 'light' | 'dark') {
  if (mode === 'dark') {
    return {
      text: '#DED7CC',
      textLighter: '#AEA79A',
      bgLight: '#171411',
      bgLighter: '#1F1B18',
      border: '#3A332D',
      borderLight: '#2A241F',
      primary: '#93CFF1',
      primaryDark: '#B4E1FA',
      codeText: '#FB7185',
      preBg: '#12100E',
      preText: '#E1D9CF',
      quoteBg: 'linear-gradient(135deg, rgba(196, 181, 159, 0.10), rgba(148, 163, 184, 0.06))',
      tableHead: 'linear-gradient(135deg, rgba(255, 255, 255, 0.035), rgba(255, 255, 255, 0.015))',
      tableAlt: 'rgba(255, 255, 255, 0.018)',
      markStart: '#594618',
      markEnd: '#886314',
      predicate: '#BAE6FD',
      nonfinite: '#C084FC',
      connector: '#F87171',
      structure: '#34D399',
      symbol: '#F59E0B',
      inlineBorder: 'rgba(180, 204, 220, 0.22)',
      inlineBgStart: 'rgba(36, 31, 27, 0.96)',
      inlineBgEnd: 'rgba(24, 21, 18, 0.98)',
      inlineShadow: '0 8px 24px rgba(2, 6, 23, 0.32)',
      badgeBg: 'rgba(147, 207, 241, 0.16)',
      badgeText: '#D8EEF9',
      closeText: '#AAA18F',
      translationText: '#E4DCCF',
      parsedBg: 'rgba(26, 22, 19, 0.84)',
      parsedBorder: 'rgba(111, 101, 90, 0.48)',
      parsedText: '#E5DDD0',
      note: '#C084FC',
      imageShadow: '0 4px 20px rgba(0, 0, 0, 0.28)',
      preShadow: '0 4px 16px rgba(0, 0, 0, 0.24)',
    }
  }

  return {
    text: '#1D1D1F',
    textLighter: '#86868B',
    bgLight: '#FFFFFF',
    bgLighter: '#F5F5F7',
    border: '#E8E8ED',
    borderLight: '#F1F1F5',
    primary: '#007AFF',
    primaryDark: '#0066D6',
    codeText: '#E11D48',
    preBg: '#1D1D1F',
    preText: '#E8E8ED',
    quoteBg: 'linear-gradient(135deg, #F5F5F7, #F5F5F7)',
    tableHead: 'linear-gradient(135deg, #F5F5F7, #F1F1F5)',
    tableAlt: '#F5F5F7',
    markStart: '#FEF3C7',
    markEnd: '#FDE68A',
    predicate: '#2563EB',
    nonfinite: '#7C3AED',
    connector: '#DC2626',
    structure: '#059669',
    symbol: '#E07B39',
    inlineBorder: 'rgba(191, 219, 254, 0.88)',
    inlineBgStart: 'rgba(239, 246, 255, 0.96)',
    inlineBgEnd: 'rgba(248, 250, 252, 0.98)',
    inlineShadow: '0 8px 24px rgba(37, 99, 235, 0.08)',
    badgeBg: 'rgba(37, 99, 235, 0.12)',
    badgeText: '#2563EB',
    closeText: '#64748B',
    translationText: '#1E293B',
    parsedBg: 'rgba(255, 255, 255, 0.72)',
    parsedBorder: 'rgba(226, 232, 240, 0.9)',
    parsedText: '#0F172A',
    note: '#7C3AED',
    imageShadow: '0 4px 20px rgba(0, 0, 0, 0.08)',
    preShadow: '0 4px 16px rgba(0, 0, 0, 0.12)',
  }
}

function getBookReaderThemeName(mode: 'light' | 'dark') {
  return `${BOOK_READER_THEME_NAME}-${mode}-${fontSize.value}-${readerDensity.value}`
}

function buildBookReaderInlineOverrideCss(mode: 'light' | 'dark') {
  const palette = getBookReaderPalette(mode)
  const linkColor = palette.primaryDark

  return String.raw`
html, body {
  background: ${palette.bgLight} !important;
}

body,
body p,
body li,
body dd,
body dt,
body div,
body span,
body section,
body article,
body main,
body [style*="color"],
body font[color] {
  color: ${palette.text} !important;
  -webkit-text-fill-color: ${palette.text};
}

body h1,
body h2,
body h3,
body h4,
body h5,
body h6,
body strong,
body b {
  color: ${palette.text} !important;
  -webkit-text-fill-color: ${palette.text};
}

body figcaption,
body small,
body td,
body th,
body blockquote,
body em,
body del {
  color: ${palette.textLighter} !important;
  -webkit-text-fill-color: ${palette.textLighter};
}

body a,
body a:link,
body a:visited,
body a:hover,
body a:active,
body a *,
body a font[color],
body a [style*="color"] {
  color: ${linkColor} !important;
  -webkit-text-fill-color: ${linkColor};
}
`
}

function buildBookReaderThemeCss(mode: 'light' | 'dark') {
  const palette = getBookReaderPalette(mode)
  const bodyFontSize = bookReaderFontSizeMap[fontSize.value]
  const bodyLineHeight = bookReaderDensityLineHeightMap[readerDensity.value]
  const imageMaxWidth = bookReaderDensityImageWidthMap[readerDensity.value]

  return String.raw`
:root {
  --c-text: ${palette.text};
  --c-text-lighter: ${palette.textLighter};
  --c-bg-light: ${palette.bgLight};
  --c-bg-lighter: ${palette.bgLighter};
  --c-border: ${palette.border};
  --c-border-light: ${palette.borderLight};
  --c-primary: ${palette.primary};
  --c-primary-dark: ${palette.primaryDark};
  --font-serif: 'Georgia', 'Times New Roman', serif;
  --font-mono: 'Cascadia Code', 'Fira Code', 'JetBrains Mono', 'Consolas', monospace;
}

html {
  background: var(--c-bg-light) !important;
}

body {
  color: var(--c-text) !important;
  background: var(--c-bg-light) !important;
  font-family: var(--font-serif) !important;
  font-size: ${bodyFontSize} !important;
  line-height: ${bodyLineHeight} !important;
  font-weight: 400 !important;
  letter-spacing: 0.006em;
  word-wrap: break-word;
  overflow-wrap: break-word;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
}

body div,
body span,
body section,
body article,
body main,
body p,
body li,
body dd,
body dt {
  color: var(--c-text) !important;
}

body p {
  margin: 1rem 0 1.15rem !important;
  color: inherit;
  text-align: justify;
}

body h1,
body h2,
body h3,
body h4,
body h5,
body h6 {
  color: var(--c-text) !important;
}

body h1 {
  margin: 2.5rem 0 1rem !important;
  padding-bottom: 0.5rem;
  border-bottom: 2px solid var(--c-border-light);
  font-size: 1.8em !important;
  font-weight: 800 !important;
  line-height: 1.3 !important;
  letter-spacing: -0.02em;
}

body h2 {
  margin: 2.2rem 0 0.8rem !important;
  padding-left: 14px;
  border-left: 4px solid var(--c-primary);
  font-size: 1.5em !important;
  font-weight: 700 !important;
  line-height: 1.35 !important;
}

body h3 {
  margin: 1.8rem 0 0.6rem !important;
  font-size: 1.25em !important;
  font-weight: 700 !important;
  line-height: 1.4 !important;
}

body h4 {
  margin: 1.5rem 0 0.5rem !important;
  font-size: 1.1em !important;
  font-weight: 600 !important;
}

body h5,
body h6 {
  margin: 1.2rem 0 0.4rem !important;
  color: var(--c-text-lighter) !important;
  font-size: 1em !important;
  font-weight: 600 !important;
}

body img {
  display: block !important;
  max-width: min(100%, ${imageMaxWidth}) !important;
  max-height: 500px !important;
  height: auto !important;
  margin: 1.5rem auto !important;
  border-radius: 10px;
  object-fit: contain;
  box-shadow: ${palette.imageShadow};
}

body figure {
  margin: 1.5rem 0 !important;
  text-align: center;
}

body figcaption {
  margin-top: 0.6rem;
  color: var(--c-text-lighter) !important;
  font-size: 0.85em !important;
  font-style: italic;
}

body blockquote {
  margin: 1.5rem 0 !important;
  padding: 1rem 1.5rem !important;
  border-left: 4px solid var(--c-primary);
  border-radius: 0 12px 12px 0;
  background: ${palette.quoteBg} !important;
  color: var(--c-text-lighter) !important;
  font-style: italic;
}

body blockquote p {
  margin: 0.5rem 0 !important;
}

body ul,
body ol {
  margin: 1rem 0 !important;
  padding-left: 1.8rem !important;
}

body ul {
  list-style-type: disc !important;
}

body ol {
  list-style-type: decimal !important;
}

body li {
  margin: 0.4rem 0 !important;
  color: inherit;
  line-height: 1.8 !important;
}

body li > ul,
body li > ol {
  margin: 0.3rem 0 !important;
}

body code {
  padding: 2px 7px;
  border-radius: 5px;
  background: linear-gradient(135deg, var(--c-border-light), var(--c-border)) !important;
  color: ${palette.codeText} !important;
  font-family: var(--font-mono) !important;
  font-size: 0.88em !important;
  word-break: break-word;
}

body pre {
  margin: 1.5rem 0 !important;
  padding: 1.2rem 1.5rem !important;
  border: 1px solid var(--c-border);
  border-radius: 12px;
  background: ${palette.preBg} !important;
  color: ${palette.preText} !important;
  overflow-x: auto;
  font-size: 0.88em !important;
  line-height: 1.6 !important;
  box-shadow: ${palette.preShadow};
}

body pre code {
  padding: 0;
  border-radius: 0;
  background: transparent !important;
  color: inherit !important;
  font-size: 1em !important;
}

body table {
  width: 100% !important;
  margin: 1.5rem 0 !important;
  border: 1px solid var(--c-border);
  border-collapse: collapse;
  border-radius: 10px;
  overflow: hidden;
  font-size: 0.92em !important;
}

body thead {
  background: ${palette.tableHead} !important;
}

body th {
  padding: 10px 14px !important;
  border-bottom: 2px solid var(--c-border);
  color: var(--c-text) !important;
  text-align: left;
  font-weight: 700 !important;
}

body td {
  padding: 10px 14px !important;
  border-bottom: 1px solid var(--c-border-light);
  color: var(--c-text-lighter) !important;
}

body tbody tr:nth-child(even) {
  background: ${palette.tableAlt} !important;
}

body hr {
  height: 2px;
  max-width: 120px;
  margin: 2.5rem auto !important;
  border: none;
  background: linear-gradient(90deg, transparent, var(--c-border), transparent) !important;
}

body a {
  color: var(--c-primary-dark) !important;
  border-bottom: none !important;
  text-decoration: underline !important;
  text-decoration-color: currentColor !important;
  text-decoration-thickness: 2px;
  text-underline-offset: 0.18em;
  font-weight: 700;
}

body a:hover {
  color: #D9F0FF !important;
  text-decoration-color: currentColor !important;
}

body a *,
body a:link *,
body a:visited *,
body a:hover *,
body a:active * {
  color: inherit !important;
}

body strong {
  color: var(--c-text) !important;
  font-weight: 700 !important;
}

body em {
  color: var(--c-text-lighter) !important;
  font-style: italic;
}

body figcaption,
body td,
body del {
  color: var(--c-text-lighter) !important;
}

body mark {
  padding: 1px 4px;
  border-radius: 3px;
  background: linear-gradient(135deg, ${palette.markStart}, ${palette.markEnd}) !important;
  color: inherit !important;
}

body del {
  color: var(--c-text-lighter) !important;
}

body sup,
body sub {
  font-size: 0.75em !important;
}

.ps-predicate { color: ${palette.predicate}; font-weight: 700; }

.ps-nonfinite {
  color: ${palette.nonfinite};
  text-decoration: underline;
  text-decoration-style: wavy;
  text-underline-offset: 3px;
}

.ps-connector { color: ${palette.connector}; font-weight: 600; }
.ps-italic { font-style: italic; }
.ps-main { font-weight: 700; }
.ps-structure { color: ${palette.structure}; font-weight: 600; }
.ps-symbol { color: ${palette.symbol}; font-weight: 800; font-family: var(--font-mono); }

.parsed-html-content {
  line-height: 2.1;
  font-family: var(--font-serif);
}

.inline-sentence-translation {
  margin: 0.85rem 0 1.15rem !important;
  padding: 12px 14px 14px !important;
  border: 1px solid ${palette.inlineBorder};
  border-radius: 14px;
  background: linear-gradient(180deg, ${palette.inlineBgStart}, ${palette.inlineBgEnd}) !important;
  box-shadow: ${palette.inlineShadow};
}

.inline-sentence-translation__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 8px;
}

.inline-sentence-translation__badge {
  display: inline-flex;
  align-items: center;
  padding: 3px 10px;
  border-radius: 999px;
  background: ${palette.badgeBg};
  color: ${palette.badgeText} !important;
  font-size: 12px !important;
  font-weight: 700 !important;
  letter-spacing: 0.01em;
}

.inline-sentence-translation__close {
  border: none;
  background: transparent;
  color: ${palette.closeText} !important;
  font-size: 12px !important;
  font-weight: 600 !important;
  cursor: pointer;
}

.inline-sentence-translation__translation {
  font-size: 0.98rem !important;
  line-height: 1.9 !important;
  color: ${palette.translationText} !important;
}

.inline-sentence-translation__parsed {
  margin-top: 10px;
  padding: 10px 12px;
  border-radius: 12px;
  background: ${palette.parsedBg} !important;
  border: 1px solid ${palette.parsedBorder};
  font-size: 0.95rem !important;
  color: ${palette.parsedText} !important;
}

.inline-sentence-translation__note {
  margin-top: 8px;
  font-size: 0.84rem !important;
  line-height: 1.65 !important;
  color: ${palette.note} !important;
  font-style: italic;
}
`
}

function applyInlineThemeOverride(contents: any) {
  const mode = currentTheme.value
  const palette = getBookReaderPalette(mode)
  const doc = contents?.document as Document | undefined
  if (!doc) return

  doc.documentElement?.style?.setProperty('background', palette.bgLight, 'important')
  doc.body?.style?.setProperty('background', palette.bgLight, 'important')
  doc.body?.style?.setProperty('color', palette.text, 'important')

  let styleEl = doc.getElementById('book-theme-inline-override') as HTMLStyleElement | null
  if (!styleEl) {
    styleEl = doc.createElement('style')
    styleEl.id = 'book-theme-inline-override'
    doc.head?.appendChild(styleEl)
  }
  styleEl.textContent = buildBookReaderInlineOverrideCss(mode)
}

const totalTocCount = computed(() => flattenToc(toc.value).length)

const visibleToc = computed<VisibleTocNode[]>(() => {
  const walk = (nodes: TocNode[], depth = 0): VisibleTocNode[] =>
    nodes.flatMap((node) => {
      const expanded = isTocExpanded(node.uid)
      const currentNode: VisibleTocNode = {
        ...node,
        depth,
        expanded,
        hasChildren: node.subitems.length > 0,
      }

      return [
        currentNode,
        ...(expanded ? walk(node.subitems, depth + 1) : []),
      ]
    })

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
  if (!anchor) {
    toast.error('未找到原句位置，请重新选句')
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

async function deleteWord(id: string) {
  await deleteVocabularyApi(id)
  vocabulary.value = vocabulary.value.filter((item) => item.id !== id)
  appStore.removeVocabularyItem(id)
  clearActiveHighlight()
  syncBookAnnotations()
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

async function deleteSentence(id: string) {
  await deleteSentenceApi(id)
  sentences.value = sentences.value.filter((item) => item.id !== id)
  appStore.removeSentenceItem(id)
  clearActiveHighlight()
  syncBookAnnotations()
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

function getShellScrollContainer(): HTMLElement {
  if (!shellScrollContainer) {
    shellScrollContainer = document.querySelector('.app-main') as HTMLElement
  }
  return shellScrollContainer || document.documentElement
}

function readStoredTocVisibility(): boolean {
  try {
    const raw = localStorage.getItem(TOC_PANEL_STORAGE_KEY)
    if (raw == null) return true
    return raw === '1'
  } catch {
    return true
  }
}

function persistTocVisibility() {
  try {
    localStorage.setItem(TOC_PANEL_STORAGE_KEY, showToc.value ? '1' : '0')
  } catch {}
}

function getTocExpandedStorageKey(bookId: string) {
  return `shiyu:book-reader:toc-expanded:${bookId}`
}

function loadExpandedTocState(bookId: string) {
  try {
    const raw = localStorage.getItem(getTocExpandedStorageKey(bookId))
    if (!raw) return {}
    const parsed = JSON.parse(raw)
    return parsed && typeof parsed === 'object' ? parsed as Record<string, boolean> : {}
  } catch {
    return {}
  }
}

function persistExpandedTocState() {
  if (!currentEbook.value) return
  try {
    localStorage.setItem(
      getTocExpandedStorageKey(currentEbook.value.id),
      JSON.stringify(expandedTocKeys.value),
    )
  } catch {}
}

function isTocExpanded(uid: string) {
  return expandedTocKeys.value[uid] === true
}

function toggleTocNode(uid: string) {
  expandedTocKeys.value = {
    ...expandedTocKeys.value,
    [uid]: !isTocExpanded(uid),
  }
  persistExpandedTocState()
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
  const right = frameRect ? frameRect.left + rect.right : rect.right
  const selectionType = detectSelectionType(range.toString())

  const sentenceGap = 6
  const estimatedWidth = selectionType === 'sentence'
    ? (range.toString().trim().length <= 30 ? 86 : 48)
    : 112
  const hasRightRoom = right + sentenceGap + estimatedWidth <= window.innerWidth - 12

  let topPosition = selectionType === 'sentence' ? bottom + sentenceGap : top - 50
  if (topPosition > window.innerHeight - 48) {
    topPosition = selectionType === 'sentence' ? top - 42 : top - 46
  }
  if (topPosition < 60) {
    topPosition = bottom + sentenceGap
  }

  const rawLeft = selectionType === 'sentence'
    ? (hasRightRoom ? right + sentenceGap : right - 4)
    : left + rect.width / 2
  const clampedLeft = selectionType === 'sentence'
    ? Math.max(12, Math.min(window.innerWidth - estimatedWidth - 12, rawLeft))
    : Math.max(56, Math.min(window.innerWidth - 16, rawLeft))

  popoverPosition.value = {
    top: topPosition,
    left: clampedLeft,
    anchor: selectionType === 'sentence' ? (hasRightRoom ? 'start' : 'tail') : 'center',
    visible: true,
  }
}

function updateQuickLookupPanelPosition() {
  if (!quickLookupVisible.value || !quickLookupRange.value) {
    quickLookupPanelPosition.value = quickLookupAnchor.value
    return
  }

  const rect = quickLookupRange.value.getBoundingClientRect()
  const ownerDocument =
    quickLookupRange.value.startContainer?.ownerDocument
    || quickLookupRange.value.commonAncestorContainer?.ownerDocument
  const frameElement = ownerDocument?.defaultView?.frameElement as HTMLElement | null
  const frameRect = frameElement?.getBoundingClientRect()
  const top = frameRect ? frameRect.top + rect.top : rect.top
  const bottom = frameRect ? frameRect.top + rect.bottom : rect.bottom
  const left = frameRect ? frameRect.left + rect.left : rect.left
  const right = frameRect ? frameRect.left + rect.right : rect.right
  const selectionType = quickLookupType.value || detectSelectionType(quickLookupRange.value.toString())

  if ((rect.width === 0 && rect.height === 0) || !Number.isFinite(top) || !Number.isFinite(left)) {
    quickLookupPanelPosition.value = quickLookupAnchor.value
    return
  }

  let topPosition = selectionType === 'sentence' ? top : top - 50
  if (topPosition > window.innerHeight - 48) {
    topPosition = selectionType === 'sentence' ? top : top - 46
  }
  if (selectionType !== 'sentence' && topPosition < 60) {
    topPosition = bottom + 10
  }

  const rawLeft = selectionType === 'sentence' ? right : left + rect.width / 2
  const clampedLeft = Math.max(56, Math.min(window.innerWidth - 16, rawLeft))
  quickLookupPanelPosition.value = {
    top: topPosition,
    left: clampedLeft,
    sourceTop: top,
    sourceBottom: bottom,
  }
}

function scheduleQuickLookupPanelPositionUpdate() {
  requestAnimationFrame(() => updateQuickLookupPanelPosition())
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
    handleTooltipClose()
    clearHoveredAnnotation()
    if (popoverPosition.value.visible) {
      clearSelection()
    }
  }
  const onViewportChange = () => {
    if (quickLookupVisible.value) {
      scheduleQuickLookupPanelPositionUpdate()
    }
    updateToolbarVisibility()
  }
  const onWheel = (event: WheelEvent) => {
    if (!event.ctrlKey) return
    event.preventDefault()
    adjustFontSize(event.deltaY < 0 ? 1 : -1)
  }
  const onPointerMove = (event: MouseEvent) => {
    const frameElement = contents?.document?.defaultView?.frameElement as HTMLElement | null
    const candidatePoints = [{ x: event.clientX, y: event.clientY }]
    if (frameElement) {
      const frameRect = frameElement.getBoundingClientRect()
      candidatePoints.push({
        x: frameRect.left + event.clientX,
        y: frameRect.top + event.clientY,
      })
    }

    const hovered = findHoveredAnnotation(candidatePoints)
    if (!hovered) {
      if (hoveredAnnotationKey) {
        clearHoveredAnnotation()
        handleTooltipLeave()
      }
      return
    }

    const hoveredKey = getAnnotationKey(hovered)
    if (hoveredKey === hoveredAnnotationKey) {
      return
    }

    clearHoveredAnnotation()
    hoveredAnnotationKey = hoveredKey
    setAnnotationVisualState(hovered, true)
    const mark = resolveAnnotationMark(hovered)
    if (mark?.element) {
      showTooltipForAnnotation(
        hovered.annotationId,
        hovered.annotationType,
        mark.element,
        readerHostRef.value,
      )
    }
  }
  const onPointerLeave = () => {
    if (!hoveredAnnotationKey) return
    clearHoveredAnnotation()
    handleTooltipLeave()
  }

  contents.document.addEventListener('mouseup', dismissIfCollapsed)
  contents.document.addEventListener('touchend', dismissIfCollapsed)
  contents.document.addEventListener('mousedown', onPointerDown)
  contents.document.addEventListener('touchstart', onPointerDown)
  contents.document.addEventListener('mousemove', onPointerMove)
  contents.document.addEventListener('mouseleave', onPointerLeave)
  contents.window?.addEventListener?.('scroll', onViewportChange, { passive: true })
  contents.window?.addEventListener?.('resize', onViewportChange)
  contents.document.addEventListener('wheel', onWheel, { passive: false })

  contentCleanupFns.push(() => {
    try {
      contents.document.removeEventListener('mouseup', dismissIfCollapsed)
      contents.document.removeEventListener('touchend', dismissIfCollapsed)
      contents.document.removeEventListener('mousedown', onPointerDown)
      contents.document.removeEventListener('touchstart', onPointerDown)
      contents.document.removeEventListener('mousemove', onPointerMove)
      contents.document.removeEventListener('mouseleave', onPointerLeave)
      contents.window?.removeEventListener?.('scroll', onViewportChange)
      contents.window?.removeEventListener?.('resize', onViewportChange)
      contents.document.removeEventListener('wheel', onWheel)
    } catch {}
  })
}

function normalizeHref(href?: string): string {
  return (href || '').split('#')[0]
}

function normalizeToc(nodes: any[] = [], parentUid = 'root'): TocNode[] {
  return nodes.map((node, index) => {
    const uid = `${parentUid}/${String(node.id || node.href || node.label || `node-${index}`)}-${index}`
    return {
      uid,
    id: node.id,
    label: node.label || node.title || '未命名章节',
    href: node.href || '',
      subitems: normalizeToc(node.subitems || node.children || [], uid),
    }
  })
}

function flattenToc(nodes: TocNode[]): TocNode[] {
  return nodes.flatMap((node) => [node, ...flattenToc(node.subitems)])
}

function resolveInitialTocHref(): string | null {
  const first = flattenToc(toc.value).find((node) => normalizeHref(node.href))
  return first?.href || null
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

function hasMeaningfulRenderedText(): boolean {
  for (const contents of getRenderedContents()) {
    const body = contents?.document?.body as HTMLElement | null | undefined
    if (!body) continue

    if (body.querySelector('p, h1, h2, h3, h4, h5, h6, li, blockquote, pre, table')) {
      return true
    }

    const compactText = (body.textContent || '').replace(/\s+/g, '')
    if (compactText.length >= 24) {
      return true
    }
  }

  return false
}

function resolveBookProgress(location: any): number | null {
  const start = location?.start
  if (!start) return null

  if (typeof start.percentage === 'number' && Number.isFinite(start.percentage)) {
    return Math.max(0, Math.min(1, start.percentage))
  }

  if (start.cfi && bookInstance?.locations?.length?.()) {
    try {
      const percentage = bookInstance.locations.percentageFromCfi(start.cfi)
      if (typeof percentage === 'number' && Number.isFinite(percentage)) {
        return Math.max(0, Math.min(1, percentage))
      }
    } catch (e) {
      console.warn('根据 CFI 计算图书进度失败:', e)
    }
  }

  return null
}

async function ensureBookLocationsGenerated() {
  if (!bookInstance?.locations?.generate || locationsReady) return
  if (locationsGenerationPromise) return locationsGenerationPromise

  locationsGenerationPromise = bookInstance.locations.generate(1600)
    .then(() => {
      locationsReady = true
      const location = rendition?.currentLocation?.()
      if (location) {
        handleRelocated(location)
      }
    })
    .catch((e: any) => {
      console.warn('生成图书进度 locations 失败:', e)
    })
    .finally(() => {
      locationsGenerationPromise = null
    })

  return locationsGenerationPromise
}

function applyReaderTheme() {
  if (!rendition?.themes) return

  const mode = currentTheme.value
  const themeName = getBookReaderThemeName(mode)
  const palette = getBookReaderPalette(mode)
  const bodyFontSize = bookReaderFontSizeMap[fontSize.value]
  const bodyLineHeight = bookReaderDensityLineHeightMap[readerDensity.value]

  if (!registeredReaderThemes.has(themeName)) {
    rendition.themes.registerCss(themeName, buildBookReaderThemeCss(mode))
    registeredReaderThemes.add(themeName)
  }

  rendition.themes.select(themeName)
  rendition.themes.override('color', palette.text, true)
  rendition.themes.override('background', palette.bgLight, true)
  rendition.themes.override('font-size', bodyFontSize, true)
  rendition.themes.override('line-height', bodyLineHeight, true)
  rendition.themes.font("'Georgia', 'Times New Roman', serif")
}

function syncRenderedReaderTheme() {
  if (!rendition) return
  applyReaderTheme()
  for (const contents of getRenderedContents()) {
    applyInlineThemeOverride(contents)
  }
}

async function finalizeDisplayedContent() {
  await nextTick()
  syncRenderedReaderTheme()
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

function cycleReaderWidth() {
  const widths: ReaderWidth[] = ['narrow', 'medium', 'wide']
  const idx = widths.indexOf(readerWidth.value)
  readerWidth.value = widths[(idx + 1) % widths.length]
  void persistReaderPreference(READER_WIDTH_SETTING_KEY, readerWidth.value)
}

function cycleReaderDensity() {
  const densities: ReaderDensity[] = ['compact', 'balanced', 'relaxed']
  const idx = densities.indexOf(readerDensity.value)
  readerDensity.value = densities[(idx + 1) % densities.length]
  void persistReaderPreference(READER_DENSITY_SETTING_KEY, readerDensity.value)
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

function applyStoredReaderPreferences() {
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
}

async function persistReaderPreference(key: string, value: string) {
  try {
    await settingsStore.setSetting(key, value)
  } catch (error) {
    console.error(`保存阅读器偏好失败: ${key}`, error)
  }
}

function updateToolbarVisibility() {
  const contentScrollTops = getRenderedContents()
    .map((contents) => {
      const doc = contents?.document
      if (!doc) return 0
      return Math.max(
        doc.documentElement?.scrollTop || 0,
        doc.body?.scrollTop || 0,
        contents?.window?.scrollY || 0,
      )
    })

  const maxScrollTop = contentScrollTops.length ? Math.max(...contentScrollTops) : getShellScrollContainer().scrollTop
  showTopToolbar.value = maxScrollTop > BOOK_TOOLBAR_SCROLL_THRESHOLD
}

function handleReaderWheel(event: WheelEvent) {
  if (!event.ctrlKey) return
  event.preventDefault()
  adjustFontSize(event.deltaY < 0 ? 1 : -1)
}

function syncBookUpdate(updated: EbookItem) {
  currentEbook.value = updated
  emit('updated', updated)
}

function getAnnotationKey(entry: Pick<BookAnnotationEntry, 'annotationId' | 'annotationType'>) {
  return `${entry.annotationType}:${entry.annotationId}`
}

function applyAnnotationAttrs(element: Element | null | undefined, attrs: Record<string, string>) {
  if (!element) return

  const applyTo = (node: Element) => {
    for (const [name, value] of Object.entries(attrs)) {
      node.setAttribute(name, value)
    }
  }

  applyTo(element)
  for (const child of Array.from(element.children)) {
    applyTo(child)
  }
}

function resolveAnnotationMark(entry: Pick<BookAnnotationEntry, 'annotation' | 'mark'>) {
  return entry.annotation?.mark ?? entry.mark ?? null
}

function setAnnotationVisualState(entry: BookAnnotationEntry, hovered: boolean) {
  applyAnnotationAttrs(resolveAnnotationMark(entry)?.element, hovered ? entry.hoverAttrs : entry.idleAttrs)
}

function clearHoveredAnnotation() {
  if (!hoveredAnnotationKey) return
  const current = renderedAnnotations.find((entry) => getAnnotationKey(entry) === hoveredAnnotationKey)
  if (current) {
    setAnnotationVisualState(current, false)
  }
  hoveredAnnotationKey = null
}

function rectContains(rect: DOMRect, clientX: number, clientY: number) {
  const left = rect.left
  const top = rect.top
  const right = left + rect.width
  const bottom = top + rect.height
  return clientX >= left && clientX <= right && clientY >= top && clientY <= bottom
}

function markContainsPoint(
  mark: BookAnnotationEntry['mark'] | null | undefined,
  points: Array<{ x: number; y: number }>,
) {
  if (!mark?.getBoundingClientRect || !mark?.getClientRects) return false

  const bounds = mark.getBoundingClientRect()
  if (!points.some((point) => rectContains(bounds, point.x, point.y))) {
    return false
  }

  const rects = Array.from(mark.getClientRects?.() || [])
  if (rects.length === 0) {
    return true
  }

  return rects.some((rect) =>
    points.some((point) => rectContains(rect, point.x, point.y)),
  )
}

function findHoveredAnnotation(points: Array<{ x: number; y: number }>) {
  for (let index = renderedAnnotations.length - 1; index >= 0; index -= 1) {
    const entry = renderedAnnotations[index]
    if (markContainsPoint(resolveAnnotationMark(entry), points)) {
      return entry
    }
  }
  return null
}

function clearRenderedAnnotations() {
  clearHoveredAnnotation()
  handleTooltipClose()

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
    const key = `word-highlight:${item.ebook_cfi}`
    if (seen.has(key)) continue
    seen.add(key)
    const isFocused = activeHighlightId.value === item.id && activeHighlightType.value === 'word'
    const isDark = currentTheme.value === 'dark'
    const idleAttrs = {
      fill: isDark
        ? (isFocused ? '#F59E0B' : '#FBBF24')
        : (isFocused ? '#f43f5e' : '#fb7185'),
      'fill-opacity': isDark
        ? (isFocused ? '0.34' : '0.20')
        : (isFocused ? '0.28' : '0.16'),
      'mix-blend-mode': 'multiply',
    }
    const hoverAttrs = {
      ...idleAttrs,
      'fill-opacity': isDark
        ? (isFocused ? '0.42' : '0.28')
        : (isFocused ? '0.34' : '0.24'),
    }
    try {
      const annotation = rendition.annotations.highlight(
        item.ebook_cfi,
        { id: item.id, type: 'word' },
        undefined,
        isFocused ? 'book-word-highlight book-word-highlight--focus' : 'book-word-highlight',
        idleAttrs,
      )
      renderedAnnotations.push({
        cfi: item.ebook_cfi,
        type: 'highlight',
        annotationId: item.id,
        annotationType: 'word',
        annotation,
        mark: annotation?.mark ?? null,
        idleAttrs,
        hoverAttrs,
      })
    } catch (e) {
      console.warn('恢复图书单词高亮失败:', e)
    }
  }

  for (const item of sentences.value) {
    if (!item.ebook_cfi) continue
    const key = `sentence-highlight:${item.ebook_cfi}`
    if (seen.has(key)) continue
    seen.add(key)
    const isFocused = activeHighlightId.value === item.id && activeHighlightType.value === 'sentence'
    const isDark = currentTheme.value === 'dark'
    const idleAttrs = {
      fill: isDark
        ? (isFocused ? '#14B8A6' : '#2DD4BF')
        : (isFocused ? '#3b82f6' : '#60a5fa'),
      'fill-opacity': isDark
        ? (isFocused ? '0.30' : '0.18')
        : (isFocused ? '0.26' : '0.16'),
      'mix-blend-mode': 'multiply',
    }
    const hoverAttrs = {
      ...idleAttrs,
      'fill-opacity': isDark
        ? (isFocused ? '0.38' : '0.26')
        : (isFocused ? '0.34' : '0.24'),
    }
    try {
      const annotation = rendition.annotations.highlight(
        item.ebook_cfi,
        { id: item.id, type: 'sentence' },
        undefined,
        isFocused ? 'book-sentence-highlight book-sentence-highlight--focus' : 'book-sentence-highlight',
        idleAttrs,
      )
      renderedAnnotations.push({
        cfi: item.ebook_cfi,
        type: 'highlight',
        annotationId: item.id,
        annotationType: 'sentence',
        annotation,
        mark: annotation?.mark ?? null,
        idleAttrs,
        hoverAttrs,
      })
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
  const progress = resolveBookProgress(location)
  currentChapter.value = resolveChapterLabel(start?.href)
  if (progress !== null) {
    queueProgressSave(progress, start?.cfi || undefined)
  } else if (start?.cfi && currentEbook.value?.cfi_position !== start.cfi) {
    queueProgressSave(currentEbook.value?.progress || 0, start.cfi)
  }
  scheduleQuickLookupPanelPositionUpdate()
  updateToolbarVisibility()
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
  removeInlineSentenceTranslation()
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
  locationsReady = false
  locationsGenerationPromise = null

  if (readerHostRef.value) {
    readerHostRef.value.innerHTML = ''
  }
}

async function initReader() {
  loading.value = true
  error.value = null
  toc.value = []
  expandedTocKeys.value = {}
  currentChapter.value = ''
  cleanupReader()

  try {
    await settingsStore.loadSettings()
    applyStoredReaderPreferences()
    const latest = await getEbook(props.ebook.id).catch(() => props.ebook)
    currentEbook.value = latest
    await loadBookAnnotations()

    await nextTick()
    if (!readerHostRef.value || !currentEbook.value) {
      throw new Error('阅读容器未初始化')
    }

    const assetUrl = convertFileSrc(currentEbook.value.file_path)
    bookInstance = ePub(assetUrl)
    locationsReady = Boolean(bookInstance.locations?.length?.())
    locationsGenerationPromise = null

    const navigation = await bookInstance.loaded.navigation
    toc.value = normalizeToc(navigation?.toc || [])
    expandedTocKeys.value = loadExpandedTocState(latest.id)
    currentChapter.value = toc.value[0]?.label || ''
    const initialTocHref = resolveInitialTocHref()
    const initialTarget = props.focusCfi || currentEbook.value.cfi_position || initialTocHref || undefined

    rendition = bookInstance.renderTo(readerHostRef.value, {
      manager: 'default',
      width: '100%',
      height: '100%',
      flow: 'scrolled-doc',
      spread: 'none',
      allowScriptedContent: true,
    })

    applyReaderTheme()
    rendition.hooks.content.register((contents: any) => {
      applyInlineThemeOverride(contents)
      attachContentInteraction(contents)
    })
    rendition.on('relocated', handleRelocated)
    rendition.on('selected', handleSelected)
    await rendition.display(initialTarget)
    await finalizeDisplayedContent()

    // Some EPUBs place cover / front-matter ahead of the actual TOC chapter.
    // If first open lands on an effectively empty page, fall back to the first TOC entry.
    if (!props.focusCfi && initialTocHref && initialTarget !== initialTocHref && !hasMeaningfulRenderedText()) {
      await rendition.display(initialTocHref)
      await finalizeDisplayedContent()
    }

    updateToolbarVisibility()
    void ensureBookLocationsGenerated()
    setActiveHighlight(props.highlightId, props.highlightType)
    syncBookAnnotations()
    scheduleQuickLookupPanelPositionUpdate()
  } catch (e: any) {
    error.value = e?.message || String(e)
    toast.error('打开图书失败: ' + error.value)
  } finally {
    loading.value = false
  }
}

async function openChapter(item: TocNode) {
  if (!rendition || !item.href) return
  removeInlineSentenceTranslation()
  clearSelection()
  await rendition.display(item.href || normalizeHref(item.href))
  await finalizeDisplayedContent()
  currentChapter.value = item.label
}

function prevPage() {
  removeInlineSentenceTranslation()
  clearSelection()
  rendition?.prev?.()
}

function nextPage() {
  removeInlineSentenceTranslation()
  clearSelection()
  rendition?.next?.()
}

onMounted(() => {
  void initReader()
  readerShellRef.value?.addEventListener('wheel', handleReaderWheel, { passive: false })
  getShellScrollContainer().addEventListener('scroll', scheduleQuickLookupPanelPositionUpdate, { passive: true })
  getShellScrollContainer().addEventListener('scroll', updateToolbarVisibility, { passive: true })
  window.addEventListener('resize', scheduleQuickLookupPanelPositionUpdate)
  window.addEventListener('resize', updateToolbarVisibility)
})

onActivated(() => {
  syncRenderedReaderTheme()
  scheduleQuickLookupPanelPositionUpdate()
  updateToolbarVisibility()
})

onUnmounted(() => {
  readerShellRef.value?.removeEventListener('wheel', handleReaderWheel)
  getShellScrollContainer().removeEventListener('scroll', scheduleQuickLookupPanelPositionUpdate)
  getShellScrollContainer().removeEventListener('scroll', updateToolbarVisibility)
  window.removeEventListener('resize', scheduleQuickLookupPanelPositionUpdate)
  window.removeEventListener('resize', updateToolbarVisibility)
  shellScrollContainer = null
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
  async (newCfi, oldCfi) => {
    if (!newCfi || newCfi === oldCfi || !rendition) return
    clearSelection()
    setActiveHighlight(props.highlightId, props.highlightType)
    await rendition.display(newCfi)
    await finalizeDisplayedContent()
  },
)

watch(
  () => [quickLookupVisible.value, quickLookupSelectedText.value] as const,
  () => {
    if (!quickLookupVisible.value) {
      quickLookupPanelPosition.value = null
      return
    }
    nextTick(() => scheduleQuickLookupPanelPositionUpdate())
  },
)

watch(showToc, async () => {
  persistTocVisibility()
  await nextTick()
  if (!rendition || !readerHostRef.value) return

  const width = readerHostRef.value.clientWidth
  const height = readerHostRef.value.clientHeight
  if (width > 0 && height > 0) {
    rendition.resize?.(width, height)
  }
  scheduleQuickLookupPanelPositionUpdate()
  updateToolbarVisibility()
})

watch([currentTheme, fontSize, readerDensity], () => {
  if (!rendition) return
  syncRenderedReaderTheme()
  scheduleQuickLookupPanelPositionUpdate()
})

watch(readerWidth, async () => {
  await nextTick()
  if (!rendition || !readerHostRef.value) return
  const width = readerHostRef.value.clientWidth
  const height = readerHostRef.value.clientHeight
  if (width > 0 && height > 0) {
    rendition.resize?.(width, height)
  }
  scheduleQuickLookupPanelPositionUpdate()
  updateToolbarVisibility()
})
</script>

<template>
  <section ref="readerShellRef" class="book-reader" :style="readerShellStyle">
    <transition name="slide-down">
      <div v-if="showTopToolbar" class="reader-toolbar glass-card">
        <div class="reader-actions">
          <button class="header-btn" @click="emit('close')">← 返回书架</button>
          <button class="header-btn" @click="showToc = !showToc">{{ showToc ? '隐藏目录' : '显示目录' }}</button>
          <button class="header-btn" @click="prevPage">上一页</button>
          <button class="header-btn primary" @click="nextPage">下一页</button>
        </div>
        <div class="reader-toolbar-title">
          <div class="reader-toolbar-title__main">{{ readerToolbarTitle }}</div>
          <div class="reader-toolbar-title__sub">{{ readerToolbarSubtitle }}</div>
        </div>
        <div class="reader-toolbar-meta">
          <button class="header-btn reader-display-btn" @click="cycleFontSize" :title="'字号: ' + bookReaderFontSizeLabel[fontSize]">
            字号 {{ bookReaderFontSizeLabel[fontSize] }}
          </button>
          <button class="header-btn reader-display-btn" @click="cycleReaderWidth" :title="'正文宽度: ' + bookReaderWidthLabel[readerWidth]">
            宽度 {{ bookReaderWidthLabel[readerWidth] }}
          </button>
          <button class="header-btn reader-display-btn" @click="cycleReaderDensity" :title="'正文比例: ' + bookReaderDensityLabel[readerDensity]">
            比例 {{ bookReaderDensityLabel[readerDensity] }}
          </button>
          <span class="reader-progress-pill">{{ progressPercent }}%</span>
        </div>
      </div>
    </transition>

    <div class="reader-layout" :class="{ 'reader-layout--no-toc': !showToc }">
      <aside v-if="showToc" class="reader-toc glass-card">
        <button class="reader-edge-toggle header-btn header-btn--toc" @click="showToc = false">
          隐藏目录
        </button>
        <div class="toc-book-meta">
          <h1 class="reader-title">{{ currentEbook?.title || ebook.title }}</h1>
          <p class="reader-subtitle">
            <span v-if="currentEbook?.author">{{ currentEbook.author }}</span>
            <span>{{ currentChapter || '正在加载章节' }}</span>
            <span>{{ progressPercent }}%</span>
          </p>
        </div>
        <div class="toc-header">
          <h3>目录</h3>
          <div class="toc-header-actions">
            <span>{{ totalTocCount }} 项</span>
          </div>
        </div>
        <div v-if="toc.length === 0" class="toc-empty">该图书未提供可用目录</div>
        <div v-else class="toc-tree">
          <div
            v-for="node in visibleToc"
            :key="node.uid"
            class="toc-row"
            :style="{ paddingLeft: `${node.depth * 16}px` }"
          >
            <div
              class="toc-node"
              :class="{ 'toc-node--active': currentChapter === node.label }"
            >
              <button
                class="toc-node__main"
                type="button"
                @click="openChapter(node)"
              >
                <span class="toc-node__label">{{ node.label }}</span>
              </button>
              <button
                v-if="node.hasChildren"
                class="toc-node__toggle"
                type="button"
                :title="node.expanded ? '收起子目录' : '展开子目录'"
                @click.stop="toggleTocNode(node.uid)"
              >
                <svg
                  class="toc-node__toggle-icon"
                  :class="{ 'toc-node__toggle-icon--expanded': node.expanded }"
                  viewBox="0 0 16 16"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.8"
                >
                  <path d="M5 3.5L10.5 8L5 12.5" />
                </svg>
              </button>
            </div>
          </div>
        </div>
      </aside>

      <div v-else class="reader-mini-meta glass-card">
        <button class="reader-edge-toggle header-btn header-btn--toc" @click="showToc = true">
          显示目录
        </button>
        <div class="reader-mini-title">{{ currentEbook?.title || ebook.title }}</div>
        <div class="reader-mini-subtitle">
          <span v-if="currentEbook?.author">{{ currentEbook.author }}</span>
          <span>{{ currentChapter || '正在加载章节' }}</span>
          <span>{{ progressPercent }}%</span>
        </div>
      </div>

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
      :content-element="readerHostRef"
      :selected-text="quickLookupSelectedText"
      :context-text="quickLookupContextText"
      :loading="quickLookupLoading"
      :deep-loading="quickLookupDeepLoading"
      :saving="quickLookupSaving"
      :error="quickLookupError"
      :deep-error="quickLookupDeepError"
      :word-pos="quickLookupWordPos"
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
  </section>
</template>

<style scoped>
.book-reader {
  --book-reader-host-width: 760px;
  display: flex;
  flex-direction: column;
  gap: 0;
  width: 100%;
  padding: 8px 24px 28px;
}

.reader-toolbar {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  z-index: 80;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 20px;
  border-radius: 0;
  border-left: none;
  border-right: none;
  box-shadow: 0 1px 8px rgba(0, 0, 0, 0.04);
  transition: background-color 0.25s ease, border-color 0.25s ease, box-shadow 0.25s ease;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.08);
}

.slide-down-enter-active,
.slide-down-leave-active {
  transition: opacity 0.2s ease, transform 0.24s ease;
}

.slide-down-enter-from,
.slide-down-leave-to {
  opacity: 0;
  transform: translateY(-12px);
}

.reader-title {
  margin: 0;
  font-size: 1.15rem;
  color: var(--c-text);
  line-height: 1.4;
}

.reader-subtitle {
  margin: 0;
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  font-size: 0.84rem;
  color: var(--c-text-lighter);
}

.reader-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.reader-toolbar-title {
  min-width: 0;
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.reader-toolbar-title__main {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.92rem;
  font-weight: 700;
  color: var(--c-text);
}

.reader-toolbar-title__sub {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.78rem;
  color: var(--c-text-lighter);
}

.reader-toolbar-meta {
  display: inline-flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  flex-wrap: wrap;
}

.reader-progress-pill {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 58px;
  height: 36px;
  padding: 0 12px;
  border-radius: 10px;
  background: var(--c-overlay-bg);
  border: 1px solid var(--c-border);
  color: var(--c-text-lighter);
  font-size: 0.88rem;
  font-weight: 700;
}

.reader-display-btn {
  min-width: 92px;
  justify-content: center;
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

.header-btn--toc {
  padding: 6px 12px;
  font-size: 0.82rem;
}

.reader-layout {
  position: relative;
  display: grid;
  grid-template-columns: 280px minmax(0, 1fr);
  align-items: start;
  gap: 24px;
  width: 100%;
  min-height: calc(100vh - 72px);
}

.reader-layout--no-toc {
  display: block;
}

.reader-mini-meta {
  position: absolute;
  top: 0;
  left: 0;
  z-index: 25;
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-width: 280px;
  padding: 28px 12px 10px;
  border-radius: 14px;
}

.reader-mini-title {
  font-size: 0.92rem;
  font-weight: 700;
  line-height: 1.35;
  color: var(--c-text);
}

.reader-mini-subtitle {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  font-size: 0.78rem;
  line-height: 1.45;
  color: var(--c-text-lighter);
}

.reader-layout--no-toc .reader-stage {
  width: min(100%, calc(var(--book-reader-host-width) + 160px));
  margin: 0 auto;
}

.glass-card {
  background: var(--c-glass-bg);
  border: 1px solid var(--c-glass-border);
  box-shadow: var(--c-shadow-lg);
  backdrop-filter: blur(14px);
  -webkit-backdrop-filter: blur(14px);
}

.reader-toc {
  align-self: start;
  position: relative;
  border-radius: 18px;
  padding: 30px 16px 16px;
  overflow: auto;
  max-height: calc(100vh - 72px);
}

.reader-edge-toggle {
  position: absolute;
  top: 8px;
  left: 12px;
  z-index: 32;
  padding: 5px 12px;
  border-radius: 999px;
  background: var(--c-overlay-bg-strong);
  border-color: var(--c-border);
  box-shadow: 0 8px 18px rgba(15, 23, 42, 0.12);
}

.toc-book-meta {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-bottom: 18px;
  padding-bottom: 16px;
  border-bottom: 1px solid var(--c-overlay-border);
}

.toc-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 12px;
}

.toc-header-actions {
  display: inline-flex;
  align-items: center;
  gap: 8px;
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
  gap: 6px;
}

.toc-row {
  display: block;
}

.toc-node {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 0;
  padding: 0;
  border-radius: 10px;
  border: 1px solid var(--c-border);
  background: var(--c-bg-light);
  color: var(--c-text);
  transition: all 0.15s ease;
}

.toc-node:hover {
  border-color: var(--c-primary);
  background: var(--c-primary-light);
}

.toc-node--active {
  border-color: var(--c-primary);
  background: var(--c-primary-light);
  color: var(--c-primary-dark);
  font-weight: 700;
}

.toc-node__main {
  flex: 1;
  min-width: 0;
  display: inline-flex;
  align-items: center;
  min-height: 44px;
  padding: 10px 8px 10px 12px;
  border: none;
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: pointer;
  font: inherit;
}

.toc-node__label {
  display: block;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.toc-node__toggle {
  flex: 0 0 auto;
  width: 32px;
  height: 32px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  margin: 6px 8px 6px 0;
  padding: 0;
  border: none;
  border-radius: 10px;
  background: rgba(148, 163, 184, 0.08);
  color: var(--c-text-lighter);
  cursor: pointer;
  transition: background 0.18s ease, color 0.18s ease;
}

.toc-node__toggle:hover {
  background: rgba(148, 163, 184, 0.18);
  color: var(--c-text);
}

.toc-node__toggle-icon {
  width: 18px;
  height: 18px;
  transition: transform 0.18s ease;
}

.toc-node__toggle-icon--expanded {
  transform: rotate(90deg);
}

.reader-stage {
  position: relative;
  border-radius: 20px;
  padding: 16px;
  min-height: calc(100vh - 72px);
  overflow: hidden;
  width: 100%;
}

.reader-host {
  width: min(100%, var(--book-reader-host-width));
  height: calc(100vh - 104px);
  margin: 0 auto;
  border-radius: 14px;
  overflow: hidden;
  background: var(--c-bg-light);
  box-shadow: inset 0 0 0 1px var(--c-border-light);
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
  background: var(--c-overlay-bg);
  backdrop-filter: blur(2px);
  -webkit-backdrop-filter: blur(2px);
}

.stage-state--error {
  color: #dc2626;
}

@media (min-width: 1260px) {
  .reader-layout {
    display: block;
  }

  .reader-toc {
    position: absolute;
    top: 0;
    left: 0;
    width: 280px;
  }

  .reader-stage,
  .reader-layout--no-toc .reader-stage {
    width: min(100%, 920px);
    margin: 0 auto;
  }
}

@media (max-width: 960px) {
  .book-reader {
    padding-top: 8px;
  }

  .reader-toolbar {
    padding: 8px 12px;
    justify-content: space-between;
    flex-wrap: wrap;
  }

  .reader-toolbar-title {
    order: 3;
    width: 100%;
  }

  .reader-layout {
    grid-template-columns: 1fr;
  }

  .reader-mini-meta {
    left: 8px;
    right: 8px;
    max-width: none;
  }

  .reader-toc {
    max-height: 240px;
  }

  .reader-toolbar-meta {
    min-width: 0;
  }
}
</style>
