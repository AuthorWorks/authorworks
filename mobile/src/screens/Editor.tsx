import React, { useState, useEffect, useRef } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { ArrowLeft, Save, Bold, Italic, Heading1, Heading2, List, Loader2, CheckCircle } from 'lucide-react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useDebouncedCallback } from 'use-debounce';
import { api } from '../lib/api';
import { useUIStore } from '../stores/uiStore';
import type { Chapter } from '../types';

export const Editor: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { showToast } = useUIStore();
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const [content, setContent] = useState('');
  const [title, setTitle] = useState('');
  const [isSaving, setIsSaving] = useState(false);
  const [lastSaved, setLastSaved] = useState<Date | null>(null);
  const [wordCount, setWordCount] = useState(0);

  const { data: chapter, isLoading } = useQuery<Chapter>({
    queryKey: ['chapter', id],
    queryFn: () => api.getChapter(id!),
    enabled: !!id,
  });

  useEffect(() => {
    if (chapter) {
      setContent(chapter.content || '');
      setTitle(chapter.title || '');
      setWordCount(chapter.word_count || 0);
    }
  }, [chapter]);

  const saveMutation = useMutation({
    mutationFn: (data: { title: string; content: string }) => api.updateChapter(id!, data),
    onSuccess: () => {
      setLastSaved(new Date());
      queryClient.invalidateQueries({ queryKey: ['chapter', id] });
      queryClient.invalidateQueries({ queryKey: ['chapters'] });
    },
    onError: () => {
      showToast('Failed to save changes', 'error');
    },
  });

  const debouncedSave = useDebouncedCallback((newContent: string) => {
    setIsSaving(true);
    saveMutation.mutate(
      { title, content: newContent },
      {
        onSettled: () => setIsSaving(false),
      }
    );
  }, 2000);

  const handleContentChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newContent = e.target.value;
    setContent(newContent);
    const words = newContent.split(/\s+/).filter(Boolean).length;
    setWordCount(words);
    debouncedSave(newContent);
  };

  const handleSave = () => {
    setIsSaving(true);
    saveMutation.mutate(
      { title, content },
      {
        onSettled: () => setIsSaving(false),
      }
    );
  };

  const insertFormatting = (format: string) => {
    if (!textareaRef.current) return;

    const textarea = textareaRef.current;
    const start = textarea.selectionStart;
    const end = textarea.selectionEnd;
    const selectedText = content.substring(start, end);
    let newText = '';

    switch (format) {
      case 'bold':
        newText = content.substring(0, start) + `**${selectedText || 'bold text'}**` + content.substring(end);
        break;
      case 'italic':
        newText = content.substring(0, start) + `*${selectedText || 'italic text'}*` + content.substring(end);
        break;
      case 'h1':
        newText = content.substring(0, start) + `# ${selectedText || 'Heading'}` + content.substring(end);
        break;
      case 'h2':
        newText = content.substring(0, start) + `## ${selectedText || 'Heading'}` + content.substring(end);
        break;
      case 'list':
        newText = content.substring(0, start) + `- ${selectedText || 'List item'}` + content.substring(end);
        break;
      default:
        return;
    }

    setContent(newText);
    debouncedSave(newText);
  };

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault();
        handleSave();
      }
      if ((e.metaKey || e.ctrlKey) && e.key === 'b') {
        e.preventDefault();
        insertFormatting('bold');
      }
      if ((e.metaKey || e.ctrlKey) && e.key === 'i') {
        e.preventDefault();
        insertFormatting('italic');
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [content, title]);

  if (isLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-indigo-500"></div>
      </div>
    );
  }

  if (!chapter) {
    return (
      <div className="min-h-screen flex items-center justify-center p-4">
        <div className="text-center">
          <h2 className="text-xl font-bold mb-2">Chapter not found</h2>
          <button onClick={() => navigate(-1)} className="btn-primary">
            Go Back
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-slate-950 flex flex-col">
      {/* Toolbar */}
      <div className="sticky top-0 z-40 bg-slate-900/95 backdrop-blur-xl border-b border-slate-800">
        <div className="p-3 flex items-center justify-between">
          <div className="flex items-center gap-3 flex-1 min-w-0">
            <button
              onClick={() => navigate(-1)}
              className="p-2 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800"
            >
              <ArrowLeft className="h-5 w-5" />
            </button>

            <input
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              onBlur={handleSave}
              className="flex-1 min-w-0 text-sm font-semibold bg-transparent border-none focus:outline-none text-white truncate"
              placeholder="Chapter Title"
            />
          </div>

          <div className="flex items-center gap-2 ml-3">
            <span className="text-xs text-slate-500 whitespace-nowrap">
              {wordCount.toLocaleString()} words
            </span>
          </div>
        </div>

        {/* Formatting Toolbar */}
        <div className="px-3 pb-2 flex items-center gap-1 overflow-x-auto">
          <button
            onClick={() => insertFormatting('bold')}
            className="p-2 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800"
            title="Bold (Cmd+B)"
          >
            <Bold className="h-4 w-4" />
          </button>
          <button
            onClick={() => insertFormatting('italic')}
            className="p-2 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800"
            title="Italic (Cmd+I)"
          >
            <Italic className="h-4 w-4" />
          </button>
          <button
            onClick={() => insertFormatting('h1')}
            className="p-2 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800"
            title="Heading 1"
          >
            <Heading1 className="h-4 w-4" />
          </button>
          <button
            onClick={() => insertFormatting('h2')}
            className="p-2 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800"
            title="Heading 2"
          >
            <Heading2 className="h-4 w-4" />
          </button>
          <button
            onClick={() => insertFormatting('list')}
            className="p-2 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800"
            title="List"
          >
            <List className="h-4 w-4" />
          </button>

          <div className="flex-1" />

          {isSaving ? (
            <div className="flex items-center gap-2 text-xs text-slate-500">
              <Loader2 className="h-3 w-3 animate-spin" />
              <span>Saving...</span>
            </div>
          ) : lastSaved ? (
            <div className="flex items-center gap-2 text-xs text-green-500">
              <CheckCircle className="h-3 w-3" />
              <span>Saved {lastSaved.toLocaleTimeString()}</span>
            </div>
          ) : null}

          <button onClick={handleSave} disabled={isSaving} className="btn-primary !px-3 !py-1 text-xs">
            {isSaving ? <Loader2 className="h-3 w-3 animate-spin" /> : <Save className="h-3 w-3" />}
          </button>
        </div>
      </div>

      {/* Editor */}
      <div className="flex-1 p-4">
        <textarea
          ref={textareaRef}
          value={content}
          onChange={handleContentChange}
          placeholder="Start writing your chapter..."
          className="w-full h-full min-h-[calc(100vh-180px)] bg-transparent text-slate-200 text-base leading-relaxed resize-none focus:outline-none placeholder-slate-600 font-serif"
          style={{ fontFamily: 'Georgia, serif' }}
        />
      </div>
    </div>
  );
};
