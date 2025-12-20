import { useAuthStore } from '../stores/authStore';
import type {
  LoginResponse,
  DashboardStats,
  BooksResponse,
  Book,
  ChaptersResponse,
  Chapter,
  User,
  CreateBookData,
  UpdateBookData,
  CreateChapterData,
  UpdateChapterData,
} from '../types';

const API_BASE_URL = import.meta.env.VITE_API_URL || 'https://api.authorworks.io/v1';

class APIClient {
  private getHeaders(): HeadersInit {
    const token = useAuthStore.getState().token;
    const headers: HeadersInit = {
      'Content-Type': 'application/json',
    };

    if (token) {
      headers['Authorization'] = `Bearer ${token}`;
    }

    return headers;
  }

  async request<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
    const url = `${API_BASE_URL}${endpoint}`;
    const headers = this.getHeaders();

    try {
      const response = await fetch(url, {
        ...options,
        headers: {
          ...headers,
          ...options.headers,
        },
      });

      if (response.status === 401) {
        // Token expired, clear auth
        useAuthStore.getState().clearAuth();
        throw new Error('Authentication expired');
      }

      if (!response.ok) {
        const error = await response.json().catch(() => ({ message: 'Request failed' }));
        throw new Error(error.message || `HTTP ${response.status}`);
      }

      return await response.json();
    } catch (error) {
      if (error instanceof Error) {
        throw error;
      }
      throw new Error('Network error');
    }
  }

  // Auth
  async login(email: string, password: string): Promise<LoginResponse> {
    return this.request<LoginResponse>('/auth/login', {
      method: 'POST',
      body: JSON.stringify({ email, password }),
    });
  }

  // Dashboard
  async getDashboardStats(): Promise<DashboardStats> {
    return this.request<DashboardStats>('/dashboard/stats');
  }

  // Books
  async getBooks(params?: { status?: string; limit?: number; offset?: number }): Promise<BooksResponse> {
    const query = new URLSearchParams(params as any).toString();
    return this.request<BooksResponse>(`/books${query ? `?${query}` : ''}`);
  }

  async getBook(id: string): Promise<Book> {
    return this.request<Book>(`/books/${id}`);
  }

  async createBook(data: CreateBookData): Promise<Book> {
    return this.request<Book>('/books', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async updateBook(id: string, data: UpdateBookData): Promise<Book> {
    return this.request<Book>(`/books/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
  }

  async deleteBook(id: string): Promise<void> {
    return this.request<void>(`/books/${id}`, {
      method: 'DELETE',
    });
  }

  // Chapters
  async getChapters(bookId: string): Promise<ChaptersResponse> {
    return this.request<ChaptersResponse>(`/books/${bookId}/chapters`);
  }

  async getChapter(id: string): Promise<Chapter> {
    return this.request<Chapter>(`/chapters/${id}`);
  }

  async createChapter(bookId: string, data: CreateChapterData): Promise<Chapter> {
    return this.request<Chapter>(`/books/${bookId}/chapters`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async updateChapter(id: string, data: UpdateChapterData): Promise<Chapter> {
    return this.request<Chapter>(`/chapters/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
  }

  async deleteChapter(id: string): Promise<void> {
    return this.request<void>(`/chapters/${id}`, {
      method: 'DELETE',
    });
  }

  // User
  async getUserProfile(): Promise<User> {
    return this.request<User>('/user/profile');
  }

  async updateUserProfile(data: Partial<User>): Promise<User> {
    return this.request<User>('/user/profile', {
      method: 'PUT',
      body: JSON.stringify(data),
    });
  }
}

export const api = new APIClient();
