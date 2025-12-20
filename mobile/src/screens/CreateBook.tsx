import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { ArrowLeft } from 'lucide-react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { api } from '../lib/api';
import { Input } from '../components/Input';
import { Textarea } from '../components/Textarea';
import { Button } from '../components/Button';
import { useUIStore } from '../stores/uiStore';
import type { Book, CreateBookData } from '../types';

export const CreateBook: React.FC = () => {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { showToast } = useUIStore();

  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [genre, setGenre] = useState('');

  const createMutation = useMutation({
    mutationFn: (data: CreateBookData) => api.createBook(data),
    onSuccess: (data: Book) => {
      queryClient.invalidateQueries({ queryKey: ['books'] });
      showToast('Book created successfully!', 'success');
      navigate(`/books/${data.id}`);
    },
    onError: () => {
      showToast('Failed to create book', 'error');
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim()) {
      showToast('Please enter a title', 'error');
      return;
    }
    createMutation.mutate({
      title: title.trim(),
      description: description.trim() || undefined,
      genre: genre || undefined,
    });
  };

  return (
    <div className="min-h-screen p-4 pb-24">
      <div className="flex items-center gap-3 mb-6">
        <button
          onClick={() => navigate(-1)}
          className="p-2 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800"
        >
          <ArrowLeft className="h-5 w-5" />
        </button>
        <h1 className="text-2xl font-playfair font-bold">Create New Book</h1>
      </div>

      <form onSubmit={handleSubmit} className="space-y-4 max-w-2xl">
        <Input
          label="Title *"
          type="text"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          placeholder="Enter book title"
          maxLength={200}
          required
        />

        <Textarea
          label="Description"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          placeholder="What's your book about?"
          maxLength={1000}
          rows={4}
        />

        <div>
          <label className="block text-sm font-medium text-slate-300 mb-2">Genre</label>
          <select
            value={genre}
            onChange={(e) => setGenre(e.target.value)}
            className="input"
          >
            <option value="">Select a genre</option>
            <option value="fantasy">Fantasy</option>
            <option value="science-fiction">Science Fiction</option>
            <option value="mystery">Mystery</option>
            <option value="thriller">Thriller</option>
            <option value="romance">Romance</option>
            <option value="horror">Horror</option>
            <option value="literary-fiction">Literary Fiction</option>
            <option value="historical-fiction">Historical Fiction</option>
            <option value="young-adult">Young Adult</option>
            <option value="non-fiction">Non-Fiction</option>
            <option value="memoir">Memoir</option>
            <option value="poetry">Poetry</option>
            <option value="other">Other</option>
          </select>
        </div>

        <div className="flex gap-3 pt-4">
          <Button type="button" variant="secondary" onClick={() => navigate(-1)}>
            Cancel
          </Button>
          <Button type="submit" isLoading={createMutation.isPending} className="flex-1">
            Create Book
          </Button>
        </div>
      </form>
    </div>
  );
};
