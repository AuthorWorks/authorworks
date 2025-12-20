import React from 'react';
import { Link } from 'react-router-dom';
import { BookOpen, TrendingUp, Sparkles, Clock, Plus, ArrowRight } from 'lucide-react';
import { useQuery } from '@tanstack/react-query';
import { api } from '../lib/api';
import { Card } from '../components/Card';
import { Button } from '../components/Button';
import { useAuthStore } from '../stores/authStore';
import type { DashboardStats, BooksResponse, Book } from '../types';

export const Dashboard: React.FC = () => {
  const { user } = useAuthStore();

  const { data: stats } = useQuery<DashboardStats>({
    queryKey: ['dashboard-stats'],
    queryFn: () => api.getDashboardStats(),
  });

  const { data: booksData, isLoading: booksLoading } = useQuery<BooksResponse>({
    queryKey: ['recent-books'],
    queryFn: () => api.getBooks({ limit: 5 }),
  });

  const recentBooks = booksData?.books || [];

  return (
    <div className="min-h-screen p-4 pb-24">
      {/* Welcome Section */}
      <div className="mb-6">
        <h1 className="text-2xl font-playfair font-bold mb-1">
          Welcome back, {user?.name?.split(' ')[0] || 'Author'}
        </h1>
        <p className="text-slate-400 text-sm">Here's what's happening with your books today.</p>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-2 gap-3 mb-6">
        <Card className="!p-3">
          <div className="flex items-center justify-between mb-2">
            <div className="h-8 w-8 rounded-lg bg-indigo-500/20 flex items-center justify-center">
              <BookOpen className="h-4 w-4 text-indigo-400" />
            </div>
            <span className="text-xl font-bold">{stats?.totalBooks || 0}</span>
          </div>
          <p className="text-slate-400 text-xs">Total Books</p>
        </Card>

        <Card className="!p-3">
          <div className="flex items-center justify-between mb-2">
            <div className="h-8 w-8 rounded-lg bg-purple-500/20 flex items-center justify-center">
              <TrendingUp className="h-4 w-4 text-purple-400" />
            </div>
            <span className="text-xl font-bold">{stats?.totalWords?.toLocaleString() || 0}</span>
          </div>
          <p className="text-slate-400 text-xs">Words</p>
        </Card>

        <Card className="!p-3">
          <div className="flex items-center justify-between mb-2">
            <div className="h-8 w-8 rounded-lg bg-pink-500/20 flex items-center justify-center">
              <Sparkles className="h-4 w-4 text-pink-400" />
            </div>
            <span className="text-xl font-bold">{stats?.aiWordsUsed?.toLocaleString() || 0}</span>
          </div>
          <p className="text-slate-400 text-xs">AI Words</p>
        </Card>

        <Card className="!p-3">
          <div className="flex items-center justify-between mb-2">
            <div className="h-8 w-8 rounded-lg bg-green-500/20 flex items-center justify-center">
              <Clock className="h-4 w-4 text-green-400" />
            </div>
            <span className="text-xl font-bold">{stats?.activeStreak || 0}</span>
          </div>
          <p className="text-slate-400 text-xs">Day Streak</p>
        </Card>
      </div>

      {/* Recent Books */}
      <div className="mb-6">
        <div className="flex items-center justify-between mb-3">
          <h2 className="text-lg font-semibold">Recent Books</h2>
          <Link to="/books" className="text-indigo-400 text-sm flex items-center gap-1">
            View all <ArrowRight className="h-3 w-3" />
          </Link>
        </div>

        <div className="space-y-3">
          {booksLoading ? (
            <Card className="text-center py-8">
              <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-indigo-500 mx-auto"></div>
            </Card>
          ) : recentBooks.length === 0 ? (
            <Card className="text-center py-8">
              <BookOpen className="h-12 w-12 text-slate-600 mx-auto mb-3" />
              <h3 className="text-lg font-medium text-slate-300 mb-2">No books yet</h3>
              <p className="text-slate-500 mb-4 text-sm">Create your first book to get started</p>
              <Link to="/books/new">
                <Button>
                  <Plus className="h-4 w-4 mr-2" />
                  Create Book
                </Button>
              </Link>
            </Card>
          ) : (
            recentBooks.map((book: Book) => (
              <Link key={book.id} to={`/books/${book.id}`}>
                <Card>
                  <div className="flex items-start gap-3">
                    <div className="h-16 w-12 rounded-lg bg-gradient-to-br from-indigo-500/20 to-purple-500/20 flex items-center justify-center shrink-0">
                      {book.cover_url ? (
                        <img src={book.cover_url} alt={book.title} className="h-full w-full rounded-lg object-cover" />
                      ) : (
                        <BookOpen className="h-6 w-6 text-indigo-400" />
                      )}
                    </div>
                    <div className="flex-1 min-w-0">
                      <h3 className="font-semibold text-white truncate">{book.title}</h3>
                      <p className="text-slate-400 text-sm mt-1 line-clamp-2">
                        {book.description || 'No description'}
                      </p>
                      <div className="flex items-center gap-3 mt-2 text-xs text-slate-500">
                        <span>{book.word_count?.toLocaleString() || 0} words</span>
                        <span className="capitalize px-2 py-0.5 rounded-full bg-slate-800">
                          {book.status || 'draft'}
                        </span>
                      </div>
                    </div>
                  </div>
                </Card>
              </Link>
            ))
          )}
        </div>
      </div>

      {/* Quick Actions */}
      <div className="mb-6">
        <h2 className="text-lg font-semibold mb-3">Quick Actions</h2>
        <div className="space-y-2">
          <Link to="/books/new">
            <Card>
              <div className="flex items-center gap-3">
                <div className="h-10 w-10 rounded-lg bg-indigo-500/20 flex items-center justify-center">
                  <Plus className="h-5 w-5 text-indigo-400" />
                </div>
                <span className="text-slate-300">Create new book</span>
              </div>
            </Card>
          </Link>
        </div>
      </div>
    </div>
  );
};
