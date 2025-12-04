'use client'

import { useEffect, useState } from 'react'
import { useParams, useRouter } from 'next/navigation'
import Link from 'next/link'
import { ArrowLeft, BookOpen, Plus, Edit, Play, Sparkles, MoreVertical, Trash2, Clock, FileText } from 'lucide-react'
import { useAuth } from '../../hooks/useAuth'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'

interface Chapter {
  id: string
  title: string
  chapter_number: number
  word_count: number
  status: string
  created_at: string
  updated_at: string
}

interface Book {
  id: string
  title: string
  description: string | null
  genre: string | null
  status: string
  cover_image_url: string | null
  word_count: number
  metadata: Record<string, any>
  created_at: string
  updated_at: string
}

export default function BookDetailPage() {
  const params = useParams()
  const router = useRouter()
  const queryClient = useQueryClient()
  const { isAuthenticated, isLoading: authLoading, accessToken } = useAuth()
  const bookId = params.id as string
  const [menuOpenId, setMenuOpenId] = useState<string | null>(null)

  useEffect(() => {
    if (!authLoading && !isAuthenticated) {
      router.push('/')
    }
  }, [authLoading, isAuthenticated, router])

  const { data: book, isLoading: bookLoading } = useQuery({
    queryKey: ['book', bookId],
    queryFn: async () => {
      const response = await fetch(`/api/books/${bookId}`, {
        headers: { Authorization: `Bearer ${accessToken}` },
      })
      if (!response.ok) throw new Error('Failed to fetch book')
      return response.json()
    },
    enabled: isAuthenticated && !!bookId,
  })

  const { data: chaptersData, isLoading: chaptersLoading } = useQuery({
    queryKey: ['chapters', bookId],
    queryFn: async () => {
      const response = await fetch(`/api/books/${bookId}/chapters`, {
        headers: { Authorization: `Bearer ${accessToken}` },
      })
      if (!response.ok) throw new Error('Failed to fetch chapters')
      return response.json()
    },
    enabled: isAuthenticated && !!bookId,
  })

  const deleteChapterMutation = useMutation({
    mutationFn: async (chapterId: string) => {
      const response = await fetch(`/api/chapters/${chapterId}`, {
        method: 'DELETE',
        headers: { Authorization: `Bearer ${accessToken}` },
      })
      if (!response.ok) throw new Error('Failed to delete chapter')
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['chapters', bookId] })
      queryClient.invalidateQueries({ queryKey: ['book', bookId] })
    },
  })

  const generateChapterMutation = useMutation({
    mutationFn: async (chapterId: string) => {
      const response = await fetch('/api/generate/chapter', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${accessToken}`,
        },
        body: JSON.stringify({ chapter_id: chapterId }),
      })
      if (!response.ok) throw new Error('Failed to queue generation')
      return response.json()
    },
  })

  if (authLoading || bookLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-indigo-500"></div>
      </div>
    )
  }

  if (!isAuthenticated || !book) return null

  const chapters: Chapter[] = chaptersData?.chapters || []

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      {/* Header */}
      <div className="mb-8">
        <Link href="/books" className="inline-flex items-center gap-2 text-slate-400 hover:text-white mb-4">
          <ArrowLeft className="h-4 w-4" />
          Back to Books
        </Link>
        
        <div className="flex flex-col lg:flex-row gap-6">
          {/* Cover */}
          <div className="w-48 h-64 rounded-xl bg-gradient-to-br from-indigo-500/20 to-purple-500/20 flex items-center justify-center shrink-0 overflow-hidden">
            {book.cover_image_url ? (
              <img src={book.cover_image_url} alt={book.title} className="h-full w-full object-cover" />
            ) : (
              <BookOpen className="h-16 w-16 text-indigo-400" />
            )}
          </div>

          {/* Info */}
          <div className="flex-1">
            <div className="flex items-start justify-between gap-4">
              <div>
                <h1 className="text-3xl font-playfair font-bold">{book.title}</h1>
                <p className="text-slate-400 mt-2">{book.description || 'No description'}</p>
              </div>
              <Link href={`/books/${bookId}/edit`} className="btn-secondary shrink-0">
                <Edit className="h-4 w-4 mr-2" />
                Edit
              </Link>
            </div>

            <div className="flex flex-wrap gap-4 mt-6 text-sm">
              <div className="flex items-center gap-2 text-slate-400">
                <FileText className="h-4 w-4" />
                {book.word_count?.toLocaleString() || 0} words
              </div>
              <div className="flex items-center gap-2 text-slate-400">
                <BookOpen className="h-4 w-4" />
                {chapters.length} chapters
              </div>
              {book.genre && (
                <span className="px-3 py-1 rounded-full bg-indigo-500/20 text-indigo-400">
                  {book.genre}
                </span>
              )}
              <span className={`px-3 py-1 rounded-full capitalize ${
                book.status === 'published' ? 'bg-green-500/20 text-green-400' :
                book.status === 'editing' ? 'bg-yellow-500/20 text-yellow-400' :
                'bg-slate-700 text-slate-400'
              }`}>
                {book.status}
              </span>
            </div>

            {/* Quick Actions */}
            <div className="flex gap-3 mt-6">
              {chapters.length > 0 && (
                <Link
                  href={`/editor/${chapters[0].id}`}
                  className="btn-primary"
                >
                  <Play className="h-4 w-4 mr-2" />
                  Continue Writing
                </Link>
              )}
              <Link
                href={`/books/${bookId}/chapters/new`}
                className="btn-secondary"
              >
                <Plus className="h-4 w-4 mr-2" />
                Add Chapter
              </Link>
            </div>
          </div>
        </div>
      </div>

      {/* Chapters */}
      <div className="mt-12">
        <div className="flex items-center justify-between mb-6">
          <h2 className="text-2xl font-semibold">Chapters</h2>
          <Link
            href={`/books/${bookId}/chapters/new`}
            className="btn-ghost text-indigo-400"
          >
            <Plus className="h-4 w-4 mr-2" />
            Add Chapter
          </Link>
        </div>

        {chaptersLoading ? (
          <div className="flex items-center justify-center py-12">
            <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-indigo-500"></div>
          </div>
        ) : chapters.length === 0 ? (
          <div className="card text-center py-12">
            <BookOpen className="h-12 w-12 text-slate-600 mx-auto mb-4" />
            <h3 className="text-lg font-medium text-slate-300 mb-2">No chapters yet</h3>
            <p className="text-slate-500 mb-4">Add your first chapter to start writing</p>
            <Link href={`/books/${bookId}/chapters/new`} className="btn-primary">
              <Plus className="h-4 w-4 mr-2" />
              Add Chapter
            </Link>
          </div>
        ) : (
          <div className="space-y-3">
            {chapters.map((chapter) => (
              <div key={chapter.id} className="card hover:border-indigo-500/50 transition-all flex items-center gap-4 relative group">
                <div className="h-12 w-12 rounded-lg bg-slate-800 flex items-center justify-center shrink-0">
                  <span className="text-lg font-semibold text-slate-400">{chapter.chapter_number}</span>
                </div>
                
                <Link href={`/editor/${chapter.id}`} className="flex-1 min-w-0">
                  <h3 className="font-semibold text-white group-hover:text-indigo-400 transition-colors truncate">
                    {chapter.title}
                  </h3>
                  <div className="flex items-center gap-4 mt-1 text-sm text-slate-500">
                    <span>{chapter.word_count?.toLocaleString() || 0} words</span>
                    <span className="flex items-center gap-1">
                      <Clock className="h-3 w-3" />
                      {new Date(chapter.updated_at).toLocaleDateString()}
                    </span>
                  </div>
                </Link>

                <div className="flex items-center gap-2 shrink-0">
                  <button
                    onClick={() => generateChapterMutation.mutate(chapter.id)}
                    disabled={generateChapterMutation.isPending}
                    className="p-2 rounded-lg text-purple-400 hover:bg-purple-500/20 transition-all"
                    title="Generate content with AI"
                  >
                    <Sparkles className="h-5 w-5" />
                  </button>
                  
                  <div className="relative">
                    <button
                      onClick={() => setMenuOpenId(menuOpenId === chapter.id ? null : chapter.id)}
                      className="p-2 rounded-lg text-slate-400 hover:bg-slate-700 transition-all"
                    >
                      <MoreVertical className="h-5 w-5" />
                    </button>
                    {menuOpenId === chapter.id && (
                      <div className="absolute right-0 mt-1 w-40 bg-slate-800 border border-slate-700 rounded-lg shadow-xl py-1 z-10">
                        <Link
                          href={`/editor/${chapter.id}`}
                          className="flex items-center gap-2 px-4 py-2 text-sm text-slate-300 hover:bg-slate-700"
                          onClick={() => setMenuOpenId(null)}
                        >
                          <Edit className="h-4 w-4" />
                          Edit
                        </Link>
                        <button
                          onClick={() => {
                            if (confirm('Delete this chapter?')) {
                              deleteChapterMutation.mutate(chapter.id)
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
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

