'use client'

import { useEffect, useState } from 'react'
import { useRouter } from 'next/navigation'
import Link from 'next/link'
import { BookOpen, Plus, Search, Filter, Grid, List, MoreVertical, Trash2, Edit } from 'lucide-react'
import { useAuth } from '../hooks/useAuth'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'

interface Book {
  id: string
  title: string
  description: string | null
  genre: string | null
  status: string
  cover_image_url: string | null
  word_count: number
  created_at: string
  updated_at: string
}

export default function BooksPage() {
  const router = useRouter()
  const queryClient = useQueryClient()
  const { isAuthenticated, isLoading, accessToken } = useAuth()
  const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid')
  const [searchQuery, setSearchQuery] = useState('')
  const [statusFilter, setStatusFilter] = useState<string>('all')
  const [menuOpenId, setMenuOpenId] = useState<string | null>(null)

  useEffect(() => {
    if (!isLoading && !isAuthenticated) {
      router.push('/')
    }
  }, [isLoading, isAuthenticated, router])

  const { data, isLoading: booksLoading } = useQuery({
    queryKey: ['books', statusFilter],
    queryFn: async () => {
      const params = new URLSearchParams()
      if (statusFilter !== 'all') params.set('status', statusFilter)
      
      const response = await fetch(`/api/books?${params}`, {
        headers: { Authorization: `Bearer ${accessToken}` },
      })
      if (!response.ok) throw new Error('Failed to fetch books')
      return response.json()
    },
    enabled: isAuthenticated,
  })

  const deleteMutation = useMutation({
    mutationFn: async (bookId: string) => {
      const response = await fetch(`/api/books/${bookId}`, {
        method: 'DELETE',
        headers: { Authorization: `Bearer ${accessToken}` },
      })
      if (!response.ok) throw new Error('Failed to delete book')
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['books'] })
    },
  })

  const filteredBooks = data?.books?.filter((book: Book) =>
    book.title.toLowerCase().includes(searchQuery.toLowerCase())
  ) || []

  if (isLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-indigo-500"></div>
      </div>
    )
  }

  if (!isAuthenticated) return null

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 mb-8">
        <div>
          <h1 className="text-3xl font-playfair font-bold">My Books</h1>
          <p className="text-slate-400 mt-1">Manage and organize your book projects</p>
        </div>
        <Link href="/books/new" className="btn-primary shrink-0">
          <Plus className="h-5 w-5 mr-2" />
          New Book
        </Link>
      </div>

      {/* Filters */}
      <div className="flex flex-col sm:flex-row gap-4 mb-6">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-5 w-5 text-slate-500" />
          <input
            type="text"
            placeholder="Search books..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="input pl-10"
          />
        </div>
        <div className="flex gap-2">
          <select
            value={statusFilter}
            onChange={(e) => setStatusFilter(e.target.value)}
            className="input w-auto"
          >
            <option value="all">All Status</option>
            <option value="draft">Draft</option>
            <option value="writing">Writing</option>
            <option value="editing">Editing</option>
            <option value="published">Published</option>
          </select>
          <div className="flex rounded-lg border border-slate-700 overflow-hidden">
            <button
              onClick={() => setViewMode('grid')}
              className={`p-2 ${viewMode === 'grid' ? 'bg-slate-700 text-white' : 'text-slate-400 hover:text-white'}`}
            >
              <Grid className="h-5 w-5" />
            </button>
            <button
              onClick={() => setViewMode('list')}
              className={`p-2 ${viewMode === 'list' ? 'bg-slate-700 text-white' : 'text-slate-400 hover:text-white'}`}
            >
              <List className="h-5 w-5" />
            </button>
          </div>
        </div>
      </div>

      {/* Books Grid/List */}
      {booksLoading ? (
        <div className="flex items-center justify-center py-20">
          <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-indigo-500"></div>
        </div>
      ) : filteredBooks.length === 0 ? (
        <div className="card text-center py-16">
          <BookOpen className="h-16 w-16 text-slate-600 mx-auto mb-4" />
          <h3 className="text-xl font-medium text-slate-300 mb-2">No books found</h3>
          <p className="text-slate-500 mb-6">
            {searchQuery ? 'Try a different search term' : 'Create your first book to get started'}
          </p>
          {!searchQuery && (
            <Link href="/books/new" className="btn-primary">
              <Plus className="h-4 w-4 mr-2" />
              Create Book
            </Link>
          )}
        </div>
      ) : viewMode === 'grid' ? (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
          {filteredBooks.map((book: Book) => (
            <div key={book.id} className="card group hover:border-indigo-500/50 transition-all relative">
              <Link href={`/books/${book.id}`} className="block">
                <div className="aspect-[3/4] rounded-lg bg-gradient-to-br from-indigo-500/20 to-purple-500/20 mb-4 flex items-center justify-center overflow-hidden">
                  {book.cover_image_url ? (
                    <img src={book.cover_image_url} alt={book.title} className="h-full w-full object-cover" />
                  ) : (
                    <BookOpen className="h-12 w-12 text-indigo-400" />
                  )}
                </div>
                <h3 className="font-semibold text-white group-hover:text-indigo-400 transition-colors truncate">
                  {book.title}
                </h3>
                <p className="text-slate-400 text-sm mt-1 line-clamp-2 h-10">
                  {book.description || 'No description'}
                </p>
                <div className="flex items-center justify-between mt-3 text-xs text-slate-500">
                  <span>{book.word_count?.toLocaleString() || 0} words</span>
                  <span className={`capitalize px-2 py-0.5 rounded-full ${
                    book.status === 'published' ? 'bg-green-500/20 text-green-400' :
                    book.status === 'editing' ? 'bg-yellow-500/20 text-yellow-400' :
                    'bg-slate-700 text-slate-400'
                  }`}>
                    {book.status}
                  </span>
                </div>
              </Link>
              
              {/* Menu */}
              <div className="absolute top-4 right-4">
                <button
                  onClick={(e) => {
                    e.preventDefault()
                    setMenuOpenId(menuOpenId === book.id ? null : book.id)
                  }}
                  className="p-1 rounded hover:bg-slate-700 text-slate-400 hover:text-white"
                >
                  <MoreVertical className="h-5 w-5" />
                </button>
                {menuOpenId === book.id && (
                  <div className="absolute right-0 mt-1 w-40 bg-slate-800 border border-slate-700 rounded-lg shadow-xl py-1 z-10">
                    <Link
                      href={`/books/${book.id}/edit`}
                      className="flex items-center gap-2 px-4 py-2 text-sm text-slate-300 hover:bg-slate-700"
                      onClick={() => setMenuOpenId(null)}
                    >
                      <Edit className="h-4 w-4" />
                      Edit Details
                    </Link>
                    <button
                      onClick={() => {
                        if (confirm('Are you sure you want to delete this book?')) {
                          deleteMutation.mutate(book.id)
                        }
                        setMenuOpenId(null)
                      }}
                      className="flex items-center gap-2 px-4 py-2 text-sm text-red-400 hover:bg-slate-700 w-full"
                    >
                      <Trash2 className="h-4 w-4" />
                      Delete
                    </button>
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="space-y-3">
          {filteredBooks.map((book: Book) => (
            <Link key={book.id} href={`/books/${book.id}`}>
              <div className="card hover:border-indigo-500/50 transition-all flex items-center gap-4">
                <div className="h-20 w-14 rounded-lg bg-gradient-to-br from-indigo-500/20 to-purple-500/20 flex items-center justify-center shrink-0 overflow-hidden">
                  {book.cover_image_url ? (
                    <img src={book.cover_image_url} alt={book.title} className="h-full w-full object-cover" />
                  ) : (
                    <BookOpen className="h-6 w-6 text-indigo-400" />
                  )}
                </div>
                <div className="flex-1 min-w-0">
                  <h3 className="font-semibold text-white truncate">{book.title}</h3>
                  <p className="text-slate-400 text-sm truncate">{book.description || 'No description'}</p>
                </div>
                <div className="text-right shrink-0">
                  <div className="text-sm text-slate-400">{book.word_count?.toLocaleString() || 0} words</div>
                  <div className={`text-xs capitalize px-2 py-0.5 rounded-full inline-block mt-1 ${
                    book.status === 'published' ? 'bg-green-500/20 text-green-400' :
                    book.status === 'editing' ? 'bg-yellow-500/20 text-yellow-400' :
                    'bg-slate-700 text-slate-400'
                  }`}>
                    {book.status}
                  </div>
                </div>
              </div>
            </Link>
          ))}
        </div>
      )}
    </div>
  )
}

