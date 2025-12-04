'use client'

import { useEffect, useState, useCallback, useRef } from 'react'
import { useParams, useRouter } from 'next/navigation'
import Link from 'next/link'
import { ArrowLeft, Save, Sparkles, Undo, Redo, Bold, Italic, List, Heading1, Heading2, Loader2, CheckCircle } from 'lucide-react'
import { useAuth } from '../../hooks/useAuth'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useDebouncedCallback } from 'use-debounce'

export default function EditorPage() {
  const params = useParams()
  const router = useRouter()
  const queryClient = useQueryClient()
  const { isAuthenticated, isLoading: authLoading, accessToken } = useAuth()
  const chapterId = params.id as string
  
  const [content, setContent] = useState('')
  const [title, setTitle] = useState('')
  const [isSaving, setIsSaving] = useState(false)
  const [lastSaved, setLastSaved] = useState<Date | null>(null)
  const [wordCount, setWordCount] = useState(0)
  const textareaRef = useRef<HTMLTextAreaElement>(null)

  useEffect(() => {
    if (!authLoading && !isAuthenticated) {
      router.push('/')
    }
  }, [authLoading, isAuthenticated, router])

  // Fetch chapter data
  const { data: chapter, isLoading: chapterLoading } = useQuery({
    queryKey: ['chapter', chapterId],
    queryFn: async () => {
      const response = await fetch(`/api/chapters/${chapterId}`, {
        headers: { Authorization: `Bearer ${accessToken}` },
      })
      if (!response.ok) throw new Error('Failed to fetch chapter')
      return response.json()
    },
    enabled: isAuthenticated && !!chapterId,
  })

  // Set initial content
  useEffect(() => {
    if (chapter) {
      setContent(chapter.content || '')
      setTitle(chapter.title || '')
      setWordCount(chapter.word_count || 0)
    }
  }, [chapter])

  // Save mutation
  const saveMutation = useMutation({
    mutationFn: async (data: { title: string; content: string }) => {
      const response = await fetch(`/api/chapters/${chapterId}`, {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${accessToken}`,
        },
        body: JSON.stringify(data),
      })
      if (!response.ok) throw new Error('Failed to save')
      return response.json()
    },
    onSuccess: () => {
      setLastSaved(new Date())
      queryClient.invalidateQueries({ queryKey: ['chapter', chapterId] })
    },
  })

  // Auto-save with debounce
  const debouncedSave = useDebouncedCallback((newContent: string) => {
    setIsSaving(true)
    saveMutation.mutate({ title, content: newContent }, {
      onSettled: () => setIsSaving(false)
    })
  }, 2000)

  // Handle content change
  const handleContentChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newContent = e.target.value
    setContent(newContent)
    setWordCount(newContent.split(/\s+/).filter(Boolean).length)
    debouncedSave(newContent)
  }

  // Manual save
  const handleSave = () => {
    setIsSaving(true)
    saveMutation.mutate({ title, content }, {
      onSettled: () => setIsSaving(false)
    })
  }

  // AI enhancement
  const enhanceMutation = useMutation({
    mutationFn: async (type: string) => {
      const response = await fetch('/api/generate/enhance', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${accessToken}`,
        },
        body: JSON.stringify({
          chapter_id: chapterId,
          content,
          enhancement_type: type,
        }),
      })
      if (!response.ok) throw new Error('Failed to enhance')
      return response.json()
    },
  })

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault()
        handleSave()
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [content, title])

  if (authLoading || chapterLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-indigo-500"></div>
      </div>
    )
  }

  if (!isAuthenticated || !chapter) return null

  return (
    <div className="min-h-screen bg-slate-950 flex flex-col">
      {/* Toolbar */}
      <div className="sticky top-16 z-40 bg-slate-900/95 backdrop-blur-xl border-b border-slate-800">
        <div className="max-w-5xl mx-auto px-4 py-3 flex items-center justify-between">
          <div className="flex items-center gap-4">
            <Link
              href={`/books/${chapter.book_id}`}
              className="p-2 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800"
            >
              <ArrowLeft className="h-5 w-5" />
            </Link>
            
            <input
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              onBlur={handleSave}
              className="text-lg font-semibold bg-transparent border-none focus:outline-none focus:ring-0 text-white"
              placeholder="Chapter Title"
            />
          </div>

          <div className="flex items-center gap-2">
            {/* Status */}
            <div className="flex items-center gap-2 text-sm text-slate-500 mr-4">
              {isSaving ? (
                <>
                  <Loader2 className="h-4 w-4 animate-spin" />
                  <span>Saving...</span>
                </>
              ) : lastSaved ? (
                <>
                  <CheckCircle className="h-4 w-4 text-green-500" />
                  <span>Saved {lastSaved.toLocaleTimeString()}</span>
                </>
              ) : null}
            </div>

            {/* Word count */}
            <span className="text-sm text-slate-500 mr-4">
              {wordCount.toLocaleString()} words
            </span>

            {/* Formatting */}
            <div className="flex items-center gap-1 border-l border-slate-700 pl-4">
              <button className="p-2 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800">
                <Bold className="h-4 w-4" />
              </button>
              <button className="p-2 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800">
                <Italic className="h-4 w-4" />
              </button>
              <button className="p-2 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800">
                <Heading1 className="h-4 w-4" />
              </button>
              <button className="p-2 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800">
                <Heading2 className="h-4 w-4" />
              </button>
              <button className="p-2 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800">
                <List className="h-4 w-4" />
              </button>
            </div>

            {/* AI */}
            <div className="flex items-center gap-1 border-l border-slate-700 pl-4">
              <button
                onClick={() => enhanceMutation.mutate('style')}
                disabled={enhanceMutation.isPending}
                className="p-2 rounded-lg text-purple-400 hover:bg-purple-500/20 disabled:opacity-50"
                title="Enhance with AI"
              >
                <Sparkles className="h-5 w-5" />
              </button>
            </div>

            {/* Save */}
            <button
              onClick={handleSave}
              disabled={isSaving}
              className="btn-primary ml-4"
            >
              {isSaving ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Save className="h-4 w-4" />
              )}
              <span className="ml-2">Save</span>
            </button>
          </div>
        </div>
      </div>

      {/* Editor */}
      <div className="flex-1 max-w-4xl mx-auto w-full px-4 py-8">
        <textarea
          ref={textareaRef}
          value={content}
          onChange={handleContentChange}
          placeholder="Start writing your chapter..."
          className="w-full h-full min-h-[calc(100vh-200px)] bg-transparent text-slate-200 text-lg leading-relaxed resize-none focus:outline-none placeholder-slate-600 font-serif"
          style={{ fontFamily: 'Georgia, serif' }}
        />
      </div>

      {/* AI Panel (shown when enhancing) */}
      {enhanceMutation.isPending && (
        <div className="fixed bottom-4 right-4 bg-slate-800 border border-slate-700 rounded-xl p-4 shadow-xl flex items-center gap-3">
          <Loader2 className="h-5 w-5 animate-spin text-purple-400" />
          <span className="text-slate-300">AI is enhancing your content...</span>
        </div>
      )}
    </div>
  )
}

