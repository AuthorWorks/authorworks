'use client'

import { useState } from 'react'
import { useRouter } from 'next/navigation'
import Link from 'next/link'
import { ArrowLeft, BookOpen, Sparkles, Loader2 } from 'lucide-react'
import { useAuth } from '../../hooks/useAuth'
import { useMutation } from '@tanstack/react-query'

const GENRES = [
  'Fantasy', 'Science Fiction', 'Romance', 'Mystery', 'Thriller',
  'Horror', 'Historical Fiction', 'Literary Fiction', 'Young Adult',
  'Children\'s', 'Non-Fiction', 'Biography', 'Self-Help', 'Other'
]

export default function NewBookContent() {
  const router = useRouter()
  const { isAuthenticated, accessToken } = useAuth()
  const [title, setTitle] = useState('')
  const [description, setDescription] = useState('')
  const [genre, setGenre] = useState('')
  const [generateOutline, setGenerateOutline] = useState(false)
  const [outlinePrompt, setOutlinePrompt] = useState('')

  const createMutation = useMutation({
    mutationFn: async () => {
      const response = await fetch('/api/books', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${accessToken}`,
        },
        body: JSON.stringify({
          title,
          description,
          genre,
        }),
      })
      if (!response.ok) throw new Error('Failed to create book')
      return response.json()
    },
    onSuccess: async (data) => {
      if (generateOutline && outlinePrompt) {
        await fetch('/api/generate/outline', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            Authorization: `Bearer ${accessToken}`,
          },
          body: JSON.stringify({
            book_id: data.id,
            prompt: outlinePrompt,
            genre,
          }),
        })
      }
      router.push(`/books/${data.id}`)
    },
  })

  if (!isAuthenticated) {
    router.push('/')
    return null
  }

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (!title.trim()) return
    createMutation.mutate()
  }

  return (
    <div className="max-w-3xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <div className="mb-8">
        <Link href="/books" className="inline-flex items-center gap-2 text-slate-400 hover:text-white mb-4">
          <ArrowLeft className="h-4 w-4" />
          Back to Books
        </Link>
        <h1 className="text-3xl font-playfair font-bold">Create New Book</h1>
        <p className="text-slate-400 mt-1">Start your next masterpiece</p>
      </div>

      <form onSubmit={handleSubmit} className="space-y-6">
        <div className="card space-y-6">
          <h2 className="text-xl font-semibold flex items-center gap-2">
            <BookOpen className="h-5 w-5 text-indigo-400" />
            Book Details
          </h2>

          <div>
            <label className="block text-sm font-medium text-slate-300 mb-2">
              Title <span className="text-red-400">*</span>
            </label>
            <input
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Enter your book title"
              className="input"
              required
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-slate-300 mb-2">
              Description
            </label>
            <textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Brief description of your book (optional)"
              rows={4}
              className="textarea"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-slate-300 mb-2">
              Genre
            </label>
            <select
              value={genre}
              onChange={(e) => setGenre(e.target.value)}
              className="input"
            >
              <option value="">Select a genre</option>
              {GENRES.map((g) => (
                <option key={g} value={g}>{g}</option>
              ))}
            </select>
          </div>
        </div>

        <div className="card space-y-6">
          <div className="flex items-center justify-between">
            <h2 className="text-xl font-semibold flex items-center gap-2">
              <Sparkles className="h-5 w-5 text-purple-400" />
              AI Outline Generation
            </h2>
            <label className="relative inline-flex items-center cursor-pointer">
              <input
                type="checkbox"
                checked={generateOutline}
                onChange={(e) => setGenerateOutline(e.target.checked)}
                className="sr-only peer"
              />
              <div className="w-11 h-6 bg-slate-700 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-indigo-500/25 rounded-full peer peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-indigo-600"></div>
            </label>
          </div>

          {generateOutline && (
            <div className="animate-fade-in">
              <label className="block text-sm font-medium text-slate-300 mb-2">
                Describe your book idea
              </label>
              <textarea
                value={outlinePrompt}
                onChange={(e) => setOutlinePrompt(e.target.value)}
                placeholder="Tell us about your book idea, main characters, plot points, themes, or any other details that will help generate a better outline..."
                rows={6}
                className="textarea"
              />
              <p className="text-slate-500 text-sm mt-2">
                The AI will generate a complete chapter outline based on your description.
              </p>
            </div>
          )}
        </div>

        <div className="flex gap-4">
          <Link href="/books" className="btn-secondary flex-1 justify-center">
            Cancel
          </Link>
          <button
            type="submit"
            disabled={!title.trim() || createMutation.isPending}
            className="btn-primary flex-1 justify-center disabled:opacity-50"
          >
            {createMutation.isPending ? (
              <>
                <Loader2 className="h-5 w-5 mr-2 animate-spin" />
                Creating...
              </>
            ) : (
              <>
                <BookOpen className="h-5 w-5 mr-2" />
                Create Book
              </>
            )}
          </button>
        </div>
      </form>
    </div>
  )
}

