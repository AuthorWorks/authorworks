'use client'

import { useMutation } from '@tanstack/react-query'
import { ArrowLeft, BookOpen, ChevronDown, ChevronUp, Feather, FileText, Lightbulb, Loader2, Sparkles, Users } from 'lucide-react'
import Link from 'next/link'
import { useRouter } from 'next/navigation'
import { useState } from 'react'
import { useAuth } from '../../hooks/useAuth'

const GENRES = [
  'Fantasy', 'Science Fiction', 'Romance', 'Mystery', 'Thriller',
  'Horror', 'Historical Fiction', 'Literary Fiction', 'Young Adult',
  'Children\'s', 'Non-Fiction', 'Biography', 'Self-Help', 'Other'
]

const STYLE_PRESETS = [
  { id: 'literary', name: 'Literary', description: 'Rich prose with deep character exploration and thematic depth' },
  { id: 'commercial', name: 'Commercial', description: 'Fast-paced, accessible storytelling focused on plot and entertainment' },
  { id: 'lyrical', name: 'Lyrical', description: 'Poetic language with emphasis on rhythm, imagery, and beauty' },
  { id: 'minimalist', name: 'Minimalist', description: 'Sparse, precise prose that says more with less' },
  { id: 'humorous', name: 'Humorous', description: 'Witty, satirical, or comedic tone throughout the narrative' },
  { id: 'dark', name: 'Dark & Atmospheric', description: 'Moody, intense prose with gothic or noir influences' },
  { id: 'custom', name: 'Custom', description: 'Define your own unique writing style' },
]

export default function NewBookContent() {
  const router = useRouter()
  const { isAuthenticated, accessToken } = useAuth()

  // Basic info
  const [title, setTitle] = useState('')
  const [description, setDescription] = useState('')
  const [genre, setGenre] = useState('')

  // Advanced creative options
  const [showAdvanced, setShowAdvanced] = useState(false)
  const [braindump, setBraindump] = useState('')
  const [stylePreset, setStylePreset] = useState('')
  const [customStyle, setCustomStyle] = useState('')
  const [characters, setCharacters] = useState('')
  const [synopsis, setSynopsis] = useState('')

  // AI generation options
  const [generateOutline, setGenerateOutline] = useState(false)
  const [outlinePrompt, setOutlinePrompt] = useState('')
  const [chapterCount, setChapterCount] = useState(12)

  const createMutation = useMutation({
    mutationFn: async () => {
      // Build metadata object with advanced creative options
      const metadata: Record<string, any> = {}
      if (braindump) metadata.braindump = braindump
      if (stylePreset) metadata.style_preset = stylePreset
      if (stylePreset === 'custom' && customStyle) metadata.custom_style = customStyle
      if (characters) metadata.characters = characters
      if (synopsis) metadata.synopsis = synopsis

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
          metadata,
        }),
      })
      if (!response.ok) throw new Error('Failed to create book')
      return response.json()
    },
    onSuccess: async (data) => {
      if (generateOutline) {
        // Combine all creative inputs into a comprehensive prompt for outline generation
        const fullPrompt = [
          outlinePrompt,
          braindump && `Creative Ideas: ${braindump}`,
          characters && `Characters: ${characters}`,
          synopsis && `Story Synopsis: ${synopsis}`,
        ].filter(Boolean).join('\n\n')

        await fetch('/api/generate/outline', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            Authorization: `Bearer ${accessToken}`,
          },
          body: JSON.stringify({
            book_id: data.id,
            prompt: fullPrompt || `Generate a compelling ${genre || 'fiction'} novel outline`,
            genre,
            style: stylePreset === 'custom' ? customStyle : stylePreset,
            chapter_count: chapterCount,
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
        {/* Essential Book Details */}
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
              placeholder="A brief tagline or description for your book"
              rows={3}
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

        {/* Advanced Creative Options - Collapsible */}
        <div className="card">
          <button
            type="button"
            onClick={() => setShowAdvanced(!showAdvanced)}
            className="w-full flex items-center justify-between text-left"
          >
            <h2 className="text-xl font-semibold flex items-center gap-2">
              <Lightbulb className="h-5 w-5 text-amber-400" />
              Creative Details
              <span className="text-sm font-normal text-slate-500">(optional)</span>
            </h2>
            {showAdvanced ? (
              <ChevronUp className="h-5 w-5 text-slate-400" />
            ) : (
              <ChevronDown className="h-5 w-5 text-slate-400" />
            )}
          </button>

          {showAdvanced && (
            <div className="mt-6 space-y-6 animate-fade-in">
              {/* Braindump / Creative Ideas */}
              <div>
                <label className="block text-sm font-medium text-slate-300 mb-2 flex items-center gap-2">
                  <Lightbulb className="h-4 w-4 text-amber-400" />
                  Braindump / Creative Ideas
                </label>
                <textarea
                  value={braindump}
                  onChange={(e) => setBraindump(e.target.value)}
                  placeholder="Pour out all your ideas here! Themes, settings, plot twists, world-building details, inspiration sources, tone... anything that comes to mind about your story."
                  rows={5}
                  className="textarea"
                />
                <p className="text-slate-500 text-xs mt-1">
                  This is your creative playground - no structure needed, just ideas.
                </p>
              </div>

              {/* Writing Style */}
              <div>
                <label className="block text-sm font-medium text-slate-300 mb-2 flex items-center gap-2">
                  <Feather className="h-4 w-4 text-emerald-400" />
                  Writing Style
                </label>
                <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-2 mb-3">
                  {STYLE_PRESETS.map((style) => (
                    <button
                      key={style.id}
                      type="button"
                      onClick={() => setStylePreset(style.id)}
                      className={`p-3 rounded-lg border text-left transition-all ${
                        stylePreset === style.id
                          ? 'border-indigo-500 bg-indigo-500/10 text-white'
                          : 'border-slate-700 hover:border-slate-600 text-slate-400 hover:text-slate-300'
                      }`}
                    >
                      <div className="font-medium text-sm">{style.name}</div>
                    </button>
                  ))}
                </div>
                {stylePreset && (
                  <p className="text-slate-400 text-sm mb-3">
                    {STYLE_PRESETS.find(s => s.id === stylePreset)?.description}
                  </p>
                )}
                {stylePreset === 'custom' && (
                  <textarea
                    value={customStyle}
                    onChange={(e) => setCustomStyle(e.target.value)}
                    placeholder="Describe your ideal writing style... e.g., 'A blend of Terry Pratchett's wit and Ursula K. Le Guin's worldbuilding, with short punchy sentences and vivid metaphors.'"
                    rows={3}
                    className="textarea"
                  />
                )}
              </div>

              {/* Characters */}
              <div>
                <label className="block text-sm font-medium text-slate-300 mb-2 flex items-center gap-2">
                  <Users className="h-4 w-4 text-purple-400" />
                  Key Characters
                </label>
                <textarea
                  value={characters}
                  onChange={(e) => setCharacters(e.target.value)}
                  placeholder="Describe your main characters...

Example:
• Maya Chen - A 30-year-old marine biologist, curious and stubborn, haunted by her father's disappearance at sea
• Captain Reyes - Weathered fishing boat captain, knows more than he lets on, protective but secretive"
                  rows={5}
                  className="textarea font-mono text-sm"
                />
              </div>

              {/* Synopsis */}
              <div>
                <label className="block text-sm font-medium text-slate-300 mb-2 flex items-center gap-2">
                  <FileText className="h-4 w-4 text-cyan-400" />
                  Story Synopsis
                </label>
                <textarea
                  value={synopsis}
                  onChange={(e) => setSynopsis(e.target.value)}
                  placeholder="Write a synopsis of your story. This can be as detailed or brief as you like - from a single paragraph pitch to a full summary with beginning, middle, and end."
                  rows={5}
                  className="textarea"
                />
              </div>
            </div>
          )}
        </div>

        {/* AI Outline Generation */}
        <div className="card space-y-6">
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-xl font-semibold flex items-center gap-2">
                <Sparkles className="h-5 w-5 text-purple-400" />
                AI Outline Generation
              </h2>
              <p className="text-slate-500 text-sm mt-1">
                Let AI create a chapter-by-chapter outline for your story
              </p>
            </div>
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
            <div className="space-y-4 animate-fade-in">
              <div>
                <label className="block text-sm font-medium text-slate-300 mb-2">
                  Additional story direction <span className="text-slate-500">(optional)</span>
                </label>
                <textarea
                  value={outlinePrompt}
                  onChange={(e) => setOutlinePrompt(e.target.value)}
                  placeholder="Any specific directions for the outline? Plot points you want included, story beats, pacing preferences..."
                  rows={4}
                  className="textarea"
                />
                <p className="text-slate-500 text-xs mt-1">
                  The AI will combine this with your creative details above to generate the outline.
                </p>
              </div>

              <div>
                <label className="block text-sm font-medium text-slate-300 mb-2">
                  Target chapter count
                </label>
                <div className="flex items-center gap-4">
                  <input
                    type="range"
                    min={5}
                    max={30}
                    value={chapterCount}
                    onChange={(e) => setChapterCount(Number(e.target.value))}
                    className="flex-1 h-2 bg-slate-700 rounded-lg appearance-none cursor-pointer accent-indigo-500"
                  />
                  <span className="text-white font-medium w-12 text-center">{chapterCount}</span>
                </div>
                <p className="text-slate-500 text-xs mt-1">
                  Approximate number of chapters (the AI may adjust based on story needs)
                </p>
              </div>
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

