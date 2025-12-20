import React from 'react';
import { useParams, Link, useNavigate } from 'react-router-dom';
import { ArrowLeft, Plus, BookOpen, Trash2, Edit } from 'lucide-react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { api } from '../lib/api';
import { Card } from '../components/Card';
import { Button } from '../components/Button';
import { useUIStore } from '../stores/uiStore';
import type { Book, ChaptersResponse, Chapter } from '../types';

export const BookDetail: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { showToast } = useUIStore();

  const { data: book, isLoading: bookLoading } = useQuery<Book>({
    queryKey: ['book', id],
    queryFn: () => api.getBook(id!),
    enabled: !!id,
  });

  const { data: chaptersData, isLoading: chaptersLoading } = useQuery<ChaptersResponse>({
    queryKey: ['chapters', id],
    queryFn: () => api.getChapters(id!),
    enabled: !!id,
  });

  const deleteBookMutation = useMutation({
    mutationFn: () => api.deleteBook(id!),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['books'] });
      showToast('Book deleted successfully', 'success');
      navigate('/books');
    },
    onError: () => {
      showToast('Failed to delete book', 'error');
    },
  });

  const deleteChapterMutation = useMutation({
    mutationFn: (chapterId: string) => api.deleteChapter(chapterId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['chapters', id] });
      showToast('Chapter deleted successfully', 'success');
    },
    onError: () => {
      showToast('Failed to delete chapter', 'error');
    },
  });

  const chapters = chaptersData?.chapters || [];

  const handleDeleteBook = () => {
    if (window.confirm('Are you sure you want to delete this book? This cannot be undone.')) {
      deleteBookMutation.mutate();
    }
  };

  const handleDeleteChapter = (chapterId: string) => {
    if (window.confirm('Are you sure you want to delete this chapter?')) {
      deleteChapterMutation.mutate(chapterId);
    }
  };

  if (bookLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-indigo-500"></div>
      </div>
    );
  }

  if (!book) {
    return (
      <div className="min-h-screen flex items-center justify-center p-4">
        <Card className="text-center py-12 max-w-md">
          <BookOpen className="h-16 w-16 text-slate-600 mx-auto mb-4" />
          <h2 className="text-xl font-bold mb-2">Book not found</h2>
          <p className="text-slate-400 mb-6">This book may have been deleted.</p>
          <Link to="/books">
            <Button>Back to Books</Button>
          </Link>
        </Card>
      </div>
    );
  }

  return (
    <div className="min-h-screen p-4 pb-24">
      {/* Header */}
      <div className="flex items-center gap-3 mb-6">
        <Link to="/books" className="p-2 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800">
          <ArrowLeft className="h-5 w-5" />
        </Link>
        <h1 className="text-xl font-playfair font-bold flex-1 truncate">{book.title}</h1>
      </div>

      {/* Book Info */}
      <Card className="mb-6">
        <div className="flex gap-4 mb-4">
          <div className="h-32 w-24 rounded-lg bg-gradient-to-br from-indigo-500/20 to-purple-500/20 flex items-center justify-center shrink-0">
            {book.cover_image_url ? (
              <img src={book.cover_image_url} alt={book.title} className="h-full w-full rounded-lg object-cover" />
            ) : (
              <BookOpen className="h-12 w-12 text-indigo-400" />
            )}
          </div>
          <div className="flex-1 min-w-0">
            <h2 className="text-xl font-bold mb-2">{book.title}</h2>
            <p className="text-slate-400 text-sm mb-3 line-clamp-3">{book.description || 'No description'}</p>
            <div className="flex flex-wrap gap-2 text-xs">
              <span className="px-2 py-1 rounded-full bg-slate-800 text-slate-400">
                {book.word_count?.toLocaleString() || 0} words
              </span>
              <span className={`px-2 py-1 rounded-full capitalize ${
                book.status === 'published' ? 'bg-green-500/20 text-green-400' :
                book.status === 'editing' ? 'bg-yellow-500/20 text-yellow-400' :
                'bg-slate-800 text-slate-400'
              }`}>
                {book.status}
              </span>
              {book.genre && (
                <span className="px-2 py-1 rounded-full bg-indigo-500/20 text-indigo-400">
                  {book.genre}
                </span>
              )}
            </div>
          </div>
        </div>

        <div className="flex gap-2">
          <Link to={`/books/${id}/edit`} className="flex-1">
            <Button variant="secondary" className="w-full">
              <Edit className="h-4 w-4 mr-2" />
              Edit Details
            </Button>
          </Link>
          <Button variant="secondary" onClick={handleDeleteBook} className="flex-1">
            <Trash2 className="h-4 w-4 mr-2" />
            Delete Book
          </Button>
        </div>
      </Card>

      {/* Chapters */}
      <div className="mb-6">
        <div className="flex items-center justify-between mb-3">
          <h2 className="text-lg font-semibold">Chapters ({chapters.length})</h2>
          <Link to={`/books/${id}/chapters/new`}>
            <Button>
              <Plus className="h-4 w-4 mr-2" />
              Add Chapter
            </Button>
          </Link>
        </div>

        <div className="space-y-2">
          {chaptersLoading ? (
            <Card className="text-center py-8">
              <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-indigo-500 mx-auto"></div>
            </Card>
          ) : chapters.length === 0 ? (
            <Card className="text-center py-8">
              <p className="text-slate-400 mb-4">No chapters yet. Add your first chapter to start writing!</p>
              <Link to={`/books/${id}/chapters/new`}>
                <Button>
                  <Plus className="h-4 w-4 mr-2" />
                  Add Chapter
                </Button>
              </Link>
            </Card>
          ) : (
            chapters.map((chapter: Chapter) => (
              <div key={chapter.id} className="flex items-center gap-2">
                <Link to={`/editor/${chapter.id}`} className="flex-1">
                  <Card>
                    <div className="flex items-center justify-between">
                      <div className="flex-1 min-w-0">
                        <h3 className="font-medium text-white truncate">
                          {chapter.chapter_number}. {chapter.title || `Chapter ${chapter.chapter_number}`}
                        </h3>
                        <div className="flex items-center gap-3 mt-1 text-xs text-slate-500">
                          <span>{chapter.word_count?.toLocaleString() || 0} words</span>
                          {chapter.updated_at && (
                            <span>{new Date(chapter.updated_at).toLocaleDateString()}</span>
                          )}
                        </div>
                      </div>
                    </div>
                  </Card>
                </Link>
                <button
                  onClick={() => handleDeleteChapter(chapter.id)}
                  className="p-3 rounded-lg text-red-400 hover:bg-red-500/20"
                >
                  <Trash2 className="h-4 w-4" />
                </button>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
};
