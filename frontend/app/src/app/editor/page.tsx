'use client'

import { useQuery } from '@tanstack/react-query'
import { ArrowRight, BookOpen, Plus } from 'lucide-react'
import Link from 'next/link'
import { useRouter } from 'next/navigation'
import { useEffect } from 'react'
import { useAuth } from '../hooks/useAuth'

export default function EditorPage() {
  const router = useRouter()
  const { isAuthenticated, isLoading, accessToken } = useAuth()

  // Redirect to home if not authenticated
  useEffect(() => {
    if (!isLoading && !isAuthenticated) {
      router.push('/')
    }
  }, [isLoading, isAuthenticated, router])

  // Fetch recent books to continue writing
  const { data: recentBooks, isLoading: booksLoading } = useQuery({
    queryKey: ['recent-books-editor'],
    queryFn: async () => {
      const response = await fetch('/api/books?limit=10', {
        headers: {
          Authorization: `Bearer ${accessToken}`,
        },
      })
      if (!response.ok) throw new Error('Failed to fetch books')
      return response.json()
    },
    enabled: isAuthenticated && !!accessToken,
  })

  // If there's a recent book, redirect to it
  useEffect(() => {
    if (recentBooks?.books?.length > 0) {
      const lastEditedBook = recentBooks.books[0]
      router.push(`/editor/${lastEditedBook.id}`)
    }
  }, [recentBooks, router])

  if (isLoading || booksLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-indigo-500"></div>
      </div>
    )
  }

  if (!isAuthenticated) {
    return null
  }

  // No books - show create book prompt
  return (
    <div className="max-w-2xl mx-auto px-4 sm:px-6 lg:px-8 py-16">
      <div className="text-center">
        <div className="h-20 w-20 rounded-full bg-indigo-500/20 flex items-center justify-center mx-auto mb-6">
          <BookOpen className="h-10 w-10 text-indigo-400" />
        </div>
        <h1 className="text-3xl font-playfair font-bold mb-4">Start Writing</h1>
        <p className="text-slate-400 mb-8">
          You don't have any books yet. Create your first book to start writing!
        </p>
        <div className="flex flex-col sm:flex-row gap-4 justify-center">
          <Link href="/books/new" className="btn-primary">
            <Plus className="h-5 w-5 mr-2" />
            Create New Book
          </Link>
          <Link href="/dashboard" className="btn-secondary">
            Back to Dashboard
            <ArrowRight className="h-5 w-5 ml-2" />
          </Link>
        </div>
      </div>
    </div>
  )
}
