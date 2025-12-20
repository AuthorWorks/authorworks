import React, { useState } from 'react';
import { Link } from 'react-router-dom';
import { BookOpen, Plus, Search } from 'lucide-react';
import { useQuery } from '@tanstack/react-query';
import { api } from '../lib/api';
import { Card } from '../components/Card';
import { Button } from '../components/Button';
import type { BooksResponse, Book } from '../types';

export const Books: React.FC = () => {
  const [searchQuery, setSearchQuery] = useState('');
  const [statusFilter, setStatusFilter] = useState('all');

  const { data, isLoading } = useQuery<BooksResponse>({
    queryKey: ['books', statusFilter],
    queryFn: () => api.getBooks({ status: statusFilter !== 'all' ? statusFilter : undefined }),
  });

  const books = data?.books || [];
  const filteredBooks = books.filter((book: Book) =>
    book.title.toLowerCase().includes(searchQuery.toLowerCase())
  );

  return (
    <div className="min-h-screen p-4 pb-24">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-playfair font-bold">My Books</h1>
          <p className="text-slate-400 text-sm mt-1">Manage your book projects</p>
        </div>
        <Link to="/books/new">
          <Button>
            <Plus className="h-4 w-4 mr-2" />
            New
          </Button>
        </Link>
      </div>

      {/* Search and Filter */}
      <div className="space-y-3 mb-6">
        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-5 w-5 text-slate-500" />
          <input
            type="text"
            placeholder="Search books..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="input pl-10"
          />
        </div>

        <div className="flex gap-2 overflow-x-auto pb-2">
          {['all', 'draft', 'writing', 'editing', 'published'].map((status) => (
            <button
              key={status}
              onClick={() => setStatusFilter(status)}
              className={`px-4 py-2 rounded-lg text-sm font-medium whitespace-nowrap transition-all ${
                statusFilter === status
                  ? 'bg-indigo-500 text-white'
                  : 'bg-slate-800 text-slate-400 hover:text-white'
              }`}
            >
              {status === 'all' ? 'All' : status.charAt(0).toUpperCase() + status.slice(1)}
            </button>
          ))}
        </div>
      </div>

      {/* Books Grid */}
      {isLoading ? (
        <Card className="text-center py-12">
          <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-indigo-500 mx-auto"></div>
        </Card>
      ) : filteredBooks.length === 0 ? (
        <Card className="text-center py-12">
          <BookOpen className="h-16 w-16 text-slate-600 mx-auto mb-4" />
          <h3 className="text-xl font-medium text-slate-300 mb-2">No books found</h3>
          <p className="text-slate-500 mb-6 text-sm">
            {searchQuery ? 'Try a different search term' : 'Create your first book to get started'}
          </p>
          {!searchQuery && (
            <Link to="/books/new">
              <Button>
                <Plus className="h-4 w-4 mr-2" />
                Create Book
              </Button>
            </Link>
          )}
        </Card>
      ) : (
        <div className="space-y-3">
          {filteredBooks.map((book: Book) => (
            <Link key={book.id} to={`/books/${book.id}`}>
              <Card>
                <div className="flex items-center gap-3">
                  <div className="h-20 w-14 rounded-lg bg-gradient-to-br from-indigo-500/20 to-purple-500/20 flex items-center justify-center shrink-0 overflow-hidden">
                    {book.cover_image_url ? (
                      <img src={book.cover_image_url} alt={book.title} className="h-full w-full object-cover" />
                    ) : (
                      <BookOpen className="h-6 w-6 text-indigo-400" />
                    )}
                  </div>
                  <div className="flex-1 min-w-0">
                    <h3 className="font-semibold text-white truncate">{book.title}</h3>
                    <p className="text-slate-400 text-sm truncate">{book.description || 'No description'}</p>
                    <div className="flex items-center gap-3 mt-2 text-xs text-slate-500">
                      <span>{book.word_count?.toLocaleString() || 0} words</span>
                      <span className={`capitalize px-2 py-0.5 rounded-full ${
                        book.status === 'published' ? 'bg-green-500/20 text-green-400' :
                        book.status === 'editing' ? 'bg-yellow-500/20 text-yellow-400' :
                        'bg-slate-700 text-slate-400'
                      }`}>
                        {book.status}
                      </span>
                    </div>
                  </div>
                </div>
              </Card>
            </Link>
          ))}
        </div>
      )}
    </div>
  );
};
