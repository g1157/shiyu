<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { useRoute, useRouter } from 'vue-router'
import { deleteEbook, getEbooks, importEpubAsBook, type EbookItem } from '../services/api'
import BookReader from '../components/BookReader.vue'
import DeleteConfirmModal from '../components/DeleteConfirmModal.vue'
import { useGlobalToast } from '../composables/useGlobalToast'
import type { HighlightType } from '../composables/useRouteQuery'
import { formatDate } from '../utils/format'

const route = useRoute()
const router = useRouter()
const toast = useGlobalToast()

const books = ref<EbookItem[]>([])
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

const filteredBooks = computed(() => {
  const q = searchQuery.value.trim().toLowerCase()
  if (!q) return books.value
  return books.value.filter((book) =>
    book.title.toLowerCase().includes(q) ||
    (book.author?.toLowerCase().includes(q) ?? false)
  )
})

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

async function loadBooks() {
  loading.value = true
  try {
    books.value = await getEbooks()
  } catch (e: any) {
    toast.error('加载书架失败: ' + e)
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

  if (books.value.length === 0) {
    await loadBooks()
  }

  const target = books.value.find((item) => item.id === bookId)
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
    await loadBooks()
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
    books.value = books.value.filter((item) => item.id !== deleteTarget.value?.id)
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
  const index = books.value.findIndex((item) => item.id === updated.id)
  if (index >= 0) {
    books.value[index] = updated
  }
  readerBook.value = updated
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

  <section v-else class="page-container books-page">
    <header class="page-header">
      <div>
        <h1 class="page-title">图书书架</h1>
        <p class="page-subtitle">直接导入 EPUB 为图书，保留目录、进度与章节定位。</p>
      </div>
      <div class="header-actions">
        <button class="btn btn-primary" :disabled="importing" @click="handleImportBook">
          {{ importing ? '导入中...' : '导入 EPUB 图书' }}
        </button>
      </div>
    </header>

    <div class="toolbar glass-card">
      <div class="search-box">
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="11" cy="11" r="8"/>
          <line x1="21" y1="21" x2="16.65" y2="16.65"/>
        </svg>
        <input v-model="searchQuery" placeholder="搜索书名或作者..." class="search-input" />
      </div>
      <span class="result-count">{{ filteredBooks.length }} 本</span>
    </div>

    <div v-if="loading" class="empty-state glass-card">
      <p>正在加载书架...</p>
    </div>

    <div v-else-if="filteredBooks.length === 0" class="empty-state glass-card">
      <p>书架还是空的</p>
      <span>点击“导入 EPUB 图书”即可直接以图书方式阅读，不再拆成零散文章。</span>
    </div>

    <div v-else class="book-grid">
      <article
        v-for="book in filteredBooks"
        :key="book.id"
        class="book-card glass-card"
        @click="openReader(book)"
      >
        <div class="book-cover">
          <div class="cover-icon">EPUB</div>
        </div>
        <div class="book-body">
          <div class="book-top">
            <div>
              <h3 class="book-title">{{ book.title }}</h3>
              <p class="book-author">{{ book.author || '未知作者' }}</p>
            </div>
            <button class="delete-btn" title="删除图书" @click.stop="handleDelete(book)">×</button>
          </div>

          <div class="progress-row">
            <div class="progress-track">
              <div class="progress-fill" :style="{ width: `${Math.max(0, Math.min(100, book.progress * 100))}%` }"></div>
            </div>
            <span>{{ Math.round(book.progress * 100) }}%</span>
          </div>

          <div class="book-meta">
            <span class="meta-tag">{{ book.format.toUpperCase() }}</span>
            <span class="meta-tag">导入于 {{ formatDate(book.created_at) }}</span>
            <span class="meta-tag" v-if="book.last_read_at">最近阅读 {{ formatDate(book.last_read_at) }}</span>
          </div>
        </div>
      </article>
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
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.page-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  flex-wrap: wrap;
}

.page-title {
  margin: 0;
  font-size: 1.6rem;
}

.page-subtitle {
  margin: 6px 0 0;
  color: var(--c-text-lighter);
}

.glass-card {
  background: rgba(255, 255, 255, 0.72);
  border: 1px solid rgba(255, 255, 255, 0.55);
  box-shadow: 0 18px 40px rgba(15, 23, 42, 0.08);
  backdrop-filter: blur(14px);
  -webkit-backdrop-filter: blur(14px);
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 14px 16px;
  border-radius: 16px;
}

.search-box {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  max-width: 420px;
  padding: 10px 12px;
  border: 1px solid var(--c-border);
  border-radius: 12px;
  background: var(--c-bg-light);
}

.search-box svg,
.result-count {
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
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 16px;
}

.book-card {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 16px;
  border-radius: 18px;
  cursor: pointer;
  transition: transform 0.18s ease, box-shadow 0.18s ease;
}

.book-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 22px 48px rgba(37, 99, 235, 0.12);
}

.book-cover {
  height: 140px;
  border-radius: 14px;
  background: linear-gradient(135deg, #2563eb, #7c3aed);
  display: flex;
  align-items: center;
  justify-content: center;
}

.cover-icon {
  font-size: 1.15rem;
  font-weight: 800;
  letter-spacing: 1px;
  color: #fff;
}

.book-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.book-top {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.book-title {
  margin: 0;
  font-size: 1.05rem;
  color: var(--c-text);
}

.book-author {
  margin: 6px 0 0;
  color: var(--c-text-lighter);
  font-size: 0.9rem;
}

.delete-btn {
  width: 30px;
  height: 30px;
  border: none;
  border-radius: 50%;
  background: rgba(239, 68, 68, 0.1);
  color: #dc2626;
  font-size: 1.15rem;
  cursor: pointer;
}

.progress-row {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 0.85rem;
  color: var(--c-text-lighter);
}

.progress-track {
  flex: 1;
  height: 8px;
  border-radius: 999px;
  background: rgba(148, 163, 184, 0.18);
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(135deg, #2563eb, #7c3aed);
}

.book-meta {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.meta-tag {
  padding: 4px 8px;
  border-radius: 999px;
  background: rgba(148, 163, 184, 0.12);
  color: var(--c-text-lighter);
  font-size: 0.78rem;
}

.empty-state {
  padding: 40px 20px;
  border-radius: 18px;
  text-align: center;
  color: var(--c-text-lighter);
}

.empty-state p {
  margin: 0 0 8px;
  font-size: 1rem;
  color: var(--c-text);
}
</style>
