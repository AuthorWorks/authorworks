'use client'

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowLeft, BookOpen, CheckCircle, Loader2, Save } from 'lucide-react'
import dynamic from 'next/dynamic'
import Link from 'next/link'
import { useParams, useRouter } from 'next/navigation'
import { useCallback, useEffect, useState } from 'react'
import type { Descendant } from 'slate'
import { useDebouncedCallback } from 'use-debounce'
import { useAuth } from '../../hooks/useAuth'
import {
  deserializeFromMarkdown,
  serializeToMarkdown,
} from '../../components/editor/PlateEditor'

const PlateEditor = dynamic(
  () => import('../../components/editor/PlateEditor').then((mod) => mod.PlateEditor),
  {
    ssr: false,
    loading: () => (
      <div className="flex-1 flex items-center justify-center">
        <Loader2 className="h-8 w-8 animate-spin text-indigo-500" />
      </div>
    ),
  }
)

interface Chapter {
  id: string
  book_id: string
  title: string | null
  content: string | null
  word_count: number | null
}

interface EnhanceResponse {
  result: string
}

export default function EditorPage() {
  const params = useParams()
  const router = useRouter()
  const queryClient = useQueryClient()
  const { isAuthenticated, isLoading: authLoading, accessToken } = useAuth()
  const chapterId = params.id as string

  const [editorValue, setEditorValue] = useState<Descendant[]>([])
  const [title, setTitle] = useState('')
  const [isSaving, setIsSaving] = useState(false)
  const [lastSaved, setLastSaved] = useState<Date | null>(null)
  const [wordCount, setWordCount] = useState(0)
  const [isEditorReady, setIsEditorReady] = useState(false)

  useEffect(() => {
    if (!authLoading && !isAuthenticated) router.push('/')
  }, [authLoading, isAuthenticated, router])

  const { data: chapter, isLoading: chapterLoading } = useQuery<Chapter>({
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

  useEffect(() => {
    if (chapter && !isEditorReady) {
      setEditorValue(deserializeFromMarkdown(chapter.content ?? ''))
      setTitle(chapter.title ?? '')
      setWordCount(chapter.word_count ?? 0)
      setIsEditorReady(true)
    }
  }, [chapter, isEditorReady])

  const calculateWordCount = useCallback((value: Descendant[]) => {
    const text = serializeToMarkdown(value)
    return text.split(/\s+/).filter(Boolean).length
  }, [])

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

  const debouncedSave = useDebouncedCallback((value: Descendant[]) => {
    setIsSaving(true)
    saveMutation.mutate(
      { title, content: serializeToMarkdown(value) },
      { onSettled: () => setIsSaving(false) }
    )
  }, 2000)

  const handleEditorChange = useCallback(
    (value: Descendant[]) => {
      setEditorValue(value)
      setWordCount(calculateWordCount(value))
      debouncedSave(value)
    },
    [calculateWordCount, debouncedSave]
  )

  const handleSave = useCallback(() => {
    setIsSaving(true)
    saveMutation.mutate(
      { title, content: serializeToMarkdown(editorValue) },
      { onSettled: () => setIsSaving(false) }
    )
  }, [editorValue, title, saveMutation])

  const enhanceMutation = useMutation({
    mutationFn: async (mode: string) => {
      const response = await fetch('/api/generate/enhance', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${accessToken}`,
        },
        body: JSON.stringify({
          chapter_id: chapterId,
          text: serializeToMarkdown(editorValue),
          mode,
        }),
      })
      if (!response.ok) throw new Error('Failed to enhance')
      return (await response.json()) as EnhanceResponse
    },
    onSuccess: (data) => {
      if (data.result) {
        setEditorValue(deserializeFromMarkdown(data.result))
      }
    },
  })

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault()
        handleSave()
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [handleSave])

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
      <div className="sticky top-16 z-40 bg-slate-900/95 backdrop-blur-xl border-b border-slate-800">
        <div className="max-w-6xl mx-auto px-4 py-3 flex items-center justify-between">
          <div className="flex items-center gap-4">
            <Link
              href={`/books/${chapter.book_id}`}
              className="p-2 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800 transition-colors"
            >
              <ArrowLeft className="h-5 w-5" />
            </Link>

            <div className="flex items-center gap-3">
              <div className="h-8 w-8 rounded-lg bg-indigo-500/20 flex items-center justify-center">
                <BookOpen className="h-4 w-4 text-indigo-400" />
              </div>
              <input
                type="text"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                onBlur={handleSave}
                className="text-lg font-semibold bg-transparent border-none focus:outline-none focus:ring-0 text-white placeholder-slate-500"
                placeholder="Chapter Title"
              />
            </div>
          </div>

          <div className="flex items-center gap-4">
            <div className="flex items-center gap-2 text-sm text-slate-500">
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

            <div className="text-sm text-slate-500 border-l border-slate-700 pl-4">
              {wordCount.toLocaleString()} words
            </div>

            <button onClick={handleSave} disabled={isSaving} className="btn-primary">
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

      <div className="flex-1 max-w-4xl mx-auto w-full">
        {isEditorReady && (
          <PlateEditor
            initialValue={editorValue}
            onChange={handleEditorChange}
            onAIEnhance={(type) => enhanceMutation.mutate(type)}
            isAILoading={enhanceMutation.isPending}
            placeholder="Start writing your chapter..."
            className="min-h-[calc(100vh-180px)]"
          />
        )}
      </div>

      {enhanceMutation.isPending && (
        <div className="fixed bottom-4 right-4 bg-slate-800 border border-purple-500/30 rounded-xl p-4 shadow-xl flex items-center gap-3 animate-fade-in">
          <Loader2 className="h-5 w-5 animate-spin text-purple-400" />
          <span className="text-slate-300">AI is enhancing your content...</span>
        </div>
      )}
    </div>
  )
}
