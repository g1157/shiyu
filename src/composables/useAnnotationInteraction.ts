import { ref, type Ref } from 'vue'
import type { Router } from 'vue-router'
import type { HighlightType } from './useRouteQuery'
import { useGlobalToast } from './useGlobalToast'
import { translateText } from '../services/api'
import { buildSentenceExplanation } from '../utils/sentenceExplanation'
import { sanitizeParsedSentenceHtml } from '../utils/sanitizeHtml'

interface TooltipState {
  visible: boolean
  content: string
  type: HighlightType
  annotationId: string | null
  position: { top: number; left: number; anchor?: 'center' | 'start' }
}

interface ToastInstance {
  show: (msg: string) => void
}

interface TextSelectionAPI {
  selection: Ref<{ text: string; range: Range | null; type: string | null }>
  clearSelection: () => void
  getContext: () => string
  popoverPosition: Ref<{ visible: boolean; top: number; left: number }>
}

interface AnnotationAPI {
  annotationEnabled: Ref<boolean>
  findWordById: (id: string) => { meaning: string } | undefined
  findSentenceById: (id: string) => { explanation: string } | undefined
  saveWord: (word: string, meaning: string, context: string) => Promise<any>
  saveSentence: (text: string, explanation: string) => Promise<any>
  deleteWord: (id: string) => Promise<void>
  deleteSentence: (id: string) => Promise<void>
}

interface AnnotationDraftState {
  meaning: string
  sentenceTranslation: string
  structureParsed: string
  structureNote: string
}

interface QuickLookupAnchorPosition {
  top: number
  left: number
}

function parseModelResult(text: string) {
  const raw = text.trim()
  try {
    return JSON.parse(raw)
  } catch {
    const match = raw.match(/\{[\s\S]*\}/)
    if (match) {
      try {
        return JSON.parse(match[0])
      } catch {
        return null
      }
    }
    return null
  }
}

/**
 * Annotation interaction composable — 管理阅读器中的标注表单、tooltip、点击和 hover 事件
 */
export function useAnnotationInteraction(
  router: Router,
  toastRef: Ref<ToastInstance | null>,
  textSelectionAPI: TextSelectionAPI,
  annotationAPI: AnnotationAPI,
) {
  const { selection, clearSelection, getContext, popoverPosition } = textSelectionAPI
  const {
    annotationEnabled,
    findWordById,
    findSentenceById,
    saveWord,
    saveSentence,
    deleteWord,
    deleteSentence,
  } = annotationAPI

  // State
  const showAnnotationForm = ref(false)
  const annotationType = ref<HighlightType>('word')
  const cachedSelectedText = ref('')
  const cachedContextText = ref('')
  const annotationDraft = ref<AnnotationDraftState>({
    meaning: '',
    sentenceTranslation: '',
    structureParsed: '',
    structureNote: '',
  })
  const tooltipState = ref<TooltipState>({
    visible: false,
    content: '',
    type: 'word' as HighlightType,
    annotationId: null,
    position: { top: 0, left: 0, anchor: 'center' },
  })
  const quickLookupVisible = ref(false)
  const quickLookupType = ref<HighlightType>('word')
  const quickLookupSelectedText = ref('')
  const quickLookupContextText = ref('')
  const quickLookupWordPos = ref('')
  const quickLookupMeaning = ref('')
  const quickLookupBaseMeaning = ref('')
  const quickLookupOtherMeanings = ref<string[]>([])
  const quickLookupTranslation = ref('')
  const quickLookupParsedHtml = ref('')
  const quickLookupStructureNote = ref('')
  const quickLookupLoading = ref(false)
  const quickLookupDeepLoading = ref(false)
  const quickLookupSaving = ref(false)
  const quickLookupError = ref('')
  const quickLookupDeepError = ref('')
  const quickLookupAnchor = ref<QuickLookupAnchorPosition | null>(null)
  const quickLookupRange = ref<Range | null>(null)
  let tooltipHideTimeout: ReturnType<typeof setTimeout> | null = null
  let quickLookupRequestId = 0
  let quickLookupDeepRequestId = 0

  function clearTooltipHideTimeout() {
    if (tooltipHideTimeout) {
      clearTimeout(tooltipHideTimeout)
      tooltipHideTimeout = null
    }
  }

  function hideTooltip() {
    clearTooltipHideTimeout()
    tooltipState.value.visible = false
  }

  function scheduleTooltipHide(delay = 500) {
    clearTooltipHideTimeout()
    tooltipHideTimeout = setTimeout(() => {
      tooltipState.value.visible = false
    }, delay)
  }

  function resetAnnotationDraft() {
    annotationDraft.value = {
      meaning: '',
      sentenceTranslation: '',
      structureParsed: '',
      structureNote: '',
    }
  }

  function resetQuickLookupData() {
    quickLookupWordPos.value = ''
    quickLookupMeaning.value = ''
    quickLookupBaseMeaning.value = ''
    quickLookupOtherMeanings.value = []
    quickLookupTranslation.value = ''
    quickLookupParsedHtml.value = ''
    quickLookupStructureNote.value = ''
    quickLookupError.value = ''
    quickLookupDeepError.value = ''
  }

  function buildWordMeaningText() {
    const segments: string[] = []
    if (quickLookupWordPos.value.trim()) {
      segments.push(`词性：${quickLookupWordPos.value.trim()}`)
    }
    if (quickLookupMeaning.value.trim()) {
      segments.push(`中文释义：${quickLookupMeaning.value.trim()}`)
    }
    return segments.join(' - ') || quickLookupMeaning.value.trim()
  }

  async function runQuickLookup(type: HighlightType, text: string, contextText: string, requestId: number) {
    try {
      const req = type === 'word'
        ? {
            text: contextText ? `单词：${text}\n语境：${contextText}` : text,
            prompt_type: 'word_quick' as const,
          }
        : {
            text,
            prompt_type: 'sentence_quick' as const,
          }
      const { result } = await translateText(req)
      if (requestId !== quickLookupRequestId) return

      const content = result.trim()
      if (!content) {
        throw new Error('模型未返回结果。')
      }

      const parsed = parseModelResult(content)
      if (type === 'word') {
        quickLookupWordPos.value = String(parsed?.pos || '').trim()
        quickLookupMeaning.value = String(parsed?.meaning || parsed?.zh || content).trim()
        quickLookupBaseMeaning.value = String(parsed?.base_meaning || parsed?.core_meaning || '').trim()
        quickLookupOtherMeanings.value = Array.isArray(parsed?.other_meanings)
          ? parsed.other_meanings
            .map((item: unknown) => String(item || '').trim())
            .filter((item: string) => item && item !== quickLookupMeaning.value && item !== quickLookupBaseMeaning.value)
            .slice(0, 3)
          : []
        if (!quickLookupMeaning.value) {
          throw new Error('模型未返回可保存的释义。')
        }
      } else {
        quickLookupTranslation.value = String(parsed?.translation || content).trim()
        if (!quickLookupTranslation.value) {
          throw new Error('模型未返回可保存的句子释义。')
        }
      }
    } catch (error: any) {
      if (requestId !== quickLookupRequestId) return
      quickLookupError.value = error?.message || error?.toString() || '查询失败'
    } finally {
      if (requestId === quickLookupRequestId) {
        quickLookupLoading.value = false
      }
    }
  }

  function startQuickLookup(type: HighlightType) {
    if (!selection.value.text) return

    const selectedText = selection.value.text
    const contextText = getContext()
    const anchor = {
      top: popoverPosition.value.top,
      left: popoverPosition.value.left,
    }
    const rangeSnapshot = selection.value.range ? selection.value.range.cloneRange() : null

    quickLookupType.value = type
    quickLookupSelectedText.value = selectedText
    quickLookupContextText.value = contextText
    quickLookupAnchor.value = anchor
    quickLookupRange.value = rangeSnapshot
    quickLookupVisible.value = true
    quickLookupLoading.value = true
    quickLookupDeepLoading.value = false
    quickLookupSaving.value = false
    resetQuickLookupData()
    clearSelection()

    const requestId = ++quickLookupRequestId
    void runQuickLookup(type, selectedText, contextText, requestId)
  }

  // Methods
  function handleAddWord() {
    startQuickLookup('word')
  }

  function handleAddSentence() {
    startQuickLookup('sentence')
  }

  function handleCloseForm() {
    showAnnotationForm.value = false
    resetAnnotationDraft()
    clearSelection()
  }

  async function handleSaveAnnotation(content: string) {
    try {
      if (annotationType.value === 'word') {
        await saveWord(cachedSelectedText.value, content, cachedContextText.value)
        toastRef.value?.show('单词已添加到生词本')
      } else {
        await saveSentence(cachedSelectedText.value, content)
        toastRef.value?.show('长难句已保存')
      }
    } catch (error: any) {
      const toast = useGlobalToast()
      toast.error('保存失败: ' + error)
    }
    handleCloseForm()
  }

  function closeQuickLookup() {
    quickLookupRequestId += 1
    quickLookupDeepRequestId += 1
    quickLookupVisible.value = false
    quickLookupLoading.value = false
    quickLookupDeepLoading.value = false
    quickLookupSaving.value = false
    quickLookupSelectedText.value = ''
    quickLookupContextText.value = ''
    quickLookupAnchor.value = null
    quickLookupRange.value = null
    resetQuickLookupData()
  }

  function retryQuickLookup() {
    if (!quickLookupSelectedText.value) return
    quickLookupLoading.value = true
    quickLookupDeepLoading.value = false
    quickLookupSaving.value = false
    resetQuickLookupData()
    const requestId = ++quickLookupRequestId
    void runQuickLookup(
      quickLookupType.value,
      quickLookupSelectedText.value,
      quickLookupContextText.value,
      requestId,
    )
  }

  async function requestSentenceDeepAnalysis() {
    if (
      quickLookupType.value !== 'sentence'
      || !quickLookupSelectedText.value
      || quickLookupLoading.value
      || quickLookupDeepLoading.value
    ) {
      return
    }

    const requestId = ++quickLookupDeepRequestId
    quickLookupDeepLoading.value = true
    quickLookupDeepError.value = ''

    try {
      const { result } = await translateText({
        text: `句子：${quickLookupSelectedText.value}`,
        prompt_type: 'sentence_structure',
      })
      if (requestId !== quickLookupDeepRequestId) return

      const content = result.trim()
      const parsed = parseModelResult(content)
      if (!parsed || !parsed.parsed_html) {
        throw new Error('模型未返回结构化成分解析。')
      }

      quickLookupParsedHtml.value = sanitizeParsedSentenceHtml(String(parsed.parsed_html).trim())
      quickLookupStructureNote.value = String(parsed.structure_note || '').trim()
      const translation = String(parsed.translation || '').trim()
      if (translation) {
        quickLookupTranslation.value = translation
      }
    } catch (error: any) {
      if (requestId !== quickLookupDeepRequestId) return
      quickLookupDeepError.value = error?.message || error?.toString() || '深度解析失败'
    } finally {
      if (requestId === quickLookupDeepRequestId) {
        quickLookupDeepLoading.value = false
      }
    }
  }

  function openQuickLookupEditor() {
    cachedSelectedText.value = quickLookupSelectedText.value
    cachedContextText.value = quickLookupContextText.value
    annotationType.value = quickLookupType.value
    annotationDraft.value = {
      meaning: buildWordMeaningText(),
      sentenceTranslation: quickLookupTranslation.value,
      structureParsed: quickLookupParsedHtml.value,
      structureNote: quickLookupStructureNote.value,
    }
    showAnnotationForm.value = true
    quickLookupVisible.value = false
  }

  async function saveQuickLookup() {
    if (quickLookupSaving.value || quickLookupLoading.value) return

    quickLookupSaving.value = true
    try {
      if (quickLookupType.value === 'word') {
        await saveWord(
          quickLookupSelectedText.value,
          buildWordMeaningText(),
          quickLookupContextText.value,
        )
        toastRef.value?.show('单词已添加到生词本')
      } else {
        await saveSentence(
          quickLookupSelectedText.value,
          buildSentenceExplanation(
            quickLookupParsedHtml.value,
            quickLookupStructureNote.value,
            quickLookupTranslation.value,
          ),
        )
        toastRef.value?.show('长难句已保存')
      }
      closeQuickLookup()
    } catch (error: any) {
      const toast = useGlobalToast()
      toast.error('保存失败: ' + error)
    } finally {
      quickLookupSaving.value = false
    }
  }

  function toggleAnnotations() {
    annotationEnabled.value = !annotationEnabled.value
    if (!annotationEnabled.value) {
      clearSelection()
      tooltipState.value.visible = false
    }
  }

  function handleAnnotationClick(e: MouseEvent) {
    if (!annotationEnabled.value) return
    if (e.button !== 0) return

    const target = e.target as HTMLElement

    const wordEl = target.closest('.annotated-word, .annotated-word-subtle') as HTMLElement | null
    if (wordEl) {
      const id = wordEl.getAttribute('data-word-id')
      if (id) {
        e.preventDefault()
        e.stopPropagation()
        void router.push({ path: '/vocabulary', query: { highlight: id, type: 'word' } })
      }
      return
    }

    const sentenceEl = target.closest('.annotated-sentence, .annotated-sentence-subtle') as HTMLElement | null
    if (sentenceEl) {
      const id = sentenceEl.getAttribute('data-sentence-id')
      if (id) {
        e.preventDefault()
        e.stopPropagation()
        void router.push({ path: '/sentences', query: { highlight: id, type: 'sentence' } })
      }
    }
  }

  function resolveTooltipPosition(
    target: Element,
    type: HighlightType,
    contentElementOverride?: HTMLElement | null,
  ) {
    const rect = target.getBoundingClientRect()
    const frameElement = target.ownerDocument?.defaultView?.frameElement as HTMLElement | null
    const frameRect = frameElement?.getBoundingClientRect()
    const absoluteRect = {
      top: frameRect ? frameRect.top + rect.top : rect.top,
      right: frameRect ? frameRect.left + rect.right : rect.right,
      bottom: frameRect ? frameRect.top + rect.bottom : rect.bottom,
      left: frameRect ? frameRect.left + rect.left : rect.left,
      width: rect.width,
      height: rect.height,
    }
    if (type === 'sentence') {
      const contentElement =
        contentElementOverride
        || (target.closest?.('.reader-body') as HTMLElement | null)
      const contentRect = contentElement?.getBoundingClientRect()
      const container = contentElement?.closest('.page-container, .app-main') as HTMLElement | null
      const containerRect = container?.getBoundingClientRect()
      const tooltipWidth = Math.min(320, Math.max(220, window.innerWidth - 32))
      const top = Math.max(56, Math.min(absoluteRect.top, window.innerHeight - 140))
      const gap = 14
      const leftBoundary = containerRect?.left ?? 0
      const rightBoundary = containerRect?.right ?? window.innerWidth
      const rightSpace = (rightBoundary - (contentRect?.right ?? absoluteRect.right))
      const leftSpace = ((contentRect?.left ?? absoluteRect.left) - leftBoundary)

      if (contentRect && rightSpace >= tooltipWidth + gap + 8) {
        return {
          top,
          left: Math.min(contentRect.right + gap, rightBoundary - tooltipWidth - 8),
          anchor: 'start' as const,
        }
      }

      if (contentRect && leftSpace >= tooltipWidth + gap + 8) {
        return {
          top,
          left: Math.max(leftBoundary + 8, contentRect.left - tooltipWidth - gap),
          anchor: 'start' as const,
        }
      }
    }

    return {
      top: absoluteRect.bottom + 5,
      left: absoluteRect.left + absoluteRect.width / 2,
      anchor: 'center' as const,
    }
  }

  function getTooltipPayload(annotationId: string, type: HighlightType) {
    if (type === 'word') {
      const word = findWordById(annotationId)
      if (word) {
        return { content: word.meaning, annotationId }
      }
      return null
    }

    const sentence = findSentenceById(annotationId)
    if (sentence) {
      return { content: sentence.explanation, annotationId }
    }
    return null
  }

  function showTooltipForAnnotation(
    annotationId: string,
    type: HighlightType,
    target: Element,
    contentElementOverride?: HTMLElement | null,
  ) {
    const payload = getTooltipPayload(annotationId, type)
    if (!payload?.content) return

    clearTooltipHideTimeout()
    tooltipState.value = {
      visible: true,
      content: payload.content,
      type,
      annotationId: payload.annotationId,
      position: resolveTooltipPosition(target, type, contentElementOverride),
    }
  }

  function showTooltipContent(target: HTMLElement, type: HighlightType) {
    const annotationId =
      type === 'word'
        ? target.getAttribute('data-word-id')
        : target.getAttribute('data-sentence-id')

    if (annotationId) {
      showTooltipForAnnotation(annotationId, type, target)
    }
  }

  function handleHighlightHover(e: MouseEvent) {
    if (!annotationEnabled.value) return
    const target = e.target as HTMLElement

    const wordEl = target.closest('.annotated-word, .annotated-word-subtle') as HTMLElement | null
    const sentenceEl = target.closest('.annotated-sentence, .annotated-sentence-subtle') as HTMLElement | null

    if (!wordEl && !sentenceEl) return

    clearTooltipHideTimeout()

    if (wordEl) {
      showTooltipContent(wordEl, 'word')
      return
    }

    if (sentenceEl) {
      showTooltipContent(sentenceEl, 'sentence')
    }
  }

  function handleHighlightLeave(e: MouseEvent) {
    if (!tooltipState.value.visible) return
    const relatedTarget = e.relatedTarget as HTMLElement | null
    if (
      relatedTarget?.closest?.('.annotation-tooltip')
      || relatedTarget?.closest?.('.annotated-word, .annotated-word-subtle, .annotated-sentence, .annotated-sentence-subtle')
    ) {
      return
    }
    scheduleTooltipHide()
  }

  function handleTooltipClose() {
    hideTooltip()
  }

  function handleTooltipEnter() {
    clearTooltipHideTimeout()
  }

  function handleTooltipLeave() {
    scheduleTooltipHide(500)
  }

  async function handleTooltipRemove() {
    if (!tooltipState.value.annotationId) return

    const annotationId = tooltipState.value.annotationId
    const annotationType = tooltipState.value.type
    tooltipState.value.visible = false

    try {
      if (annotationType === 'word') {
        await deleteWord(annotationId)
      } else {
        await deleteSentence(annotationId)
      }
      toastRef.value?.show(annotationType === 'word' ? '已移出生词本' : '已移出句库')
    } catch (error: any) {
      const toast = useGlobalToast()
      toast.error('删除失败: ' + error)
    }
  }

  function handlePageClick(e: MouseEvent) {
    const target = e.target as HTMLElement

    if (!target.closest('.annotation-tooltip') && !target.closest('.annotated-word') && !target.closest('.annotated-sentence') && !target.closest('.annotated-word-subtle') && !target.closest('.annotated-sentence-subtle')) {
      hideTooltip()
    }

    if (showAnnotationForm.value) {
      return
    }

    if (!target.closest('.selection-popover')) {
      if (popoverPosition.value.visible) {
        clearSelection()
      }
    }
  }

  return {
    // State
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
    // Methods
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
    showTooltipForAnnotation,
    handlePageClick,
  }
}
