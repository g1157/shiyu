<script setup lang="ts">
import { computed, defineAsyncComponent, onMounted, ref, watch } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import { storeToRefs } from 'pinia'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { useRoute, useRouter } from 'vue-router'
import { deleteEbook, getEbooks, importEpubAsBook, type EbookItem } from '../services/api'
import DeleteConfirmModal from '../components/DeleteConfirmModal.vue'
import { useGlobalToast } from '../composables/useGlobalToast'
import type { HighlightType } from '../composables/useRouteQuery'
import { useAppStore } from '../stores/appStore'
import { formatDate } from '../utils/format'

const BookReader = defineAsyncComponent(() => import('../components/BookReader.vue'))

const route = useRoute()
const router = useRouter()
const toast = useGlobalToast()
const appStore = useAppStore()
const { ebooks, recentEbooks, ebooksCount } = storeToRefs(appStore)

const loading = ref(false)
const importing = ref(false)
const searchQuery = ref('')
const showReader = ref(false)
const readerBook = ref<EbookItem | null>(null)
const readerFocusCfi = ref<string | null>(null)
const readerHighlightId = ref<string | null>(null)
const readerHighlightType = ref<HighlightType | null>(null)
const deleteTarget = ref<EbookItem | null>(null)
const showDeleteConfirm = ref(false)

const normalizedSearch = computed(() => searchQuery.value.trim().toLowerCase())
const currentBook = computed(() => recentEbooks.value[0] ?? null)
const filteredBooks = computed(() => {
  if (!normalizedSearch.value) return recentEbooks.value
  return recentEbooks.value.filter((book) =>
    book.title.toLowerCase().includes(normalizedSearch.value)
    || (book.author?.toLowerCase().includes(normalizedSearch.value) ?? false),
  )
})
const hasBooks = computed(() => ebooksCount.value > 0)

function getQueryValue(value: unknown): string | null {
  if (typeof value === 'string') return value
  if (Array.isArray(value) && typeof value[0] === 'string') return value[0]
  return null
}

function normalizeHighlightType(value: string | null): HighlightType | null {
  if (value === 'word' || value === 'sentence') return value
  return null
}

function clearBookQuery() {
  const query = { ...route.query }
  delete query.bookId
  delete query.cfi
  delete query.highlight
  delete query.type
  void router.replace({ path: route.path, query })
}

async function loadBooks(force = false) {
  loading.value = true
  try {
    if (force || ebooks.value.length === 0) {
      const latest = await getEbooks()
      appStore.setEbooks(latest)
    }
  } catch (e: any) {
    toast.error('加载书架失败: ' + e)
    throw e
  } finally {
    loading.value = false
  }
}

function openReader(
  book: EbookItem,
  syncRoute = true,
  options?: {
    focusCfi?: string | null
    highlightId?: string | null
    highlightType?: HighlightType | null
  },
) {
  readerBook.value = book
  showReader.value = true
  readerFocusCfi.value = options?.focusCfi ?? null
  readerHighlightId.value = options?.highlightId ?? null
  readerHighlightType.value = options?.highlightType ?? null
  if (syncRoute) {
    const query: Record<string, string> = { bookId: book.id }
    if (options?.focusCfi) query.cfi = options.focusCfi
    if (options?.highlightId) query.highlight = options.highlightId
    if (options?.highlightType) query.type = options.highlightType
    void router.replace({ path: '/books', query })
  }
}

function closeReader() {
  showReader.value = false
  readerBook.value = null
  readerFocusCfi.value = null
  readerHighlightId.value = null
  readerHighlightType.value = null
  clearBookQuery()
}

async function openReaderFromRouteQuery() {
  const bookId = getQueryValue(route.query.bookId)
  if (!bookId) return
  const focusCfi = getQueryValue(route.query.cfi)
  const highlightId = getQueryValue(route.query.highlight)
  const highlightType = normalizeHighlightType(getQueryValue(route.query.type))

  if (ebooks.value.length === 0) {
    await loadBooks()
  }

  const target = ebooks.value.find((item) => item.id === bookId)
  if (!target) {
    clearBookQuery()
    return
  }

  openReader(target, false, {
    focusCfi,
    highlightId,
    highlightType,
  })
}

async function handleImportBook() {
  if (importing.value) return

  try {
    const selected = await openDialog({
      multiple: false,
      filters: [{ name: 'EPUB', extensions: ['epub'] }],
    })
    if (!selected) return

    importing.value = true
    const saved = await importEpubAsBook(selected as string)
    appStore.addEbook(saved)
    openReader(saved)
    toast.success(`《${saved.title}》已导入书架`)
  } catch (e: any) {
    toast.error('导入图书失败: ' + e)
  } finally {
    importing.value = false
  }
}

function handleDelete(book: EbookItem) {
  deleteTarget.value = book
  showDeleteConfirm.value = true
}

async function confirmDelete() {
  if (!deleteTarget.value) return

  try {
    await deleteEbook(deleteTarget.value.id)
    appStore.removeEbook(deleteTarget.value.id)
    if (readerBook.value?.id === deleteTarget.value.id) {
      closeReader()
    }
  } catch (e: any) {
    toast.error('删除图书失败: ' + e)
  } finally {
    showDeleteConfirm.value = false
    deleteTarget.value = null
  }
}

function cancelDelete() {
  showDeleteConfirm.value = false
  deleteTarget.value = null
}

function handleBookUpdated(updated: EbookItem) {
  appStore.updateEbook(updated)
  readerBook.value = updated
}

function formatPercent(progress: number) {
  return `${Math.round(Math.max(0, Math.min(1, progress)) * 100)}%`
}

function getBookStatus(book: EbookItem) {
  if (book.progress >= 1) return '已完成'
  if (book.progress > 0) return '在读'
  return '未开始'
}

function getBookStatusClass(book: EbookItem) {
  if (book.progress >= 1) return 'is-complete'
  if (book.progress > 0) return 'is-active'
  return 'is-new'
}

function getCoverSrc(book: EbookItem) {
  return book.cover_path ? convertFileSrc(book.cover_path) : ''
}

function formatRelativeTime(timestamp?: number) {
  if (!timestamp) return '尚未开始'

  const diff = Date.now() - timestamp
  const hour = 60 * 60 * 1000
  const day = 24 * hour

  if (diff < hour) return '刚刚读过'
  if (diff < day) return `${Math.max(1, Math.round(diff / hour))} 小时前`
  if (diff < day * 7) return `${Math.max(1, Math.round(diff / day))} 天前`
  return formatDate(timestamp)
}

onMounted(async () => {
  await loadBooks()
  await openReaderFromRouteQuery()
})

watch(() => route.fullPath, () => {
  if (route.path !== '/books') return
  void openReaderFromRouteQuery()
})
</script>

<template>
  <BookReader
    v-if="showReader && readerBook"
    :ebook="readerBook"
    :focus-cfi="readerFocusCfi"
    :highlight-id="readerHighlightId"
    :highlight-type="readerHighlightType"
    @close="closeReader"
    @updated="handleBookUpdated"
  />

  <section v-else class="books-page">
    <header class="shelf-header">
      <div>
        <p class="shelf-kicker">Library</p>
        <h1>书架</h1>
      </div>
      <button class="btn btn-primary" :disabled="importing" @click="handleImportBook">
        {{ importing ? '导入中...' : '导入 EPUB' }}
      </button>
    </header>

    <button v-if="currentBook" class="continue-strip" type="button" @click="openReader(currentBook)">
      <div class="continue-strip__main">
        <span>继续阅读</span>
        <strong>{{ currentBook.title }}</strong>
        <small>{{ currentBook.author || '未填写作者' }} · {{ formatRelativeTime(currentBook.last_read_at) }}</small>
      </div>
      <div class="continue-strip__progress">
        <div class="progress-track">
          <div class="progress-fill" :style="{ width: formatPercent(currentBook.progress) }"></div>
        </div>
        <span>{{ formatPercent(currentBook.progress) }}</span>
      </div>
    </button>

    <section class="shelf-toolbar">
      <div class="search-box">
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="11" cy="11" r="8"/>
          <line x1="21" y1="21" x2="16.65" y2="16.65"/>
        </svg>
        <input v-model="searchQuery" placeholder="搜索书名或作者" class="search-input" />
      </div>
      <span class="result-count">{{ filteredBooks.length }} / {{ ebooksCount }} 本</span>
    </section>

    <div v-if="loading" class="empty-state">
      <p>正在加载书架...</p>
    </div>

    <div v-else-if="!hasBooks" class="empty-state empty-state--action">
      <p>书架还是空的</p>
      <span>导入一本 EPUB，就可以直接开始阅读并保存词句。</span>
      <button class="btn btn-primary" :disabled="importing" @click="handleImportBook">
        {{ importing ? '导入中...' : '导入 EPUB' }}
      </button>
    </div>

    <div v-else-if="filteredBooks.length" class="book-grid">
      <article
        v-for="book in filteredBooks"
        :key="book.id"
        class="book-card"
        role="button"
        tabindex="0"
        @click="openReader(book)"
        @keydown.enter.prevent="openReader(book)"
        @keydown.space.prevent="openReader(book)"
      >
        <div class="book-cover" :class="{ 'book-cover--image': book.cover_path }">
          <img v-if="book.cover_path" :src="getCoverSrc(book)" :alt="`${book.title} 封面`" />
          <span v-else>{{ book.format.toUpperCase() }}</span>
        </div>
        <div class="book-main">
          <div class="book-top">
            <span class="book-status" :class="getBookStatusClass(book)">{{ getBookStatus(book) }}</span>
            <button class="delete-btn" title="删除图书" @click.stop="handleDelete(book)">×</button>
          </div>
          <h2>{{ book.title }}</h2>
          <p>{{ book.author || '未填写作者' }}</p>
          <div class="progress-row">
            <div class="progress-track">
              <div class="progress-fill" :style="{ width: formatPercent(book.progress) }"></div>
            </div>
            <span>{{ formatPercent(book.progress) }}</span>
          </div>
          <small>{{ formatRelativeTime(book.last_read_at) }} · 导入于 {{ formatDate(book.created_at) }}</small>
        </div>
      </article>
    </div>

    <div v-else class="empty-state">
      <p>没有匹配的图书</p>
      <span>换个关键词试试。</span>
    </div>

    <DeleteConfirmModal
      :visible="showDeleteConfirm"
      type="single"
      :message="deleteTarget ? `确定要删除《${deleteTarget.title}》吗？对应 EPUB 文件也会从本地书架移除。` : undefined"
      @confirm="confirmDelete"
      @cancel="cancelDelete"
    />
  </section>
</template>

<style scoped>
.books-page {
  width: min(100%, 1080px);
  margin: 0 auto;
  padding: 34px 32px 48px;
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.shelf-header,
.shelf-toolbar,
.continue-strip {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.shelf-kicker {
  margin: 0 0 5px;
  color: var(--c-text-lighter);
  font-size: 0.76rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.shelf-header h1 {
  margin: 0;
  color: var(--c-text);
  font-size: 2rem;
  letter-spacing: -0.04em;
}

.continue-strip,
.shelf-toolbar,
.empty-state,
.book-card {
  border: 1px solid var(--c-border);
  border-radius: 16px;
  background: var(--c-surface-1);
}

.continue-strip {
  width: 100%;
  padding: 15px 16px;
  color: inherit;
  text-align: left;
  cursor: pointer;
  transition: border-color 0.16s ease, background 0.16s ease;
}

.continue-strip:hover,
.continue-strip:focus-visible,
.book-card:hover,
.book-card:focus-visible {
  border-color: var(--c-border-strong);
  background: var(--c-hover-bg);
  outline: none;
}

.continue-strip__main {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.continue-strip__main span,
.continue-strip__main small,
.result-count,
.book-main p,
.book-main small {
  color: var(--c-text-lighter);
}

.continue-strip__main span {
  font-size: 0.78rem;
  font-weight: 700;
}

.continue-strip__main strong {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--c-text);
  font-size: 1rem;
}

.continue-strip__progress {
  min-width: 190px;
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--c-text-lighter);
  font-weight: 700;
}

.shelf-toolbar {
  padding: 10px 12px;
}

.search-box {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  max-width: 460px;
  color: var(--c-text-lighter);
}

.search-input {
  flex: 1;
  border: none;
  background: transparent;
  outline: none;
  color: var(--c-text);
  font-size: 0.95rem;
}

.book-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 14px;
}

.book-card {
  display: grid;
  grid-template-columns: 74px minmax(0, 1fr);
  gap: 14px;
  padding: 14px;
  cursor: pointer;
  transition: border-color 0.16s ease, background 0.16s ease;
}

.book-cover {
  display: flex;
  align-items: flex-end;
  justify-content: center;
  min-height: 104px;
  padding: 10px 8px;
  border-radius: 12px;
  background: linear-gradient(180deg, #edede7, #dcdcd4);
  color: var(--c-text-lighter);
  font-size: 0.72rem;
  font-weight: 800;
  letter-spacing: 0.08em;
  overflow: hidden;
}

.book-cover--image {
  align-items: stretch;
  padding: 0;
  background: var(--c-surface-2);
}

.book-cover img {
  width: 100%;
  height: 100%;
  min-height: 104px;
  object-fit: cover;
}

:global(:root.dark) .book-cover {
  background: linear-gradient(180deg, #243047, #1a2234);
}

.book-main {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.book-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.book-status {
  color: var(--c-text-lighter);
  font-size: 0.76rem;
  font-weight: 700;
}

.book-status.is-active {
  color: var(--c-primary);
}

.book-status.is-complete {
  color: #3b7f4a;
}

.book-main h2 {
  margin: 0;
  color: var(--c-text);
  font-size: 1rem;
  line-height: 1.35;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.book-main p,
.book-main small {
  margin: 0;
  line-height: 1.5;
}

.delete-btn {
  width: 26px;
  height: 26px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--c-text-lighter);
  font-size: 1rem;
  cursor: pointer;
  opacity: 0.35;
  transition: opacity 0.16s ease, background 0.16s ease, color 0.16s ease;
}

.book-card:hover .delete-btn,
.delete-btn:focus-visible {
  opacity: 1;
}

.delete-btn:hover {
  background: rgba(199, 53, 53, 0.08);
  color: var(--c-danger);
}

.progress-row {
  display: flex;
  align-items: center;
  gap: 9px;
  color: var(--c-text-lighter);
  font-size: 0.84rem;
  font-weight: 700;
}

.progress-track {
  flex: 1;
  height: 4px;
  border-radius: 999px;
  background: var(--c-track-bg);
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  border-radius: inherit;
  background: var(--c-primary);
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 42px 24px;
  text-align: center;
  color: var(--c-text-lighter);
}

.empty-state p {
  margin: 0;
  color: var(--c-text);
  font-weight: 700;
}

.empty-state--action .btn {
  margin-top: 10px;
}

@media (max-width: 900px) {
  .books-page {
    padding: 26px 18px 40px;
  }

  .book-grid,
  .book-card {
    grid-template-columns: 1fr;
  }

  .book-cover {
    min-height: 86px;
  }
}

@media (max-width: 680px) {
  .shelf-header,
  .shelf-toolbar,
  .continue-strip {
    align-items: flex-start;
    flex-direction: column;
  }

  .continue-strip__progress {
    width: 100%;
    min-width: 0;
  }
}
</style>
