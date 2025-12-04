'use client'

import { useEffect } from 'react'
import { useRouter } from 'next/navigation'
import Link from 'next/link'
import { BookOpen, Plus, Clock, TrendingUp, Sparkles, ArrowRight } from 'lucide-react'
import { useAuth } from '../hooks/useAuth'
import { useQuery } from '@tanstack/react-query'

export default function Dashboard() {
  const router = useRouter()
  const { isAuthenticated, isLoading, accessToken, user } = useAuth()

  // Redirect to home if not authenticated
  useEffect(() => {
    if (!isLoading && !isAuthenticated) {
      router.push('/')
    }
  }, [isLoading, isAuthenticated, router])

  // Fetch dashboard data
  const { data: stats } = useQuery({
    queryKey: ['dashboard-stats'],
    queryFn: async () => {
      const response = await fetch('/api/dashboard/stats', {
        headers: {
          Authorization: `Bearer ${accessToken}`,
        },
      })
      if (!response.ok) throw new Error('Failed to fetch stats')
      return response.json()
    },
    enabled: isAuthenticated,
  })

  const { data: recentBooks } = useQuery({
    queryKey: ['recent-books'],
    queryFn: async () => {
      const response = await fetch('/api/books?limit=5', {
        headers: {
          Authorization: `Bearer ${accessToken}`,
        },
      })
      if (!response.ok) throw new Error('Failed to fetch books')
      return response.json()
    },
    enabled: isAuthenticated,
  })

  if (isLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-indigo-500"></div>
      </div>
    )
  }

  if (!isAuthenticated) {
    return null
  }

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      {/* Welcome Section */}
      <div className="mb-8">
        <h1 className="text-3xl font-playfair font-bold mb-2">
          Welcome back, {user?.name?.split(' ')[0] || 'Author'}
        </h1>
        <p className="text-slate-400">
          Here's what's happening with your books today.
        </p>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-6 mb-8">
        <div className="card">
          <div className="flex items-center justify-between mb-4">
            <div className="h-10 w-10 rounded-lg bg-indigo-500/20 flex items-center justify-center">
              <BookOpen className="h-5 w-5 text-indigo-400" />
            </div>
            <span className="text-2xl font-bold">{stats?.totalBooks || 0}</span>
          </div>
          <p className="text-slate-400 text-sm">Total Books</p>
        </div>

        <div className="card">
          <div className="flex items-center justify-between mb-4">
            <div className="h-10 w-10 rounded-lg bg-purple-500/20 flex items-center justify-center">
              <TrendingUp className="h-5 w-5 text-purple-400" />
            </div>
            <span className="text-2xl font-bold">{stats?.totalWords?.toLocaleString() || 0}</span>
          </div>
          <p className="text-slate-400 text-sm">Words Written</p>
        </div>

        <div className="card">
          <div className="flex items-center justify-between mb-4">
            <div className="h-10 w-10 rounded-lg bg-pink-500/20 flex items-center justify-center">
              <Sparkles className="h-5 w-5 text-pink-400" />
            </div>
            <span className="text-2xl font-bold">{stats?.aiWordsUsed?.toLocaleString() || 0}</span>
          </div>
          <p className="text-slate-400 text-sm">AI Words Used</p>
        </div>

        <div className="card">
          <div className="flex items-center justify-between mb-4">
            <div className="h-10 w-10 rounded-lg bg-green-500/20 flex items-center justify-center">
              <Clock className="h-5 w-5 text-green-400" />
            </div>
            <span className="text-2xl font-bold">{stats?.activeStreak || 0}</span>
          </div>
          <p className="text-slate-400 text-sm">Day Streak</p>
        </div>
      </div>

      {/* Main Content Grid */}
      <div className="grid lg:grid-cols-3 gap-8">
        {/* Recent Books */}
        <div className="lg:col-span-2">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-xl font-semibold">Recent Books</h2>
            <Link href="/books" className="text-indigo-400 hover:text-indigo-300 text-sm flex items-center gap-1">
              View all <ArrowRight className="h-4 w-4" />
            </Link>
          </div>

          <div className="space-y-4">
            {recentBooks?.books?.length > 0 ? (
              recentBooks.books.map((book: any) => (
                <Link key={book.id} href={`/books/${book.id}`}>
                  <div className="card hover:border-indigo-500/50 transition-all cursor-pointer group">
                    <div className="flex items-start gap-4">
                      <div className="h-24 w-16 rounded-lg bg-gradient-to-br from-indigo-500/20 to-purple-500/20 flex items-center justify-center shrink-0">
                        {book.cover_url ? (
                          <img src={book.cover_url} alt={book.title} className="h-24 w-16 rounded-lg object-cover" />
                        ) : (
                          <BookOpen className="h-8 w-8 text-indigo-400" />
                        )}
                      </div>
                      <div className="flex-1 min-w-0">
                        <h3 className="font-semibold text-white group-hover:text-indigo-400 transition-colors truncate">
                          {book.title}
                        </h3>
                        <p className="text-slate-400 text-sm mt-1 line-clamp-2">
                          {book.description || 'No description'}
                        </p>
                        <div className="flex items-center gap-4 mt-3 text-xs text-slate-500">
                          <span>{book.word_count?.toLocaleString() || 0} words</span>
                          <span className="capitalize px-2 py-0.5 rounded-full bg-slate-800">
                            {book.status || 'draft'}
                          </span>
                        </div>
                      </div>
                    </div>
                  </div>
                </Link>
              ))
            ) : (
              <div className="card text-center py-12">
                <BookOpen className="h-12 w-12 text-slate-600 mx-auto mb-4" />
                <h3 className="text-lg font-medium text-slate-300 mb-2">No books yet</h3>
                <p className="text-slate-500 mb-4">Create your first book to get started</p>
                <Link href="/books/new" className="btn-primary">
                  <Plus className="h-4 w-4 mr-2" />
                  Create Book
                </Link>
              </div>
            )}
          </div>
        </div>

        {/* Quick Actions & Tips */}
        <div className="space-y-6">
          {/* Quick Actions */}
          <div>
            <h2 className="text-xl font-semibold mb-4">Quick Actions</h2>
            <div className="space-y-3">
              <Link
                href="/books/new"
                className="card flex items-center gap-3 hover:border-indigo-500/50 transition-all cursor-pointer group"
              >
                <div className="h-10 w-10 rounded-lg bg-indigo-500/20 flex items-center justify-center group-hover:scale-110 transition-transform">
                  <Plus className="h-5 w-5 text-indigo-400" />
                </div>
                <span className="text-slate-300 group-hover:text-white">Create new book</span>
              </Link>

              <Link
                href="/editor"
                className="card flex items-center gap-3 hover:border-purple-500/50 transition-all cursor-pointer group"
              >
                <div className="h-10 w-10 rounded-lg bg-purple-500/20 flex items-center justify-center group-hover:scale-110 transition-transform">
                  <Sparkles className="h-5 w-5 text-purple-400" />
                </div>
                <span className="text-slate-300 group-hover:text-white">Continue writing</span>
              </Link>
            </div>
          </div>

          {/* Writing Tips */}
          <div>
            <h2 className="text-xl font-semibold mb-4">Writing Tip</h2>
            <div className="card bg-gradient-to-br from-indigo-900/20 to-purple-900/20 border-indigo-500/30">
              <div className="flex items-start gap-3">
                <Sparkles className="h-5 w-5 text-indigo-400 shrink-0 mt-0.5" />
                <div>
                  <p className="text-slate-300 text-sm">
                    "The first draft is just you telling yourself the story. Don't worry about perfection—
                    that's what editing is for."
                  </p>
                  <p className="text-slate-500 text-xs mt-2">— Terry Pratchett</p>
                </div>
              </div>
            </div>
          </div>

          {/* Usage */}
          <div>
            <h2 className="text-xl font-semibold mb-4">This Month</h2>
            <div className="card">
              <div className="space-y-4">
                <div>
                  <div className="flex justify-between text-sm mb-1">
                    <span className="text-slate-400">AI Words</span>
                    <span className="text-slate-300">
                      {stats?.aiWordsUsed?.toLocaleString() || 0} / {stats?.aiWordsLimit?.toLocaleString() || '5,000'}
                    </span>
                  </div>
                  <div className="h-2 bg-slate-800 rounded-full overflow-hidden">
                    <div
                      className="h-full bg-gradient-to-r from-indigo-500 to-purple-500 rounded-full transition-all"
                      style={{
                        width: `${Math.min(((stats?.aiWordsUsed || 0) / (stats?.aiWordsLimit || 5000)) * 100, 100)}%`,
                      }}
                    />
                  </div>
                </div>

                <div>
                  <div className="flex justify-between text-sm mb-1">
                    <span className="text-slate-400">Storage</span>
                    <span className="text-slate-300">
                      {stats?.storageUsedGb?.toFixed(2) || 0} GB / {stats?.storageLimitGb || 1} GB
                    </span>
                  </div>
                  <div className="h-2 bg-slate-800 rounded-full overflow-hidden">
                    <div
                      className="h-full bg-gradient-to-r from-green-500 to-emerald-500 rounded-full transition-all"
                      style={{
                        width: `${Math.min(((stats?.storageUsedGb || 0) / (stats?.storageLimitGb || 1)) * 100, 100)}%`,
                      }}
                    />
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

