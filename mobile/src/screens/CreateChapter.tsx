import React, { useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { ArrowLeft } from 'lucide-react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { api } from '../lib/api';
import { Input } from '../components/Input';
import { Button } from '../components/Button';
import { useUIStore } from '../stores/uiStore';
import type { Chapter, CreateChapterData } from '../types';

export const CreateChapter: React.FC = () => {
  const { bookId } = useParams<{ bookId: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { showToast } = useUIStore();

  const [title, setTitle] = useState('');

  const createMutation = useMutation({
    mutationFn: (data: CreateChapterData) => api.createChapter(bookId!, data),
    onSuccess: (data: Chapter) => {
      queryClient.invalidateQueries({ queryKey: ['chapters', bookId] });
      showToast('Chapter created successfully!', 'success');
      navigate(`/editor/${data.id}`);
    },
    onError: () => {
      showToast('Failed to create chapter', 'error');
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    createMutation.mutate({
      title: title.trim() || undefined,
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
        <h1 className="text-2xl font-playfair font-bold">Add New Chapter</h1>
      </div>

      <form onSubmit={handleSubmit} className="space-y-4 max-w-2xl">
        <Input
          label="Chapter Title (optional)"
          type="text"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          placeholder="Leave blank to auto-number"
        />

        <p className="text-sm text-slate-400">
          If you don't enter a title, the chapter will be automatically numbered (e.g., "Chapter 1").
        </p>

        <div className="flex gap-3 pt-4">
          <Button type="button" variant="secondary" onClick={() => navigate(-1)}>
            Cancel
          </Button>
          <Button type="submit" isLoading={createMutation.isPending} className="flex-1">
            Create Chapter
          </Button>
        </div>
      </form>
    </div>
  );
};
