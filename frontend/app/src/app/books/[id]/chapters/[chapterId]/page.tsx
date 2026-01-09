'use client'

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowLeft, Check, Loader2, Save } from 'lucide-react'
import Link from 'next/link'
import { useRouter } from 'next/navigation'
import { useCallback, useEffect, useState } from 'react'
import { useDebouncedCallback } from 'use-debounce'
import { useAuth } from '../../../../hooks/useAuth'

interface Chapter {
  id: string
  book_id: string
  chapter_number: number
  title: string | null
  content: string | null
  word_count: number
  created_at: string
  updated_at: string
}

interface Book {
  id: string
  title: string
}

export default function ChapterEditorPage({
  params,
}: {
  params: { id: string; chapterId: string }
}) {
  const router = useRouter()
  const queryClient = useQueryClient()
  const { isAuthenticated, isLoading: authLoading, accessToken } = useAuth()
  const [content, setContent] = useState('')
  const [title, setTitle] = useState('')
  const [saveStatus, setSaveStatus] = useState<'saved' | 'saving' | 'unsaved'>('saved')

  // Redirect if not authenticated
  useEffect(() => {
    if (!authLoading && !isAuthenticated) {
      router.push('/')
    }
  }, [authLoading, isAuthenticated, router])

  // Fetch book info
  const { data: book } = useQuery<Book>({
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

  // Fetch chapter
  const { data: chapter, isLoading: chapterLoading } = useQuery<Chapter>({
    queryKey: ['chapter', params.chapterId],
    queryFn: async () => {
      const response = await fetch(
        `/api/books/${params.id}/chapters/${params.chapterId}`,
        { headers: { Authorization: `Bearer ${accessToken}` } }
      )
      if (!response.ok) throw new Error('Failed to fetch chapter')
      return response.json()
    },
    enabled: isAuthenticated && !!accessToken,
  })

  // Set initial content when chapter loads
  useEffect(() => {
    if (chapter) {
      setContent(chapter.content || '')
      setTitle(chapter.title || '')
    }
  }, [chapter])

  // Save mutation
  const saveMutation = useMutation({
    mutationFn: async (data: { title?: string; content?: string }) => {
      const response = await fetch(
        `/api/books/${params.id}/chapters/${params.chapterId}`,
        {
          method: 'PUT',
          headers: {
            'Content-Type': 'application/json',
            Authorization: `Bearer ${accessToken}`,
          },
          body: JSON.stringify(data),
        }
      )
      if (!response.ok) throw new Error('Failed to save')
      return response.json()
    },
    onSuccess: () => {
      setSaveStatus('saved')
      queryClient.invalidateQueries({ queryKey: ['chapter', params.chapterId] })
      queryClient.invalidateQueries({ queryKey: ['chapters', params.id] })
    },
    onError: () => {
      setSaveStatus('unsaved')
    },
  })

  // Debounced auto-save
  const debouncedSave = useDebouncedCallback(
    useCallback((newContent: string, newTitle: string) => {
      setSaveStatus('saving')
      saveMutation.mutate({ content: newContent, title: newTitle || null })
    }, [saveMutation]),
    1500
  )

  const handleContentChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newContent = e.target.value
    setContent(newContent)
    setSaveStatus('unsaved')
    debouncedSave(newContent, title)
  }

  const handleTitleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newTitle = e.target.value
    setTitle(newTitle)
    setSaveStatus('unsaved')
    debouncedSave(content, newTitle)
  }

  const wordCount = content.trim() ? content.trim().split(/\s+/).length : 0

  if (authLoading || chapterLoading) {
    return (
      <div className="h-screen flex items-center justify-center">
        <Loader2 className="h-8 w-8 animate-spin text-indigo-500" />
      </div>
    )
  }

  return (
    <div className="h-screen flex flex-col bg-slate-950">
      {/* Header */}
      <header className="flex items-center justify-between px-4 py-3 border-b border-slate-800 bg-slate-900/50">
        <div className="flex items-center gap-4">
          <Link
            href={`/books/${params.id}`}
            className="flex items-center gap-2 text-slate-400 hover:text-white"
          >
            <ArrowLeft className="h-4 w-4" />
            <span className="hidden sm:inline">{book?.title || 'Back'}</span>
          </Link>
          <span className="text-slate-600">|</span>
          <span className="text-slate-400">
            Chapter {chapter?.chapter_number}
          </span>
        </div>

        <div className="flex items-center gap-4">
          <span className="text-sm text-slate-500">
            {wordCount.toLocaleString()} words
          </span>
          <div className="flex items-center gap-2 text-sm">
            {saveStatus === 'saving' && (
              <>
                <Loader2 className="h-4 w-4 animate-spin text-slate-400" />
                <span className="text-slate-400">Saving...</span>
              </>
            )}
            {saveStatus === 'saved' && (
              <>
                <Check className="h-4 w-4 text-green-500" />
                <span className="text-green-500">Saved</span>
              </>
            )}
            {saveStatus === 'unsaved' && (
              <>
                <Save className="h-4 w-4 text-amber-500" />
                <span className="text-amber-500">Unsaved</span>
              </>
            )}
          </div>
        </div>
      </header>

      {/* Editor */}
      <div className="flex-1 overflow-auto">
        <div className="max-w-3xl mx-auto px-4 py-8">
          <input
            type="text"
            value={title}
            onChange={handleTitleChange}
            placeholder={`Chapter ${chapter?.chapter_number} Title`}
            className="w-full text-3xl font-playfair font-bold bg-transparent border-none outline-none placeholder-slate-600 mb-8"
          />
          <textarea
            value={content}
            onChange={handleContentChange}
            placeholder="Start writing your chapter..."
            className="w-full min-h-[70vh] text-lg leading-relaxed bg-transparent border-none outline-none resize-none placeholder-slate-600 focus:ring-0"
            autoFocus
          />
        </div>
      </div>
    </div>
  )
}
