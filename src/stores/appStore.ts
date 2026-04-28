import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { ArticleItem, VocabularyItem, SentenceItem, EbookItem } from '../services/api'
import { getArticles, getVocabulary, getSentences, getEbooks } from '../services/api'

export const useAppStore = defineStore('app', () => {
  const articles = ref<ArticleItem[]>([])
  const articlesLoaded = ref(false)

  const ebooks = ref<EbookItem[]>([])
  const ebooksLoaded = ref(false)

  const vocabulary = ref<VocabularyItem[]>([])
  const vocabularyLoaded = ref(false)

  const sentences = ref<SentenceItem[]>([])
  const sentencesLoaded = ref(false)

  const isLoading = ref(false)
  const globalMessage = ref<string | null>(null)
  const sidebarExpanded = ref(false)

  const currentArticle = ref<ArticleItem | null>(null)
  const currentEbook = ref<EbookItem | null>(null)
  const readerFontSize = ref<'small' | 'medium' | 'large'>('medium')

  const pendingOcrDraft = ref<{ title: string; content: string } | null>(null)

  const articlesCount = computed(() => articles.value.length)
  const ebooksCount = computed(() => ebooks.value.length)
  const vocabularyCount = computed(() => vocabulary.value.length)
  const sentencesCount = computed(() => sentences.value.length)

  const totalWordCount = computed(() =>
    articles.value.reduce((sum, a) => sum + (a.word_count || 0), 0)
  )

  const stats = computed(() => ({
    articles: articlesCount.value,
    ebooks: ebooksCount.value,
    vocabulary: vocabularyCount.value,
    sentences: sentencesCount.value,
    totalWords: totalWordCount.value,
  }))

  const recentEbooks = computed(() =>
    [...ebooks.value].sort((a, b) => (b.last_read_at || b.created_at) - (a.last_read_at || a.created_at))
  )

  function setArticles(data: ArticleItem[]) {
    articles.value = data
    articlesLoaded.value = true
  }

  function addArticle(item: ArticleItem) {
    articles.value.unshift(item)
  }

  function removeArticle(id: string) {
    articles.value = articles.value.filter(a => a.id !== id)
  }

  function updateArticle(item: ArticleItem) {
    const index = articles.value.findIndex(a => a.id === item.id)
    if (index > -1) {
      articles.value[index] = item
    }
  }

  function setEbooks(data: EbookItem[]) {
    ebooks.value = data
    ebooksLoaded.value = true
  }

  function addEbook(item: EbookItem) {
    ebooks.value.unshift(item)
  }

  function removeEbook(id: string) {
    ebooks.value = ebooks.value.filter(item => item.id !== id)
  }

  function updateEbook(item: EbookItem) {
    const index = ebooks.value.findIndex(entry => entry.id === item.id)
    if (index > -1) {
      ebooks.value[index] = item
    }
  }

  function setVocabulary(data: VocabularyItem[]) {
    vocabulary.value = data
    vocabularyLoaded.value = true
  }

  function addVocabularyItem(item: VocabularyItem) {
    vocabulary.value.unshift(item)
  }

  function removeVocabularyItem(id: string) {
    vocabulary.value = vocabulary.value.filter(v => v.id !== id)
  }

  function setSentences(data: SentenceItem[]) {
    sentences.value = data
    sentencesLoaded.value = true
  }

  function addSentenceItem(item: SentenceItem) {
    sentences.value.unshift(item)
  }

  function removeSentenceItem(id: string) {
    sentences.value = sentences.value.filter(s => s.id !== id)
  }

  function setLoading(value: boolean) {
    isLoading.value = value
  }

  function setGlobalMessage(msg: string | null) {
    globalMessage.value = msg
  }

  function clearCache() {
    articles.value = []
    ebooks.value = []
    vocabulary.value = []
    sentences.value = []
    articlesLoaded.value = false
    ebooksLoaded.value = false
    vocabularyLoaded.value = false
    sentencesLoaded.value = false
  }

  function setPendingOcrDraft(title: string, content: string) {
    pendingOcrDraft.value = { title, content }
  }

  function consumePendingOcrDraft() {
    const draft = pendingOcrDraft.value
    pendingOcrDraft.value = null
    return draft
  }

  function toggleSidebar() {
    sidebarExpanded.value = !sidebarExpanded.value
  }

  async function fetchArticles(force = false) {
    if (articlesLoaded.value && !force) return articles.value
    try {
      const data = await getArticles()
      setArticles(data)
      return data
    } catch (e) {
      console.error('加载文章失败:', e)
      if (!articlesLoaded.value) setArticles([])
      return articles.value
    }
  }

  async function fetchEbooks(force = false) {
    if (ebooksLoaded.value && !force) return ebooks.value
    try {
      const data = await getEbooks()
      setEbooks(data)
      return data
    } catch (e) {
      console.error('加载图书失败:', e)
      if (!ebooksLoaded.value) setEbooks([])
      return ebooks.value
    }
  }

  async function fetchVocabulary(force = false) {
    if (vocabularyLoaded.value && !force) return vocabulary.value
    try {
      const data = await getVocabulary()
      setVocabulary(data)
      return data
    } catch (e) {
      console.error('加载生词失败:', e)
      if (!vocabularyLoaded.value) setVocabulary([])
      return vocabulary.value
    }
  }

  async function fetchSentences(force = false) {
    if (sentencesLoaded.value && !force) return sentences.value
    try {
      const data = await getSentences()
      setSentences(data)
      return data
    } catch (e) {
      console.error('加载句子失败:', e)
      if (!sentencesLoaded.value) setSentences([])
      return sentences.value
    }
  }

  return {
    articles,
    articlesLoaded,
    ebooks,
    ebooksLoaded,
    vocabulary,
    vocabularyLoaded,
    sentences,
    sentencesLoaded,
    isLoading,
    globalMessage,
    sidebarExpanded,
    currentArticle,
    currentEbook,
    readerFontSize,
    pendingOcrDraft,

    articlesCount,
    ebooksCount,
    vocabularyCount,
    sentencesCount,
    totalWordCount,
    stats,
    recentEbooks,

    setArticles,
    addArticle,
    removeArticle,
    updateArticle,
    setEbooks,
    addEbook,
    removeEbook,
    updateEbook,
    setVocabulary,
    addVocabularyItem,
    removeVocabularyItem,
    setSentences,
    addSentenceItem,
    removeSentenceItem,
    setLoading,
    setGlobalMessage,
    clearCache,
    toggleSidebar,
    setPendingOcrDraft,
    consumePendingOcrDraft,
    fetchArticles,
    fetchEbooks,
    fetchVocabulary,
    fetchSentences,
  }
})
