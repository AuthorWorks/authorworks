'use client'

import { useMutation } from '@tanstack/react-query'
import { ArrowLeft, BookOpen, ChevronDown, ChevronUp, Feather, FileText, Lightbulb, Loader2, Sparkles, Users } from 'lucide-react'
import Link from 'next/link'
import { useRouter } from 'next/navigation'
import { useEffect, useState } from 'react'
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
  const { isAuthenticated, isLoading, accessToken } = useAuth()

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

  // AI generation options - ON by default since chapters are core to the engine
  const [generateOutline, setGenerateOutline] = useState(true)
  const [outlinePrompt, setOutlinePrompt] = useState('')
  const [chapterCount, setChapterCount] = useState(12)

  // Detailed progress tracking for better UX
  const [creationStatus, setCreationStatus] = useState<'idle' | 'creating' | 'generating' | 'saving' | 'done'>('idle')
  const [statusMessage, setStatusMessage] = useState('')
  const [errorMessage, setErrorMessage] = useState('')
  const [currentStep, setCurrentStep] = useState(0)
  const [totalSteps, setTotalSteps] = useState(0)
  const [stepDetails, setStepDetails] = useState<string[]>([])

  // Detailed creation steps matching core engine phases
  const creationSteps = [
    // Phase 1: Initial Setup and Context
    { phase: 'creating', label: 'Braindump', detail: 'Processing your creative ideas...' },
    { phase: 'creating', label: 'Genre', detail: 'Analyzing genre conventions...' },
    { phase: 'creating', label: 'Style', detail: 'Defining writing style...' },
    { phase: 'generating', label: 'Characters', detail: 'Developing character profiles...' },
    { phase: 'generating', label: 'Synopsis', detail: 'Crafting story synopsis...' },
    // Phase 2: Book Structure
    { phase: 'generating', label: 'Outline', detail: 'Generating book outline...' },
    { phase: 'generating', label: 'Chapters', detail: 'Structuring chapter outlines...' },
    { phase: 'generating', label: 'Scenes', detail: 'Planning scene breakdowns...' },
    // Phase 3: Saving
    { phase: 'saving', label: 'Content', detail: 'Saving chapter content...' },
    { phase: 'saving', label: 'Metadata', detail: 'Storing book metadata...' },
    { phase: 'done', label: 'Complete', detail: 'Your book is ready!' },
  ]

  // Progress animation helper
  const advanceProgress = async (stepIndex: number, duration: number = 800) => {
    setCurrentStep(stepIndex)
    const step = creationSteps[stepIndex]
    setCreationStatus(step.phase as any)
    setStatusMessage(step.detail)
    setStepDetails(prev => [...prev, step.label])
    await new Promise(resolve => setTimeout(resolve, duration))
  }

  // Map phase names from book generator to step indices
  const phaseToStep: Record<string, number> = {
    'setup': 0,
    'braindump': 0,
    'genre': 1,
    'style': 2,
    'characters': 3,
    'synopsis': 4,
    'outline': 5,
    'chapters': 6,
    'scenes': 7,
    'content': 8,
    'rendering': 8,
    'export': 9,
    'complete': 10,
  }

  // Poll job status until complete
  const pollJobStatus = async (jobId: string, bookId: string): Promise<void> => {
    let attempts = 0
    const maxAttempts = 300 // 5 minutes max with 1s intervals

    while (attempts < maxAttempts) {
      try {
        const response = await fetch(`/api/generate/book/status/${jobId}`, {
          headers: { Authorization: `Bearer ${accessToken}` },
        })

        if (!response.ok) {
          throw new Error('Failed to get job status')
        }

        const status = await response.json()
        console.log('Job status:', status)

        // Update UI based on job status
        const stepIndex = phaseToStep[status.phase] ?? currentStep
        if (stepIndex !== currentStep) {
          await advanceProgress(stepIndex, 0)
        }
        setStatusMessage(status.current_step)

        if (status.status === 'completed') {
          await advanceProgress(10, 500)
          router.push(`/books/${bookId}`)
          return
        }

        if (status.status === 'failed') {
          throw new Error(status.error || 'Book generation failed')
        }

        if (status.status === 'cancelled') {
          throw new Error('Book generation was cancelled')
        }

        // Wait before next poll
        await new Promise(resolve => setTimeout(resolve, 1000))
        attempts++
      } catch (error) {
        console.error('Polling error:', error)
        throw error
      }
    }

    throw new Error('Book generation timed out')
  }

  const createMutation = useMutation({
    mutationFn: async () => {
      setTotalSteps(creationSteps.length)
      setStepDetails([])

      // Step 1: Braindump - Processing
      await advanceProgress(0)

      // Build metadata object with advanced creative options
      const metadata: Record<string, any> = {}
      if (braindump) metadata.braindump = braindump
      if (stylePreset) metadata.style_preset = stylePreset
      if (stylePreset === 'custom' && customStyle) metadata.custom_style = customStyle
      if (characters) metadata.characters = characters
      if (synopsis) metadata.synopsis = synopsis

      // Step 2: Genre - Create book record
      await advanceProgress(1)

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
      if (!response.ok) {
        const errorData = await response.json().catch(() => ({ error: 'Unknown error' }))
        console.error('Book creation failed:', response.status, errorData)
        throw new Error(errorData.error || `Failed to create book (${response.status})`)
      }
      return response.json()
    },
    onSuccess: async (data) => {
      if (generateOutline) {
        // Step 3: Style - Start full book generation via core engine
        await advanceProgress(2)

        try {
          // Start full book generation job using the core engine
          const generationResponse = await fetch('/api/generate/book', {
            method: 'POST',
            headers: {
              'Content-Type': 'application/json',
              Authorization: `Bearer ${accessToken}`,
            },
            body: JSON.stringify({
              book_id: data.id,
              title,
              description: description || '',
              braindump: braindump || '',
              genre: genre || '',
              style: stylePreset === 'custom' ? customStyle : stylePreset,
              characters: characters || '',
              synopsis: synopsis || '',
              outline_prompt: outlinePrompt || '',
              chapter_count: chapterCount,
              author_name: 'AuthorWorks User',
            }),
          })

          if (!generationResponse.ok) {
            const errorData = await generationResponse.json().catch(() => ({ error: 'Unknown error' }))
            console.error('Book generation failed to start:', errorData)
            setErrorMessage(errorData.error || 'Failed to start book generation')
            setCreationStatus('idle')
            return
          }

          const { job_id } = await generationResponse.json()
          console.log('Book generation started, job_id:', job_id)

          // Poll for job status and update UI with real progress
          // Note: pollJobStatus handles redirect on success
          await pollJobStatus(job_id, data.id)
        } catch (error) {
          console.error('Generation error:', error)
          setErrorMessage(error instanceof Error ? error.message : 'Book generation failed')
          setCreationStatus('idle')
        }
      } else {
        // No generation requested, just redirect
        await advanceProgress(10, 500)
        router.push(`/books/${data.id}`)
      }
    },
    onError: (error: Error) => {
      console.error('Book creation mutation error:', error)
      setCreationStatus('idle')
      setStatusMessage('')
      setErrorMessage(error.message || 'An error occurred while creating your book')
    },
  })

  // Redirect to home if not authenticated (use effect to avoid render-time navigation)
  useEffect(() => {
    if (!isLoading && !isAuthenticated) {
    router.push('/')
    }
  }, [isLoading, isAuthenticated, router])

  // Show loading state while checking auth
  if (isLoading || !isAuthenticated) {
    return (
      <div className="max-w-3xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="animate-pulse">
          <div className="h-8 bg-slate-700 rounded w-32 mb-4"></div>
          <div className="h-12 bg-slate-700 rounded w-64 mb-2"></div>
          <div className="h-4 bg-slate-700 rounded w-48"></div>
        </div>
      </div>
    )
  }

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (!title.trim()) return
    setErrorMessage('')  // Clear any previous error
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
              <span className="text-sm font-normal text-slate-500">(helps AI generate better content)</span>
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
                AI Chapter Outline
              </h2>
              <p className="text-slate-500 text-sm mt-1">
                Generate a chapter-by-chapter story structure (recommended)
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

        {/* Creation Progress Overlay */}
        {creationStatus !== 'idle' && (
          <div className="fixed inset-0 bg-slate-950/95 flex items-center justify-center z-50">
            <div className="max-w-lg w-full mx-4 p-8">
              {/* Phase Header */}
              <div className="text-center mb-8">
                <div className="relative w-16 h-16 mx-auto mb-4">
                  {creationStatus === 'creating' && (
                    <BookOpen className="w-16 h-16 text-indigo-500 animate-pulse" />
                  )}
                  {creationStatus === 'generating' && (
                    <Sparkles className="w-16 h-16 text-purple-500 animate-bounce" />
                  )}
                  {creationStatus === 'saving' && (
                    <FileText className="w-16 h-16 text-emerald-500 animate-pulse" />
                  )}
                  {creationStatus === 'done' && (
                    <BookOpen className="w-16 h-16 text-green-500" />
                  )}
                  {creationStatus !== 'done' && (
                    <div className="absolute inset-0 -m-2 flex items-center justify-center">
                      <div className="w-20 h-20 border-2 border-indigo-500/30 border-t-indigo-500 rounded-full animate-spin" />
                    </div>
                  )}
                </div>
                <h3 className="text-xl font-semibold mb-1">
                  {creationStatus === 'creating' && 'Phase 1: Initial Setup'}
                  {creationStatus === 'generating' && 'Phase 2: AI Generation'}
                  {creationStatus === 'saving' && 'Phase 3: Saving'}
                  {creationStatus === 'done' && 'Complete!'}
                </h3>
                <p className="text-slate-400 text-sm">{statusMessage}</p>
              </div>

              {/* Step Progress */}
              <div className="space-y-2 mb-6">
                {creationSteps.map((step, index) => {
                  const isComplete = index < currentStep
                  const isCurrent = index === currentStep
                  const isPending = index > currentStep

                  return (
                    <div
                      key={index}
                      className={`flex items-center gap-3 px-4 py-2 rounded-lg transition-all duration-300 ${
                        isComplete ? 'bg-green-500/10 text-green-400' :
                        isCurrent ? 'bg-indigo-500/20 text-white' :
                        'text-slate-600'
                      }`}
                    >
                      <div className={`w-6 h-6 rounded-full flex items-center justify-center text-xs font-medium ${
                        isComplete ? 'bg-green-500 text-white' :
                        isCurrent ? 'bg-indigo-500 text-white' :
                        'bg-slate-800 text-slate-500'
                      }`}>
                        {isComplete ? '✓' : index + 1}
                      </div>
                      <span className={`flex-1 ${isCurrent ? 'font-medium' : ''}`}>
                        {step.label}
                      </span>
                      {isCurrent && (
                        <Loader2 className="w-4 h-4 animate-spin text-indigo-400" />
                      )}
                    </div>
                  )
                })}
              </div>

              {/* Progress Bar */}
              <div className="mb-4">
                <div className="h-2 bg-slate-800 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-gradient-to-r from-indigo-500 to-purple-500 transition-all duration-500"
                    style={{ width: `${((currentStep + 1) / creationSteps.length) * 100}%` }}
                  />
                </div>
                <p className="text-slate-500 text-xs text-center mt-2">
                  Step {currentStep + 1} of {creationSteps.length}
                </p>
              </div>

              {/* Generating note */}
              {creationStatus === 'generating' && (
                <p className="text-slate-500 text-xs text-center">
                  AI is working through each step...
                </p>
              )}
            </div>
          </div>
        )}

        {/* Error Message */}
        {errorMessage && (
          <div className="p-4 bg-red-500/10 border border-red-500/30 rounded-lg text-red-400">
            <p className="font-medium">Error creating book</p>
            <p className="text-sm mt-1">{errorMessage}</p>
          </div>
        )}

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
                {generateOutline ? 'Creating & Generating...' : 'Creating...'}
              </>
            ) : (
              <>
                <Sparkles className="h-5 w-5 mr-2" />
                {generateOutline ? 'Create Book & Generate Outline' : 'Create Book'}
              </>
            )}
          </button>
        </div>
      </form>
    </div>
  )
}

