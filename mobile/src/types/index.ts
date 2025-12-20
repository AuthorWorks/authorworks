// API Response Types

export interface User {
  id: string;
  email: string;
  username: string;
  name: string;
  avatar_url?: string;
  bio?: string;
  website?: string;
}

export interface Book {
  id: string;
  user_id: string;
  title: string;
  description: string | null;
  genre: string | null;
  status: 'draft' | 'writing' | 'editing' | 'published';
  cover_url?: string;
  cover_image_url?: string;
  word_count: number;
  created_at: string;
  updated_at: string;
}

export interface Chapter {
  id: string;
  book_id: string;
  chapter_number: number;
  title: string | null;
  content: string | null;
  word_count: number;
  status?: string;
  created_at: string;
  updated_at: string;
}

export interface DashboardStats {
  totalBooks: number;
  totalWords: number;
  aiWordsUsed: number;
  activeStreak: number;
  aiWordsLimit?: number;
  storageUsedGb?: number;
  storageLimitGb?: number;
}

export interface BooksResponse {
  books: Book[];
  total?: number;
  limit?: number;
  offset?: number;
}

export interface ChaptersResponse {
  chapters: Chapter[];
}

export interface LoginResponse {
  token: string;
  user: User;
}

export interface CreateBookData {
  title: string;
  description?: string;
  genre?: string;
}

export interface UpdateBookData {
  title?: string;
  description?: string;
  genre?: string;
  status?: string;
}

export interface CreateChapterData {
  title?: string;
}

export interface UpdateChapterData {
  title?: string;
  content?: string;
}
