'use client'

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  ArrowLeft,
  ChevronRight,
  Edit3,
  FileText,
  Loader2,
  Plus,
  RotateCcw,
  Settings
} from 'lucide-react'
import Link from 'next/link'
import { useRouter } from 'next/navigation'
import { useEffect, useState } from 'react'
import { useAuth } from '../../hooks/useAuth'

interface Book {
  id: string
  title: string
  description: string | null
  genre: string | null
  status: string
  word_count: number
  metadata: Record<string, any>
  created_at: string
  updated_at: string
}

interface Chapter {
  id: string
  book_id: string
  chapter_number: number
  title: string | null
  word_count: number
  created_at: string
  updated_at: string
}

export default function BookDetailPage({ params }: { params: { id: string } }) {
  const router = useRouter()
  const queryClient = useQueryClient()
  const { isAuthenticated, isLoading: authLoading, accessToken } = useAuth()
  const [showNewChapter, setShowNewChapter] = useState(false)
  const [newChapterTitle, setNewChapterTitle] = useState('')
  const [generationError, setGenerationError] = useState<string | null>(null)

  // Redirect if not authenticated
  useEffect(() => {
    if (!authLoading && !isAuthenticated) {
      router.push('/')
    }
  }, [authLoading, isAuthenticated, router])

  // Fetch book details
  const { data: book, isLoading: bookLoading } = useQuery<Book>({
    queryKey: ['book', params.id],
    queryFn: async () => {
      const response = await fetch(`/api/books/${params.id}`, {
        headers: { Authorization: `Bearer ${accessToken}` },
      })
      if (!response.ok) throw new Error('Failed to fetch book')
      return response.json()
    },
    enabled: isAuthenticated && !!accessToken,
  })

  // Fetch chapters
  const { data: chaptersData, isLoading: chaptersLoading } = useQuery<{ chapters: Chapter[] }>({
    queryKey: ['chapters', params.id],
    queryFn: async () => {
      const response = await fetch(`/api/books/${params.id}/chapters`, {
        headers: { Authorization: `Bearer ${accessToken}` },
      })
      if (!response.ok) return { chapters: [] }
      return response.json()
    },
    enabled: isAuthenticated && !!accessToken,
  })

  // Create chapter mutation
  const createChapterMutation = useMutation({
    mutationFn: async () => {
      const response = await fetch(`/api/books/${params.id}/chapters`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${accessToken}`,
        },
        body: JSON.stringify({ title: newChapterTitle || null }),
      })
      if (!response.ok) throw new Error('Failed to create chapter')
      return response.json()
    },
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['chapters', params.id] })
      setShowNewChapter(false)
      setNewChapterTitle('')
      router.push(`/books/${params.id}/chapters/${data.id}`)
    },
  })

  // Continue Writing - triggers book generation via core engine
  const continueWritingMutation = useMutation({
    mutationFn: async () => {
      setGenerationError(null)
      const response = await fetch('/api/generate/book', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${accessToken}`,
        },
        body: JSON.stringify({
          book_id: params.id,
          title: book?.title,
          description: book?.description || '',
          genre: book?.genre || '',
          braindump: book?.metadata?.braindump || '',
          characters: book?.metadata?.characters || '',
          synopsis: book?.metadata?.synopsis || '',
          chapter_count: 12,
        }),
      })
      if (!response.ok) {
        const error = await response.json()
        throw new Error(error.error || 'Failed to start generation')
      }
      return response.json()
    },
    onSuccess: (data) => {
      console.log('Generation started:', data)
      // Refresh chapters after a delay to see new content
      setTimeout(() => {
        queryClient.invalidateQueries({ queryKey: ['chapters', params.id] })
        queryClient.invalidateQueries({ queryKey: ['book', params.id] })
      }, 2000)
    },
    onError: (error: Error) => {
      setGenerationError(error.message)
    },
  })

  // Reset - delete all chapters
  const resetMutation = useMutation({
    mutationFn: async () => {
      const currentChapters = chaptersData?.chapters || []
      // Delete all chapters sequentially
      for (const chapter of currentChapters) {
        const response = await fetch(`/api/chapters/${chapter.id}`, {
          method: 'DELETE',
          headers: { Authorization: `Bearer ${accessToken}` },
        })
        if (!response.ok) {
          throw new Error('Failed to delete chapter')
        }
      }
      return { deleted: currentChapters.length }
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['chapters', params.id] })
      queryClient.invalidateQueries({ queryKey: ['book', params.id] })
    },
  })

  const chapters = chaptersData?.chapters || []

  if (authLoading || bookLoading) {
    return (
      <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="animate-pulse space-y-4">
          <div className="h-8 bg-slate-700 rounded w-48"></div>
          <div className="h-12 bg-slate-700 rounded w-96"></div>
          <div className="h-4 bg-slate-700 rounded w-64"></div>
        </div>
      </div>
    )
  }

  if (!book) {
    return (
      <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8 text-center">
        <h1 className="text-2xl font-bold mb-4">Book not found</h1>
        <Link href="/books" className="btn-primary">
          Back to Books
        </Link>
      </div>
    )
  }

  return (
    <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      {/* Header */}
      <div className="mb-8">
        <Link href="/books" className="inline-flex items-center gap-2 text-slate-400 hover:text-white mb-4">
          <ArrowLeft className="h-4 w-4" />
          Back to Books
        </Link>

        <div className="flex items-start justify-between">
          <div>
            <h1 className="text-3xl font-playfair font-bold">{book.title}</h1>
            {book.description && (
              <p className="text-slate-400 mt-2">{book.description}</p>
            )}
            <div className="flex items-center gap-4 mt-3 text-sm text-slate-500">
              {book.genre && (
                <span className="px-2 py-1 bg-slate-800 rounded">{book.genre}</span>
              )}
              <span>{book.word_count.toLocaleString()} words</span>
              <span className="capitalize">{book.status}</span>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <button className="btn-ghost p-2" title="Settings">
              <Settings className="h-5 w-5" />
            </button>
          </div>
        </div>
      </div>

      {/* Chapters Section */}
      <div className="card mb-6">
        <div className="flex items-center justify-between mb-6">
          <h2 className="text-xl font-semibold flex items-center gap-2">
            <FileText className="h-5 w-5 text-indigo-400" />
            Chapters
          </h2>
          <button
            onClick={() => setShowNewChapter(true)}
            className="btn-primary py-2 px-4"
          >
            <Plus className="h-4 w-4 mr-2" />
            Add Chapter
          </button>
        </div>

        {showNewChapter && (
          <div className="mb-4 p-4 bg-slate-800/50 rounded-lg border border-slate-700">
            <input
              type="text"
              value={newChapterTitle}
              onChange={(e) => setNewChapterTitle(e.target.value)}
              placeholder="Chapter title (optional)"
              className="input mb-3"
              autoFocus
            />
            <div className="flex gap-2">
              <button
                onClick={() => createChapterMutation.mutate()}
                disabled={createChapterMutation.isPending}
                className="btn-primary py-2 px-4"
              >
                {createChapterMutation.isPending ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  'Create Chapter'
                )}
              </button>
              <button
                onClick={() => {
                  setShowNewChapter(false)
                  setNewChapterTitle('')
                }}
                className="btn-secondary py-2 px-4"
              >
                Cancel
              </button>
            </div>
          </div>
        )}

        {chaptersLoading ? (
          <div className="animate-pulse space-y-3">
            {[1, 2, 3].map((i) => (
              <div key={i} className="h-16 bg-slate-700 rounded"></div>
            ))}
          </div>
        ) : chapters.length === 0 ? (
          <div className="text-center py-12 text-slate-500">
            <FileText className="h-12 w-12 mx-auto mb-4 opacity-50" />
            <p className="mb-4">No chapters yet. Start writing!</p>
            <button
              onClick={() => setShowNewChapter(true)}
              className="btn-primary"
            >
              <Plus className="h-4 w-4 mr-2" />
              Create First Chapter
            </button>
          </div>
        ) : (
          <div className="space-y-2">
            {chapters.map((chapter) => (
              <Link
                key={chapter.id}
                href={`/books/${params.id}/chapters/${chapter.id}`}
                className="flex items-center justify-between p-4 bg-slate-800/30 rounded-lg hover:bg-slate-800/50 transition-colors group"
              >
                <div className="flex items-center gap-4">
                  <span className="w-8 h-8 flex items-center justify-center bg-slate-700 rounded-full text-sm font-medium">
                    {chapter.chapter_number}
                  </span>
                  <div>
                    <h3 className="font-medium">
                      {chapter.title || `Chapter ${chapter.chapter_number}`}
                    </h3>
                    <p className="text-sm text-slate-500">
                      {chapter.word_count.toLocaleString()} words
                    </p>
                  </div>
                </div>
                <ChevronRight className="h-5 w-5 text-slate-500 group-hover:text-white transition-colors" />
              </Link>
            ))}
          </div>
        )}
      </div>

      {/* Generation Error */}
      {generationError && (
        <div className="mb-6 p-4 bg-red-900/30 border border-red-700 rounded-lg text-red-300">
          {generationError}
        </div>
      )}

      {/* Quick Actions */}
      <div className="grid grid-cols-2 gap-4">
        <button
          onClick={() => continueWritingMutation.mutate()}
          disabled={continueWritingMutation.isPending}
          className="card flex items-center gap-3 hover:bg-slate-800/70 transition-colors disabled:opacity-50"
        >
          {continueWritingMutation.isPending ? (
            <Loader2 className="h-8 w-8 text-emerald-400 animate-spin" />
          ) : (
            <Edit3 className="h-8 w-8 text-emerald-400" />
          )}
          <div className="text-left">
            <h3 className="font-medium">
              {continueWritingMutation.isPending ? 'Generating...' : 'Continue Writing'}
            </h3>
            <p className="text-sm text-slate-500">
              {continueWritingMutation.isPending
                ? 'AI is creating content'
                : 'Generate content with AI'}
            </p>
          </div>
        </button>
        <button
          onClick={() => {
            if (confirm('Are you sure you want to reset? This will delete all chapters.')) {
              resetMutation.mutate()
            }
          }}
          disabled={resetMutation.isPending || chapters.length === 0}
          className="card flex items-center gap-3 hover:bg-slate-800/70 transition-colors disabled:opacity-50"
        >
          {resetMutation.isPending ? (
            <Loader2 className="h-8 w-8 text-red-400 animate-spin" />
          ) : (
            <RotateCcw className="h-8 w-8 text-red-400" />
          )}
          <div className="text-left">
            <h3 className="font-medium">
              {resetMutation.isPending ? 'Resetting...' : 'Reset'}
            </h3>
            <p className="text-sm text-slate-500">Delete all chapters and start over</p>
          </div>
        </button>
      </div>
    </div>
  )
}
