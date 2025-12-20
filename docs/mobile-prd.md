# AuthorWorks Mobile - Product Requirements Document (PRD)

**Version:** 1.0
**Date:** December 18, 2025
**Status:** Draft for Review
**Author:** AI Product Team
**Target Platform:** iOS & Android via Tauri 2.0

---

## Executive Summary

This PRD defines the requirements for developing native mobile applications for AuthorWorks using Tauri 2.0. The mobile apps will provide core creative writing functionality optimized for on-the-go content creation, enabling authors to write, edit, and manage their book projects from iOS and Android devices.

### Goals
- Enable mobile-first writing experience for existing AuthorWorks users
- Provide 80% of core functionality with 100% mobile optimization
- Maintain consistent brand identity and UX patterns from web platform
- Leverage Tauri 2.0 for cross-platform code efficiency
- Ship MVP within 12-16 weeks with two-person team

### Non-Goals (Phase 1)
- Media transformation features (audio, video, graphic novels)
- Social/discovery features
- Complex publishing workflows
- Subscription/payment processing (web-only)
- Real-time collaborative editing
- Advanced analytics dashboard

---

## 1. Product Overview

### 1.1 Problem Statement

AuthorWorks users need the ability to:
- Write and edit content when away from their desktop
- Capture creative ideas immediately on mobile devices
- Review and organize book projects during commute/travel
- Maintain writing streak even when not at their desk

**Current Gap:** AuthorWorks is web-only, limiting access to desktop browsers. Mobile web experience is not optimized for writing-focused workflows.

### 1.2 Solution

Native iOS and Android applications built with Tauri 2.0 that provide:
- Distraction-free mobile writing interface
- Offline-capable editing with background sync
- Native mobile UX patterns (swipe, haptics, bottom nav)
- Optimized performance for mobile hardware
- Seamless sync with web platform

### 1.3 Success Metrics

**Phase 1 (MVP Launch)**
- 30% of active users install mobile app within 60 days
- 50% of mobile users write at least once per week
- Average mobile session: 10+ minutes
- Mobile-originated content: 15% of total platform words
- App Store rating: 4.0+ stars

**Phase 2 (6 months post-launch)**
- 50% of active users have mobile app installed
- Mobile users show 25% higher retention vs web-only
- 30% of all content created originates on mobile

---

## 2. User Personas & Use Cases

### 2.1 Primary Personas

**Persona 1: Commuter Claire**
- Age: 28-35
- Professional writer working on debut novel
- 45-minute daily commute via train
- **Needs:** Quick writing sessions, offline mode, auto-sync
- **Pain Points:** Loses ideas during commute, can't always bring laptop

**Persona 2: Parent Patrick**
- Age: 35-45
- Writes during lunch breaks and kid's activities
- Sporadic 10-20 minute writing windows
- **Needs:** Fast app launch, continue from last edit, mobile dictation
- **Pain Points:** Limited time, frequent interruptions

**Persona 3: Hobbyist Hannah**
- Age: 18-25
- Fan fiction writer, mobile-native user
- Writes primarily on phone, prefers dark mode
- **Needs:** Social sharing (future), simple UI, quick chapter navigation
- **Pain Points:** Desktop-first tools feel cumbersome

### 2.2 Core User Journeys

#### Journey 1: Quick Writing Session
```
User opens app → Auto-loads last edited chapter → Writes 200-500 words →
Background save → Closes app → Content synced to cloud
```
**Duration:** 5-15 minutes
**Frequency:** 2-3x per day
**Priority:** P0 (Must have)

#### Journey 2: Idea Capture
```
User has inspiration → Opens app → Quick-create new chapter/book →
Jots down notes → Tags for later → Returns to dashboard
```
**Duration:** 2-5 minutes
**Frequency:** 1-3x per week
**Priority:** P0 (Must have)

#### Journey 3: Book Organization
```
User reviews books → Searches for specific project → Views chapters →
Reorders chapters → Updates book metadata → Marks status
```
**Duration:** 5-10 minutes
**Frequency:** 1-2x per week
**Priority:** P1 (Should have)

#### Journey 4: Offline Writing
```
User opens app (no connectivity) → Sees cached books → Edits chapter →
App queues changes → Connectivity restored → Auto-sync → Conflict resolution
```
**Duration:** 15-60 minutes
**Frequency:** 1-2x per week
**Priority:** P0 (Must have)

---

## 3. Functional Requirements

### 3.1 Authentication & Onboarding

#### FR-1.1: OAuth2/OIDC Login (P0)
- **Description:** Users authenticate via Logto (same as web)
- **Acceptance Criteria:**
  - Support email/password login
  - Support OAuth providers (Google, Apple, GitHub)
  - Token stored securely in device keychain
  - Auto-login on app relaunch (remember me)
  - Session timeout after 30 days of inactivity
  - Logout clears all cached data (optional)

#### FR-1.2: First-Run Experience (P1)
- **Description:** Onboarding flow for new mobile users
- **Acceptance Criteria:**
  - 3-screen welcome carousel explaining key features
  - Permission requests (notifications, offline storage)
  - Option to enable biometric auth (Face ID/Touch ID)
  - Quick tutorial overlay on first dashboard view
  - Skip option available

### 3.2 Dashboard

#### FR-2.1: Statistics Overview (P0)
- **Description:** Display key writing metrics
- **Acceptance Criteria:**
  - Show 4 stat cards: Total Books, Words Written, AI Words, Streak
  - Real-time updates when data changes
  - Pull-to-refresh gesture
  - Offline mode shows last cached values
  - Loading states for initial fetch

#### FR-2.2: Recent Books List (P0)
- **Description:** Quick access to recently edited books
- **Acceptance Criteria:**
  - Display 5 most recent books
  - Show cover thumbnail (or placeholder), title, word count, status
  - Tap to navigate to book detail
  - Swipe left for quick actions (edit, delete)
  - Empty state with "Create Book" CTA

#### FR-2.3: Quick Actions (P0)
- **Description:** Floating action buttons for common tasks
- **Acceptance Criteria:**
  - FAB at bottom right: "Quick Write" (opens last chapter)
  - Secondary FAB: "New Book"
  - Haptic feedback on tap
  - Animate in/out on scroll

### 3.3 Book Management

#### FR-3.1: Books List View (P0)
- **Description:** Browse all books in optimized mobile layout
- **Acceptance Criteria:**
  - List view (default) with cover, title, meta, status
  - Search bar at top (searches title + description)
  - Filter by status (draft, writing, editing, published)
  - Pull-to-refresh
  - Infinite scroll pagination (20 books per page)
  - Empty state with "Create First Book" CTA

#### FR-3.2: Book Detail View (P0)
- **Description:** View book metadata and chapter list
- **Acceptance Criteria:**
  - Header: Cover image, title, description, word count
  - Action buttons: Edit Details, Add Chapter, Delete Book
  - Chapter list: Numbered, with title, word count, last edited
  - Tap chapter to open editor
  - Swipe chapter left for delete/edit actions
  - Drag-and-drop to reorder chapters (P1)

#### FR-3.3: Create Book (P0)
- **Description:** Mobile-optimized book creation flow
- **Acceptance Criteria:**
  - Modal or full-screen form
  - Required fields: Title
  - Optional fields: Description, Genre
  - Genre picker (dropdown or modal select)
  - AI outline generation (P1 - future)
  - Photo picker for cover upload (P1)
  - Validation: Title max 200 chars, Description max 1000 chars
  - Success: Navigate to new book detail

#### FR-3.4: Edit Book Metadata (P0)
- **Description:** Update book properties
- **Acceptance Criteria:**
  - Edit title, description, genre, status
  - Change cover image (P1)
  - Delete book with confirmation dialog
  - Optimistic UI updates
  - Error handling with retry option

### 3.4 Chapter Management

#### FR-4.1: Chapter List (P0)
- **Description:** View chapters within a book
- **Acceptance Criteria:**
  - Numbered list with chapter title, word count, last edited
  - Tap to open in editor
  - Swipe left for quick actions (edit, delete)
  - Long-press for reorder mode (P1)
  - Empty state: "Add your first chapter"

#### FR-4.2: Create Chapter (P0)
- **Description:** Add new chapter to book
- **Acceptance Criteria:**
  - Modal form: Chapter Title (optional, defaults to "Chapter N")
  - Auto-increment chapter number
  - Option to generate with AI (P1 - future)
  - Success: Navigate directly to editor

#### FR-4.3: Delete Chapter (P0)
- **Description:** Remove chapter from book
- **Acceptance Criteria:**
  - Swipe-to-delete gesture
  - Confirmation dialog with warning
  - Cannot be undone (no trash/restore in MVP)
  - Optimistic UI removal
  - Error handling

### 3.5 Editor

#### FR-5.1: Mobile Writing Interface (P0)
- **Description:** Distraction-free editor optimized for mobile
- **Acceptance Criteria:**
  - Full-screen text area with minimal chrome
  - Top bar: Back button, chapter title (editable), word count
  - Bottom bar: Formatting tools (appears on text selection)
  - Auto-save every 2 seconds (debounced)
  - Save indicator: "Saving..." / "Saved at 3:42 PM"
  - Keyboard toolbar with formatting shortcuts
  - System keyboard support (no custom keyboard)

#### FR-5.2: Auto-Save & Offline Editing (P0)
- **Description:** Robust save mechanism with offline support
- **Acceptance Criteria:**
  - Debounced save after 2 seconds of inactivity
  - Queue saves when offline (local-first)
  - Sync when connectivity restored
  - Conflict detection (last-write-wins for MVP)
  - Visual indicator: "Syncing... / Synced / Offline mode"
  - Persist draft in local storage

#### FR-5.3: Text Formatting (P0)
- **Description:** Basic text formatting tools
- **Acceptance Criteria:**
  - Bold, Italic (inline styles)
  - Headings H1, H2 (block styles)
  - Bulleted/numbered lists
  - Keyboard shortcuts (iOS: Cmd+B, Android: Ctrl+B)
  - Contextual formatting bar appears on text selection
  - Undo/Redo buttons (native iOS/Android support)

#### FR-5.4: Word Count Display (P0)
- **Description:** Real-time word count tracking
- **Acceptance Criteria:**
  - Display in top bar: "2,347 words"
  - Updates as user types (throttled for performance)
  - Counts words, not characters
  - Excludes formatting markup

#### FR-5.5: AI Enhancement (P1)
- **Description:** Quick AI suggestions for improving text
- **Acceptance Criteria:**
  - "Enhance" button in toolbar (Sparkles icon)
  - Bottom sheet with options: Style, Grammar, Dialogue, Pacing
  - Select text → Tap enhance → Shows loading → Replaces text
  - Undo option after enhancement
  - Deducts from AI word quota
  - Requires online connectivity
  - **Note:** Lower priority for MVP, can defer to post-launch

### 3.6 Reading Mode

#### FR-6.1: Read-Only Chapter View (P1)
- **Description:** Distraction-free reading experience
- **Acceptance Criteria:**
  - Toggle to read mode from editor
  - Full-screen text with optimized typography
  - Swipe left/right to navigate chapters
  - Progress indicator (% through chapter)
  - Tap to show/hide controls
  - **Note:** Nice-to-have, can defer if timeline tight

### 3.7 Sync & Offline

#### FR-7.1: Background Sync (P0)
- **Description:** Seamless data synchronization
- **Acceptance Criteria:**
  - Auto-sync on app launch (if online)
  - Background sync when app in foreground
  - Periodic sync every 5 minutes (when active)
  - Manual pull-to-refresh on dashboard/books
  - Sync status indicator in UI
  - Retry failed syncs with exponential backoff

#### FR-7.2: Offline Data Access (P0)
- **Description:** Local-first architecture for offline work
- **Acceptance Criteria:**
  - Cache recently accessed books (last 10)
  - Cache all chapters of opened books
  - Store drafts in local database (SQLite)
  - Offline banner: "No connection - changes will sync when online"
  - Queue mutations for later sync
  - Conflict resolution: last-write-wins (notify user of conflicts)

#### FR-7.3: Cache Management (P1)
- **Description:** User control over cached data
- **Acceptance Criteria:**
  - Settings option: "Clear cache"
  - Settings option: "Keep books offline" (toggle per book)
  - Cache size display in settings
  - Auto-evict old cache (LRU, keep 100 MB max)

### 3.8 Settings & Profile

#### FR-8.1: User Profile View (P0)
- **Description:** Display user info and preferences
- **Acceptance Criteria:**
  - Avatar, username, email (read-only)
  - Edit bio, website, social links
  - View subscription tier
  - Logout button

#### FR-8.2: App Settings (P1)
- **Description:** Configure app behavior
- **Acceptance Criteria:**
  - Dark mode toggle (system default)
  - Font size: Small, Medium, Large (for editor)
  - Auto-save interval: 2s, 5s, 10s
  - Biometric authentication toggle
  - Notification preferences
  - Clear cache button

#### FR-8.3: Usage Stats (P1)
- **Description:** Mobile-optimized stats dashboard
- **Acceptance Criteria:**
  - AI words used / limit (progress bar)
  - Storage used / limit (progress bar)
  - Link to upgrade (opens web in browser)

### 3.9 Notifications (P1 - Post-MVP)

#### FR-9.1: Push Notifications
- **Description:** Engage users with timely reminders
- **Acceptance Criteria:**
  - Writing streak reminder (daily at user-selected time)
  - "You haven't written in 3 days" re-engagement
  - Optional: Collaborator actions (if collaboration enabled)
  - User can disable in settings

---

## 4. Non-Functional Requirements

### 4.1 Performance

#### NFR-1.1: App Launch Time
- **Requirement:** Cold start < 3 seconds
- **Measurement:** Time from tap to interactive dashboard
- **Strategy:** Lazy load, code splitting, minimize dependencies

#### NFR-1.2: Editor Responsiveness
- **Requirement:** Keystroke latency < 16ms (60 FPS)
- **Measurement:** Input lag from keystroke to screen render
- **Strategy:** Optimize debounced save, avoid heavy DOM operations

#### NFR-1.3: Sync Speed
- **Requirement:** Sync 50 KB chapter content < 2 seconds (on 4G)
- **Measurement:** Time from save trigger to server confirmation
- **Strategy:** Delta sync, compression, efficient API design

#### NFR-1.4: Offline Mode
- **Requirement:** Full editing functionality without connectivity
- **Measurement:** 100% feature parity for core editing
- **Strategy:** Local-first architecture, SQLite, service worker patterns

### 4.2 Reliability

#### NFR-2.1: Data Integrity
- **Requirement:** Zero data loss in save operations
- **Requirement:** 99.9% successful sync rate
- **Strategy:** Transactional writes, conflict detection, retry logic

#### NFR-2.2: Crash Rate
- **Requirement:** < 0.5% crash-free sessions
- **Measurement:** Crash analytics via Sentry or Firebase
- **Strategy:** Comprehensive error handling, graceful degradation

#### NFR-2.3: Offline Resilience
- **Requirement:** App remains functional offline for 7+ days
- **Strategy:** Persistent local storage, sync queue, cache management

### 4.3 Security

#### NFR-3.1: Authentication
- **Requirement:** OAuth2/OIDC with Logto (same as web)
- **Requirement:** Secure token storage (iOS Keychain, Android Keystore)
- **Strategy:** Never store passwords, refresh tokens on expiry

#### NFR-3.2: Data Encryption
- **Requirement:** All API communication over HTTPS/TLS 1.3
- **Requirement:** Local database encrypted at rest (iOS FileProtection, Android EncryptedSharedPreferences)
- **Strategy:** Use platform-provided encryption APIs

#### NFR-3.3: Permissions
- **Requirement:** Minimal permissions (storage, notifications only)
- **Requirement:** Request permissions just-in-time with explanation
- **Strategy:** Follow platform best practices (iOS Privacy Manifest, Android permissions)

### 4.4 Usability

#### NFR-4.1: Accessibility
- **Requirement:** WCAG 2.1 AA compliance
- **Requirement:** VoiceOver (iOS) and TalkBack (Android) support
- **Strategy:** Semantic HTML, ARIA labels, contrast ratios, font scaling

#### NFR-4.2: Responsiveness
- **Requirement:** Support iPhone SE (small) to iPad Pro (large)
- **Requirement:** Support Android 320px to 1024px+ screens
- **Strategy:** Responsive layouts, test on multiple devices

#### NFR-4.3: Localization (Future)
- **Requirement:** English-only for MVP
- **Requirement:** Architecture supports i18n for future expansion
- **Strategy:** Use i18next or similar, externalize strings

### 4.5 Compatibility

#### NFR-5.1: iOS Support
- **Requirement:** iOS 13.0+ (covers 95%+ of active devices)
- **Requirement:** iPhone, iPad (universal app)
- **Strategy:** Use Tauri 2.0 iOS build target

#### NFR-5.2: Android Support
- **Requirement:** Android 8.0+ (API level 26+)
- **Requirement:** Support ARM and x86 architectures
- **Strategy:** Use Tauri 2.0 Android build target

### 4.6 Maintainability

#### NFR-6.1: Code Quality
- **Requirement:** 80%+ code coverage for critical paths (auth, sync, save)
- **Requirement:** Zero linting errors, warnings under 10
- **Strategy:** ESLint, Clippy, automated tests

#### NFR-6.2: Logging & Monitoring
- **Requirement:** Structured logging for all critical operations
- **Requirement:** Error tracking with Sentry or equivalent
- **Strategy:** Instrument save/sync/auth flows, track performance

---

## 5. Technical Architecture

### 5.1 Technology Stack

#### Frontend (UI Layer)
- **Framework:** React 18 with TypeScript
- **Styling:** Tailwind CSS (mobile-optimized)
- **State Management:** Zustand (lightweight, mobile-friendly)
- **Data Fetching:** TanStack Query (React Query) with offline plugin
- **Routing:** React Navigation (for native mobile feel)
- **Local Storage:** SQLite via Tauri plugin
- **Icons:** Lucide React (same as web)

#### Mobile Runtime
- **Framework:** Tauri 2.0.0+
- **iOS Bridge:** Swift (for native iOS APIs)
- **Android Bridge:** Kotlin (for native Android APIs)
- **Build Tool:** Tauri CLI 2.9.0+

#### Backend Integration
- **API:** Existing AuthorWorks microservices (REST)
- **Authentication:** Logto (OAuth2/OIDC)
- **Base URL:** `https://api.authorworks.io/v1`
- **Transport:** HTTPS with JWT bearer tokens

### 5.2 Application Architecture

```
┌─────────────────────────────────────────────────┐
│           Tauri Mobile App (iOS/Android)        │
├─────────────────────────────────────────────────┤
│  UI Layer (React + TypeScript)                  │
│  ┌──────────────────────────────────────────┐   │
│  │  Screens: Dashboard, Books, Editor,      │   │
│  │           Profile, Settings              │   │
│  └────────────────┬─────────────────────────┘   │
│                   │                             │
│  ┌────────────────▼─────────────────────────┐   │
│  │  State Management (Zustand)              │   │
│  │  - Auth State, Book State, UI State     │   │
│  └────────────────┬─────────────────────────┘   │
│                   │                             │
│  ┌────────────────▼─────────────────────────┐   │
│  │  Data Layer (React Query)                │   │
│  │  - Query cache, Mutation queue           │   │
│  │  - Offline plugin, Sync manager          │   │
│  └────────────────┬─────────────────────────┘   │
│                   │                             │
│  ┌────────────────▼─────────────────────────┐   │
│  │  API Client (Fetch/Axios)                │   │
│  │  - Request interceptors (auth tokens)    │   │
│  │  - Response interceptors (error handling)│   │
│  └────────────────┬─────────────────────────┘   │
│                   │                             │
├───────────────────┼─────────────────────────────┤
│  Tauri Core (Rust)                               │
│  ┌────────────────▼─────────────────────────┐   │
│  │  Commands API                            │   │
│  │  - save_to_local_db, sync_pending_changes│   │
│  └────────────────┬─────────────────────────┘   │
│                   │                             │
│  ┌────────────────▼─────────────────────────┐   │
│  │  SQLite Plugin (Local DB)                │   │
│  │  - books, chapters, drafts, sync_queue   │   │
│  └──────────────────────────────────────────┘   │
│                                                  │
│  ┌──────────────────────────────────────────┐   │
│  │  Platform Bridges                        │   │
│  │  iOS (Swift): Biometrics, Keychain,      │   │
│  │               Haptics, Share             │   │
│  │  Android (Kotlin): Biometrics, Keystore, │   │
│  │                    Haptics, Share        │   │
│  └──────────────────────────────────────────┘   │
└─────────────────────────────────────────────────┘
           │ HTTPS/TLS 1.3
           ▼
┌─────────────────────────────────────────────────┐
│  AuthorWorks Backend (Existing Services)        │
│  - User Service, Content Service,               │
│    Storage Service, Editor Service              │
└─────────────────────────────────────────────────┘
```

### 5.3 Data Flow

#### Save Flow (Online)
```
User types → Debounced (2s) → Zustand state update →
React Query mutation → API request with JWT →
Backend saves → Success response → Update local cache →
Update UI (show "Saved")
```

#### Save Flow (Offline)
```
User types → Debounced (2s) → Zustand state update →
React Query mutation → Network error →
Save to local SQLite → Add to sync queue →
Update UI (show "Offline - will sync") →
[Later: Network restored] → Process sync queue →
API requests → Update UI (show "Synced")
```

#### Sync Queue Schema
```sql
CREATE TABLE sync_queue (
  id TEXT PRIMARY KEY,
  entity_type TEXT NOT NULL, -- 'book', 'chapter'
  entity_id TEXT NOT NULL,
  operation TEXT NOT NULL,   -- 'create', 'update', 'delete'
  payload TEXT NOT NULL,      -- JSON
  retry_count INTEGER DEFAULT 0,
  created_at INTEGER NOT NULL,
  last_attempted_at INTEGER
);
```

### 5.4 Database Schema (Local SQLite)

```sql
-- Mirrors backend schema for offline access
CREATE TABLE books (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  title TEXT NOT NULL,
  description TEXT,
  genre TEXT,
  status TEXT DEFAULT 'draft',
  cover_url TEXT,
  word_count INTEGER DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  synced_at INTEGER,
  is_cached BOOLEAN DEFAULT 0
);

CREATE TABLE chapters (
  id TEXT PRIMARY KEY,
  book_id TEXT NOT NULL,
  chapter_number INTEGER NOT NULL,
  title TEXT,
  content TEXT,
  word_count INTEGER DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  synced_at INTEGER,
  FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE
);

CREATE TABLE drafts (
  chapter_id TEXT PRIMARY KEY,
  content TEXT NOT NULL,
  last_saved_at INTEGER NOT NULL,
  FOREIGN KEY (chapter_id) REFERENCES chapters(id) ON DELETE CASCADE
);

CREATE TABLE user_profile (
  user_id TEXT PRIMARY KEY,
  email TEXT,
  username TEXT,
  display_name TEXT,
  avatar_url TEXT,
  bio TEXT,
  subscription_tier TEXT
);

CREATE INDEX idx_chapters_book ON chapters(book_id);
CREATE INDEX idx_books_updated ON books(updated_at DESC);
```

### 5.5 API Integration

#### Endpoints Used by Mobile App

**Authentication**
- `POST /auth/login` - User login
- `POST /auth/logout` - User logout
- `POST /auth/refresh` - Refresh access token

**Dashboard**
- `GET /v1/dashboard/stats` - Get user statistics

**Books**
- `GET /v1/books` - List books (with pagination, filters)
- `POST /v1/books` - Create book
- `GET /v1/books/:id` - Get book details
- `PUT /v1/books/:id` - Update book
- `DELETE /v1/books/:id` - Delete book

**Chapters**
- `GET /v1/books/:bookId/chapters` - List chapters in book
- `POST /v1/books/:bookId/chapters` - Create chapter
- `GET /v1/chapters/:id` - Get chapter details
- `PUT /v1/chapters/:id` - Update chapter (content, title)
- `DELETE /v1/chapters/:id` - Delete chapter

**User**
- `GET /v1/user/profile` - Get user profile
- `PUT /v1/user/profile` - Update profile

**AI (Future)**
- `POST /v1/generate/enhance` - Enhance text with AI

#### Request/Response Examples

**Update Chapter (Save)**
```http
PUT /v1/chapters/abc-123
Authorization: Bearer eyJhbGc...
Content-Type: application/json

{
  "title": "Chapter 1: The Beginning",
  "content": "It was a dark and stormy night...",
  "word_count": 247
}

Response 200 OK:
{
  "id": "abc-123",
  "book_id": "book-456",
  "chapter_number": 1,
  "title": "Chapter 1: The Beginning",
  "content": "It was a dark and stormy night...",
  "word_count": 247,
  "updated_at": "2025-12-18T10:30:00Z"
}
```

**List Books (with filters)**
```http
GET /v1/books?status=writing&limit=20&offset=0
Authorization: Bearer eyJhbGc...

Response 200 OK:
{
  "books": [
    {
      "id": "book-456",
      "title": "My Great Novel",
      "description": "A story about...",
      "genre": "fantasy",
      "status": "writing",
      "cover_url": "https://cdn.authorworks.io/covers/book-456.jpg",
      "word_count": 25340,
      "created_at": "2025-11-01T08:00:00Z",
      "updated_at": "2025-12-18T09:45:00Z"
    }
  ],
  "total": 5,
  "limit": 20,
  "offset": 0
}
```

### 5.6 Offline Strategy

**Local-First Architecture:**
1. User actions write to local SQLite immediately
2. UI updates optimistically (show success before server confirms)
3. Background sync attempts API call
4. On success: Mark as synced, remove from queue
5. On failure: Retry with exponential backoff (1s, 2s, 4s, 8s, max 60s)
6. On connectivity change: Trigger immediate sync attempt

**Conflict Resolution (MVP):**
- Last-write-wins strategy
- Show notification if server data is newer: "Your chapter was edited on another device. Which version to keep?"
- Options: Keep mine, Keep server's, Show diff (future)

**Cache Eviction:**
- LRU (Least Recently Used) with 100 MB cap
- User can manually "pin" books for offline access (future)
- Auto-download last 3 edited books

### 5.7 Security Architecture

**Token Storage:**
- iOS: Store JWT in Keychain with `kSecAttrAccessible = WhenUnlockedThisDeviceOnly`
- Android: Store in EncryptedSharedPreferences

**Database Encryption:**
- iOS: SQLite file protected with FileProtection API
- Android: Use SQLCipher for encrypted database

**API Security:**
- All requests over HTTPS/TLS 1.3
- JWT bearer tokens in Authorization header
- Refresh token rotation every 7 days
- Auto-logout after 30 days of inactivity

**Code Security:**
- Obfuscate JavaScript bundle
- Enable SSL pinning (future - Phase 2)
- No sensitive data in logs (production builds)

---

## 6. User Interface Design

### 6.1 Design System

**Brand Colors (from Web)**
- Primary: Indigo `#6366F1` (indigo-500)
- Secondary: Purple `#A855F7` (purple-500)
- Accent: Pink `#EC4899` (pink-500)
- Success: Green `#10B981` (green-500)
- Background: Slate `#0F172A` (slate-950)
- Surface: Slate `#1E293B` (slate-900)
- Border: Slate `#334155` (slate-700)
- Text Primary: White `#FFFFFF`
- Text Secondary: Slate `#94A3B8` (slate-400)

**Typography**
- Headings: Playfair Display (serif, elegant)
- Body: System font (SF Pro on iOS, Roboto on Android)
- Editor: Georgia (serif, readable for long-form)

**Spacing**
- Base unit: 4px
- Scale: 4, 8, 12, 16, 24, 32, 48, 64

**Elevation (Shadows)**
- Card: `shadow-md` (0 4px 6px rgba(0,0,0,0.1))
- Modal: `shadow-xl` (0 20px 25px rgba(0,0,0,0.3))

### 6.2 Screen Layouts

#### Dashboard
```
┌────────────────────────────────────┐
│  [< Back]  Dashboard        [≡]   │ ← Header
├────────────────────────────────────┤
│  Welcome back, Claire              │
│  Here's what's happening...        │
│                                    │
│  ┌────────┬────────┬────────┐     │
│  │ 📚 12  │ 📝 45K │ ✨ 2K  │ ... │ ← Stats (horizontal scroll)
│  │ Books  │ Words  │ AI     │     │
│  └────────┴────────┴────────┘     │
│                                    │
│  Recent Books          [View all→] │
│  ┌────────────────────────────┐   │
│  │ 📖 [Cover] My Novel        │   │
│  │    "A story about..."      │   │
│  │    25,340 words • Writing  │   │ ← Book card (swipeable)
│  └────────────────────────────┘   │
│  ┌────────────────────────────┐   │
│  │ 📖 [Cover] Short Stories   │   │
│  │    "Collection of tales"   │   │
│  │    12,890 words • Draft    │   │
│  └────────────────────────────┘   │
│                                    │
│  [Empty space for scroll]          │
├────────────────────────────────────┤
│  [🏠] [📚] [✏️] [👤]            │ ← Bottom nav
└────────────────────────────────────┘
        [🔵] ← FAB (Quick Write)
```

#### Books List
```
┌────────────────────────────────────┐
│  [< Back]  My Books        [+ New] │
├────────────────────────────────────┤
│  ┌──────────────────────────────┐ │
│  │ 🔍 Search books...            │ │ ← Search bar
│  └──────────────────────────────┘ │
│  [All] [Draft] [Writing] [Editing]│ ← Filter chips
│                                    │
│  ┌────────────────────────────┐   │
│  │ 📖 My Novel           [⋮]  │   │ ← Swipe left for actions
│  │    "A story about love"    │   │
│  │    25,340 words • Writing  │   │
│  └────────────────────────────┘   │
│  ┌────────────────────────────┐   │
│  │ 📖 Short Stories      [⋮]  │   │
│  │    "Collection of tales"   │   │
│  │    12,890 words • Draft    │   │
│  └────────────────────────────┘   │
│                                    │
│  [Load more...]                    │
├────────────────────────────────────┤
│  [🏠] [📚] [✏️] [👤]            │
└────────────────────────────────────┘
```

#### Book Detail
```
┌────────────────────────────────────┐
│  [< Books]  My Novel        [⋮]   │
├────────────────────────────────────┤
│  ┌──────────────────────────────┐ │
│  │       [Cover Image]          │ │ ← Book header
│  │                              │ │
│  │  My Novel                    │ │
│  │  "A story about love and..." │ │
│  │  25,340 words • Writing      │ │
│  │                              │ │
│  │  [Edit Details] [+ Chapter]  │ │
│  └──────────────────────────────┘ │
│                                    │
│  Chapters (12)                     │
│  ┌────────────────────────────┐   │
│  │  1. The Beginning          │   │ ← Swipe left to delete
│  │     2,340 words • 2d ago   │   │
│  └────────────────────────────┘   │
│  ┌────────────────────────────┐   │
│  │  2. First Meeting          │   │
│  │     1,890 words • 4d ago   │   │
│  └────────────────────────────┘   │
│  ...                               │
├────────────────────────────────────┤
│  [🏠] [📚] [✏️] [👤]            │
└────────────────────────────────────┘
```

#### Editor
```
┌────────────────────────────────────┐
│  [< Back] Chapter 1    2,340 words │ ← Minimal header
├────────────────────────────────────┤
│                                    │
│                                    │
│  It was a dark and stormy night,  │
│  when the hero first arrived in   │
│  the village. The rain poured     │
│  down in sheets, obscuring...     │ ← Full-screen textarea
│                                    │
│  [Cursor blinking]                 │
│                                    │
│                                    │
│                                    │
│  [Lots of whitespace for writing] │
│                                    │
│                                    │
├────────────────────────────────────┤
│  [B][I][H1][H2][•]        [✨]   │ ← Formatting toolbar
└────────────────────────────────────┘
     [Saved at 3:42 PM] ← Toast notification
```

### 6.3 Navigation Structure

**Bottom Tab Navigation (Primary)**
- 🏠 **Dashboard** - Overview, stats, recent books
- 📚 **Books** - Browse all books
- ✏️ **Write** - Quick access to last chapter or create new
- 👤 **Profile** - User settings, stats, logout

**Modal Screens (Overlay)**
- Create Book
- Edit Book Details
- Add Chapter
- Settings
- About

**Full-Screen Flows**
- Editor (with back to book detail)
- Onboarding (first launch)
- Authentication (login/signup)

### 6.4 Gestures & Interactions

**Swipe Actions**
- Swipe left on book card → Delete, Edit buttons
- Swipe left on chapter → Delete button
- Swipe between chapters in editor (future)

**Pull-to-Refresh**
- Dashboard (refresh stats + recent books)
- Books list (refresh all books)
- Book detail (refresh chapters)

**Long Press**
- Long press on chapter → Reorder mode (future)
- Long press on book → Quick actions menu

**Haptic Feedback**
- Tap on FAB → Light impact
- Swipe to delete → Warning impact
- Save success → Success impact
- Error → Error notification impact

### 6.5 Responsive Breakpoints

**Phone (Default)**
- Width: 320px - 480px
- Single column layouts
- Bottom navigation
- Full-screen modals

**Tablet (iPad, large Android)**
- Width: 600px+
- Two-column layouts (e.g., books list + detail side-by-side)
- Side navigation (optional)
- Popovers instead of full-screen modals

**Landscape**
- Optimize editor for wider screen (max-width constraint)
- Show more stats on dashboard

---

## 7. Engineering Tasks Breakdown

### 7.1 Project Setup & Infrastructure

#### Task 1.1: Initialize Tauri 2.0 Mobile Project
**Owner:** Senior Engineer
**Estimate:** 4 hours
**Priority:** P0
**Dependencies:** None

**Acceptance Criteria:**
- Tauri 2.0 project created with `tauri init --mobile`
- iOS and Android build targets configured
- Verified iOS build on macOS with Xcode
- Verified Android build with Android Studio
- Basic "Hello World" app runs on both platforms

**Technical Notes:**
```bash
# Commands
npm create tauri-app@latest authorworks-mobile
cd authorworks-mobile
cargo install tauri-cli --version ^2.0
cargo tauri ios init
cargo tauri android init
cargo tauri ios dev  # Test iOS
cargo tauri android dev  # Test Android
```

**Subtasks:**
1. Install Tauri CLI 2.9.0+
2. Create project with React + TypeScript template
3. Configure iOS target (Xcode 14+, Swift 5)
4. Configure Android target (API 26+, Kotlin 1.8+)
5. Set up development certificates (iOS)
6. Test emulator builds (iOS Simulator, Android Emulator)

---

#### Task 1.2: Configure Build Pipeline (CI/CD)
**Owner:** DevOps Engineer / Senior Engineer
**Estimate:** 8 hours
**Priority:** P1
**Dependencies:** 1.1

**Acceptance Criteria:**
- GitHub Actions workflow builds iOS IPA
- GitHub Actions workflow builds Android APK/AAB
- Builds triggered on PR to `main`
- Artifacts uploaded to GitHub releases
- Secrets configured (signing certificates, API keys)

**Technical Notes:**
- Use `fastlane` for iOS code signing automation
- Use GitHub Actions for Android signing with keystore

**Subtasks:**
1. Create `.github/workflows/build-ios.yml`
2. Create `.github/workflows/build-android.yml`
3. Configure iOS code signing (match/fastlane)
4. Configure Android keystore (GitHub secrets)
5. Test builds on CI
6. Document build process in README

---

#### Task 1.3: Set Up Development Environment
**Owner:** Both Engineers
**Estimate:** 2 hours each
**Priority:** P0
**Dependencies:** 1.1

**Acceptance Criteria:**
- Both engineers can build and run iOS app
- Both engineers can build and run Android app
- Hot reload working for frontend changes
- Debugging enabled (Chrome DevTools, Rust debug logs)
- Shared ESLint + Prettier config

**Subtasks:**
1. Install Xcode (iOS engineer)
2. Install Android Studio (both)
3. Install Node.js 18+, Rust 1.75+
4. Clone repo, run `npm install`
5. Run dev commands, verify hot reload
6. Set up VSCode/Cursor with extensions (ESLint, Rust Analyzer)

---

### 7.2 Frontend Foundation

#### Task 2.1: Design System Implementation
**Owner:** Frontend Engineer
**Estimate:** 12 hours
**Priority:** P0
**Dependencies:** 1.1

**Acceptance Criteria:**
- Tailwind CSS configured with custom theme (brand colors)
- Shared component library: Button, Card, Input, Modal, BottomSheet
- Typography styles defined (Playfair Display, system fonts)
- Icon library integrated (Lucide React)
- Storybook or component showcase (optional)

**Technical Notes:**
- Match web design system (indigo/purple/slate palette)
- Use Tailwind utility classes for responsive design
- Components should be mobile-optimized (larger tap targets)

**Subtasks:**
1. Install Tailwind CSS, configure `tailwind.config.js`
2. Add custom colors to theme (indigo-500, purple-500, etc.)
3. Create `Button` component (primary, secondary, ghost variants)
4. Create `Card` component (with hover states)
5. Create `Input` component (with validation states)
6. Create `Modal` and `BottomSheet` components (mobile-optimized)
7. Add Playfair Display font (via Google Fonts or local)
8. Document components in README or Storybook

---

#### Task 2.2: Navigation Setup (Bottom Tabs + Stack)
**Owner:** Frontend Engineer
**Estimate:** 8 hours
**Priority:** P0
**Dependencies:** 2.1

**Acceptance Criteria:**
- Bottom tab navigation with 4 tabs: Dashboard, Books, Write, Profile
- Stack navigation for detail screens (e.g., Book Detail, Editor)
- Tab bar icons from Lucide React
- Active tab highlighted with accent color
- Navigation works on both iOS and Android

**Technical Notes:**
- Use React Navigation or similar (mobile-friendly)
- Bottom tabs should be fixed at bottom (iOS safe area aware)

**Subtasks:**
1. Install React Navigation (`@react-navigation/native`)
2. Set up Bottom Tab Navigator with 4 tabs
3. Create placeholder screens for each tab
4. Add stack navigator for Book Detail → Editor flow
5. Style tab bar (icons, colors, active state)
6. Test navigation on iOS and Android
7. Handle safe area insets (iOS notch, Android gesture bar)

---

#### Task 2.3: State Management Setup (Zustand)
**Owner:** Frontend Engineer
**Estimate:** 6 hours
**Priority:** P0
**Dependencies:** 2.1

**Acceptance Criteria:**
- Zustand store configured
- Slices for: Auth state, Books state, UI state (loading, errors)
- TypeScript types for all state
- Persist auth state to secure storage (via Tauri command)
- State accessible in all components via hooks

**Technical Notes:**
- Keep stores modular (separate files for auth, books, etc.)
- Use `persist` middleware for auth token

**Subtasks:**
1. Install Zustand
2. Create `stores/authStore.ts` (token, user, isAuthenticated)
3. Create `stores/booksStore.ts` (books list, selected book)
4. Create `stores/uiStore.ts` (global loading, toast messages)
5. Create hooks: `useAuth()`, `useBooks()`, `useUI()`
6. Implement persist middleware (store token in Tauri secure storage)
7. Test state updates across components

---

#### Task 2.4: API Client Setup (Axios/Fetch + React Query)
**Owner:** Frontend Engineer
**Estimate:** 8 hours
**Priority:** P0
**Dependencies:** 2.3

**Acceptance Criteria:**
- API client configured with base URL
- Request interceptor adds JWT to Authorization header
- Response interceptor handles 401 (token refresh)
- React Query configured with offline plugin
- Query cache persists to local storage (via Tauri)
- Error handling for network failures

**Technical Notes:**
- Base URL: `https://api.authorworks.io/v1` (replace with staging for dev)
- Use `@tanstack/react-query` v5
- Use `@tanstack/react-query-persist-client` for cache persistence

**Subtasks:**
1. Install Axios + React Query
2. Create `api/client.ts` with Axios instance
3. Add request interceptor (attach JWT from auth store)
4. Add response interceptor (handle 401, refresh token)
5. Configure React Query with offline plugin
6. Create query hooks: `useBooks()`, `useChapter()`, etc.
7. Test API calls with mock token
8. Handle offline mode (show cached data)

---

### 7.3 Authentication

#### Task 3.1: OAuth2/OIDC Integration (Logto)
**Owner:** Backend-focused Engineer
**Estimate:** 12 hours
**Priority:** P0
**Dependencies:** 2.3, 2.4

**Acceptance Criteria:**
- Logto SDK integrated for iOS and Android
- Login flow: Tap "Sign In" → Browser opens → User logs in → Redirected back to app
- JWT token stored securely (iOS Keychain, Android Keystore)
- Token refresh implemented (7-day rotation)
- Logout clears token and cached data
- Works on both iOS and Android

**Technical Notes:**
- Use Logto's mobile SDKs (or custom OAuth2 flow)
- Handle deep links for OAuth redirect (iOS: Universal Links, Android: App Links)

**Subtasks:**
1. Register mobile app in Logto console (get client ID, redirect URI)
2. Install Logto SDK (if available) or configure OAuth2 manually
3. Create Tauri command: `login_with_oauth` (opens browser, handles redirect)
4. Create Tauri command: `store_token_securely` (iOS Keychain, Android Keystore)
5. Create Tauri command: `get_stored_token`
6. Implement token refresh logic (React Query mutation)
7. Create login screen UI (button to trigger OAuth)
8. Test full login flow on iOS and Android

---

#### Task 3.2: Auth State Management
**Owner:** Frontend Engineer
**Estimate:** 4 hours
**Priority:** P0
**Dependencies:** 3.1

**Acceptance Criteria:**
- Auth store tracks: token, user profile, isAuthenticated
- On app launch, check for stored token and validate
- If token expired, attempt refresh
- If refresh fails, clear auth and show login
- Protected routes redirect to login if not authenticated

**Subtasks:**
1. Update `authStore.ts` with login/logout actions
2. Create `useAuth()` hook with helper methods
3. Add `AuthGuard` component (redirects to login if not authed)
4. Test auth flow: Login → Use app → Close app → Reopen (should be logged in)
5. Test token expiry: Manually expire token, verify refresh works

---

#### Task 3.3: Biometric Authentication (Optional - P1)
**Owner:** Backend-focused Engineer
**Estimate:** 8 hours
**Priority:** P1
**Dependencies:** 3.1

**Acceptance Criteria:**
- User can enable biometric auth in settings
- On app launch, prompt for Face ID/Touch ID if enabled
- Successful biometric auth loads cached token
- Failed biometric shows login screen
- Works on both iOS and Android

**Technical Notes:**
- iOS: Use LocalAuthentication framework (Swift)
- Android: Use BiometricPrompt API (Kotlin)

**Subtasks:**
1. Create Tauri command: `is_biometric_available` (checks device support)
2. Create Tauri command: `authenticate_biometric` (prompts user)
3. Add toggle in settings screen: "Enable Biometric Auth"
4. On app launch, check if biometric enabled → prompt → load token
5. Test on iOS (Face ID simulator)
6. Test on Android (fingerprint emulator)

---

### 7.4 Dashboard

#### Task 4.1: Dashboard Layout & Stats
**Owner:** Frontend Engineer
**Estimate:** 8 hours
**Priority:** P0
**Dependencies:** 2.1, 2.2, 2.4

**Acceptance Criteria:**
- Dashboard screen matches design (stats grid + recent books)
- Stats fetched from `/v1/dashboard/stats` endpoint
- Display 4 stat cards: Total Books, Words Written, AI Words, Streak
- Loading state (skeleton or spinner)
- Pull-to-refresh gesture
- Empty state if no books

**Subtasks:**
1. Create `screens/Dashboard.tsx`
2. Create `useStats()` React Query hook
3. Fetch stats from API on mount
4. Render stats grid (responsive, horizontal scroll on small screens)
5. Add pull-to-refresh (using React Native Gesture Handler or Tauri plugin)
6. Test loading, success, and error states
7. Test offline mode (show cached stats)

---

#### Task 4.2: Recent Books List
**Owner:** Frontend Engineer
**Estimate:** 6 hours
**Priority:** P0
**Dependencies:** 4.1

**Acceptance Criteria:**
- Display 5 most recent books below stats
- Each book shows: cover thumbnail, title, description, word count, status
- Tap on book navigates to Book Detail screen
- Swipe left for quick actions (Edit, Delete)
- Empty state: "Create your first book" CTA

**Subtasks:**
1. Create `useRecentBooks()` React Query hook
2. Fetch from `/v1/books?limit=5&sort=updated_at`
3. Create `BookCard` component (matches design)
4. Add swipe-to-delete gesture (using Gesture Handler)
5. Test navigation to book detail
6. Test empty state

---

#### Task 4.3: Quick Actions (FAB)
**Owner:** Frontend Engineer
**Estimate:** 4 hours
**Priority:** P1
**Dependencies:** 4.1

**Acceptance Criteria:**
- Floating action button (FAB) at bottom right
- Primary FAB: "Quick Write" → Opens last edited chapter in editor
- Secondary FAB (expandable): "New Book"
- Haptic feedback on tap
- Hides on scroll down, shows on scroll up

**Subtasks:**
1. Create `FAB` component (positioned fixed bottom right)
2. Add expand/collapse animation
3. Implement "Quick Write" action (fetch last chapter, navigate to editor)
4. Implement "New Book" action (open create book modal)
5. Add haptic feedback (via Tauri plugin)
6. Test FAB on scroll (hide/show animation)

---

### 7.5 Books Management

#### Task 5.1: Books List Screen
**Owner:** Frontend Engineer
**Estimate:** 10 hours
**Priority:** P0
**Dependencies:** 2.1, 2.2, 2.4

**Acceptance Criteria:**
- List all books with search and filter
- Search bar at top (searches title + description)
- Filter chips: All, Draft, Writing, Editing, Published
- List view (default): Cover + title + meta
- Infinite scroll pagination (20 books per page)
- Pull-to-refresh
- Swipe-to-delete with confirmation
- Empty state with CTA

**Subtasks:**
1. Create `screens/BooksList.tsx`
2. Create `useBooks()` React Query hook (with pagination)
3. Implement search (debounced, client-side for MVP)
4. Implement status filter (refetch with query param)
5. Create `BookListItem` component
6. Add swipe-to-delete gesture with confirmation dialog
7. Add infinite scroll (react-query infinite queries)
8. Test search, filter, pagination, delete

---

#### Task 5.2: Book Detail Screen
**Owner:** Frontend Engineer
**Estimate:** 10 hours
**Priority:** P0
**Dependencies:** 5.1

**Acceptance Criteria:**
- Display book metadata: cover, title, description, word count, status
- Action buttons: Edit Details, Add Chapter, Delete Book
- Chapter list: Numbered, title, word count, last edited
- Tap chapter → Navigate to editor
- Swipe chapter left → Delete with confirmation
- Drag-and-drop to reorder chapters (P1 - optional)

**Subtasks:**
1. Create `screens/BookDetail.tsx`
2. Create `useBook(id)` React Query hook
3. Fetch book details and chapters from `/v1/books/:id`
4. Render book header (cover, title, meta)
5. Render chapter list with `ChapterListItem` component
6. Add tap-to-edit navigation
7. Add swipe-to-delete chapter (with confirmation)
8. Add "Add Chapter" button → Open modal
9. Test navigation, delete, empty state

---

#### Task 5.3: Create Book Modal
**Owner:** Frontend Engineer
**Estimate:** 8 hours
**Priority:** P0
**Dependencies:** 2.1, 5.1

**Acceptance Criteria:**
- Modal or full-screen form to create book
- Fields: Title (required), Description (optional), Genre (picker)
- Genre picker: Modal select with options (Fantasy, Sci-Fi, Romance, etc.)
- Validation: Title max 200 chars, Description max 1000 chars
- On success: Close modal, navigate to new book detail
- Error handling (show toast)

**Subtasks:**
1. Create `modals/CreateBookModal.tsx`
2. Create form with validation (react-hook-form or native)
3. Create genre picker (modal select or dropdown)
4. Create `useCreateBook()` React Query mutation
5. POST to `/v1/books` with form data
6. Handle success: invalidate books query, navigate to book detail
7. Handle error: show error toast
8. Test validation, success, error flows

---

#### Task 5.4: Edit Book Metadata
**Owner:** Frontend Engineer
**Estimate:** 6 hours
**Priority:** P0
**Dependencies:** 5.2, 5.3

**Acceptance Criteria:**
- Modal to edit book title, description, genre, status
- Pre-populate with current values
- Same validation as create
- On success: Update UI optimistically
- Delete book option with confirmation

**Subtasks:**
1. Create `modals/EditBookModal.tsx` (reuse CreateBookModal logic)
2. Create `useUpdateBook()` React Query mutation
3. PUT to `/v1/books/:id` with updated fields
4. Add "Delete Book" button with confirmation dialog
5. Create `useDeleteBook()` mutation (DELETE `/v1/books/:id`)
6. Handle success: invalidate queries, navigate back
7. Test edit and delete flows

---

### 7.6 Chapter Management & Editor

#### Task 6.1: Editor Screen - Basic Layout
**Owner:** Frontend Engineer
**Estimate:** 8 hours
**Priority:** P0
**Dependencies:** 2.1, 2.2

**Acceptance Criteria:**
- Full-screen editor with minimal chrome
- Top bar: Back button, chapter title (editable), word count
- Main area: Large textarea for content
- Auto-resize textarea to fit content
- No bottom nav (full writing focus)
- Matches design (Georgia font, dark mode)

**Subtasks:**
1. Create `screens/Editor.tsx`
2. Create layout with header + textarea
3. Add editable chapter title (inline input)
4. Display word count (updates as user types)
5. Style textarea (Georgia font, line height, padding)
6. Test on multiple screen sizes (iPhone SE to iPad)

---

#### Task 6.2: Editor - Content Loading & Saving
**Owner:** Backend-focused Engineer
**Estimate:** 12 hours
**Priority:** P0
**Dependencies:** 6.1, 2.4

**Acceptance Criteria:**
- Fetch chapter content from `/v1/chapters/:id` on mount
- Display loading spinner while fetching
- Populate textarea with content
- Auto-save after 2 seconds of inactivity (debounced)
- Save sends PUT request to `/v1/chapters/:id`
- Show save status: "Saving...", "Saved at 3:42 PM"
- Manual save button (Cmd+S / Ctrl+S)

**Subtasks:**
1. Create `useChapter(id)` React Query hook
2. Fetch chapter on mount, populate textarea
3. Create `useSaveChapter()` mutation (PUT `/v1/chapters/:id`)
4. Implement debounced auto-save (use-debounce or lodash.debounce)
5. Add save status indicator in top bar
6. Add keyboard shortcut listener (Cmd+S / Ctrl+S)
7. Test auto-save, manual save, loading states

---

#### Task 6.3: Editor - Offline Mode & Local Draft
**Owner:** Backend-focused Engineer
**Estimate:** 16 hours
**Priority:** P0
**Dependencies:** 6.2, 7.1 (SQLite setup)

**Acceptance Criteria:**
- User can write offline (no network errors)
- Content saves to local SQLite immediately
- Sync queue tracks pending saves
- When online, sync queue processes automatically
- Visual indicator: "Offline - changes will sync"
- On sync success, update "Saved at..." timestamp
- Handle conflicts: Last-write-wins (show notification)

**Technical Notes:**
- This is the most complex task; requires solid Rust + SQLite skills

**Subtasks:**
1. Create Tauri command: `save_chapter_local(chapter_id, content)`
2. Update SQLite `drafts` table on local save
3. Add to `sync_queue` table if offline
4. Create Tauri command: `process_sync_queue()` (sends queued saves to API)
5. Listen for connectivity changes (Tauri event or plugin)
6. On connectivity restored, trigger `process_sync_queue()`
7. Handle sync errors with retry logic (exponential backoff)
8. Detect conflicts (server `updated_at` > local `updated_at`)
9. Show notification on conflict: "Chapter updated on another device"
10. Test offline write → close app → reopen → sync
11. Test conflict scenario (edit on web + mobile simultaneously)

---

#### Task 6.4: Editor - Text Formatting Toolbar
**Owner:** Frontend Engineer
**Estimate:** 10 hours
**Priority:** P0
**Dependencies:** 6.1

**Acceptance Criteria:**
- Formatting toolbar appears on text selection (or fixed at bottom)
- Buttons: Bold, Italic, H1, H2, Bulleted List
- Apply Markdown formatting (e.g., `**bold**`, `# Heading`)
- Keyboard shortcuts: Cmd+B (bold), Cmd+I (italic)
- Undo/Redo buttons (use browser/native undo stack)

**Technical Notes:**
- Formatting can be Markdown-based (simpler) or rich text editor (complex)
- For MVP, Markdown is sufficient

**Subtasks:**
1. Create `Toolbar` component (fixed at bottom or contextual)
2. Implement bold: Wrap selection with `**text**`
3. Implement italic: Wrap selection with `*text*`
4. Implement headings: Add `# ` or `## ` at line start
5. Implement lists: Add `- ` at line start
6. Add keyboard shortcut listeners (Cmd+B, Cmd+I)
7. Test formatting on iOS and Android

---

#### Task 6.5: Create Chapter Flow
**Owner:** Frontend Engineer
**Estimate:** 6 hours
**Priority:** P0
**Dependencies:** 5.2

**Acceptance Criteria:**
- From Book Detail, tap "Add Chapter" → Modal appears
- Modal has field: Chapter Title (optional, defaults to "Chapter N")
- On create: POST to `/v1/books/:bookId/chapters`
- On success: Close modal, navigate to editor for new chapter
- Error handling

**Subtasks:**
1. Create `modals/CreateChapterModal.tsx`
2. Create form with optional title field
3. Create `useCreateChapter()` mutation (POST `/v1/books/:bookId/chapters`)
4. Handle success: invalidate chapters query, navigate to editor
5. Handle error: show toast
6. Test create flow from book detail

---

### 7.7 Local Database & Sync

#### Task 7.1: SQLite Database Setup (Tauri Plugin)
**Owner:** Backend-focused Engineer
**Estimate:** 10 hours
**Priority:** P0
**Dependencies:** 1.1

**Acceptance Criteria:**
- Tauri SQLite plugin installed and configured
- Database schema created (books, chapters, drafts, sync_queue, user_profile)
- Tauri commands for CRUD operations: `get_books_local()`, `save_book_local()`, etc.
- Database file location: iOS (Documents), Android (App Data)
- Database encrypted on iOS and Android

**Technical Notes:**
- Use `tauri-plugin-sql` or custom Rust + SQLite integration
- Encryption: iOS (FileProtection), Android (SQLCipher)

**Subtasks:**
1. Install `tauri-plugin-sql` or add `rusqlite` dependency
2. Create `src-tauri/src/db.rs` module for database logic
3. Define schema (see Section 5.4 Database Schema)
4. Create Tauri command: `init_database()` (creates tables)
5. Create CRUD commands for each entity
6. Test database creation on iOS and Android
7. Test queries (insert, select, update, delete)
8. Enable encryption (iOS: NSFileProtection, Android: SQLCipher)

---

#### Task 7.2: Sync Queue Implementation
**Owner:** Backend-focused Engineer
**Estimate:** 14 hours
**Priority:** P0
**Dependencies:** 7.1, 6.3

**Acceptance Criteria:**
- When offline save occurs, add entry to `sync_queue` table
- Sync queue processor runs periodically (every 5 min) or on connectivity change
- For each queued item: Attempt API request
- On success: Remove from queue, update local entity, mark synced
- On failure: Increment retry count, apply exponential backoff
- Max retries: 5 (then mark as failed, notify user)

**Subtasks:**
1. Create Tauri command: `add_to_sync_queue(entity_type, entity_id, operation, payload)`
2. Create Tauri command: `process_sync_queue()` (processes all queued items)
3. Implement retry logic with exponential backoff (1s, 2s, 4s, 8s, 16s, max 60s)
4. On sync success: DELETE from sync_queue, UPDATE entity with `synced_at`
5. On sync failure: UPDATE retry_count, last_attempted_at
6. Create Tauri event listener for connectivity changes (trigger sync)
7. Create periodic sync timer (every 5 min if app active)
8. Test offline → online → sync queue processes automatically
9. Test sync failures (mock API errors)

---

#### Task 7.3: Conflict Resolution
**Owner:** Backend-focused Engineer
**Estimate:** 10 hours
**Priority:** P1 (MVP can use last-write-wins)
**Dependencies:** 7.2

**Acceptance Criteria:**
- Detect conflicts: Local `updated_at` < Server `updated_at`
- For MVP: Last-write-wins (server data overwrites local)
- Show notification: "Chapter updated on another device"
- Future: Show diff, let user choose version

**Subtasks:**
1. On sync, compare local `updated_at` vs server `updated_at`
2. If server is newer: Show notification, overwrite local with server data
3. If local is newer: Proceed with sync (PUT to server)
4. Create Tauri command: `resolve_conflict(entity_id, resolution)` (future)
5. Test conflict scenario (edit on web + mobile)

---

### 7.8 Settings & Profile

#### Task 8.1: Profile Screen
**Owner:** Frontend Engineer
**Estimate:** 6 hours
**Priority:** P0
**Dependencies:** 2.2, 3.1

**Acceptance Criteria:**
- Display user profile: avatar, username, email, bio
- "Edit Profile" button → Modal to edit bio, website, social links
- View subscription tier (read-only)
- Logout button → Clears auth, navigates to login
- Matches web design

**Subtasks:**
1. Create `screens/Profile.tsx`
2. Fetch user profile from `/v1/user/profile`
3. Display profile info
4. Create "Edit Profile" modal (reuse form components)
5. Create `useUpdateProfile()` mutation (PUT `/v1/user/profile`)
6. Add logout button → Clear auth store → Navigate to login
7. Test edit and logout flows

---

#### Task 8.2: Settings Screen
**Owner:** Frontend Engineer
**Estimate:** 8 hours
**Priority:** P1
**Dependencies:** 8.1

**Acceptance Criteria:**
- Settings screen with sections: Appearance, Editor, Notifications, Data
- Appearance: Dark mode toggle (system default for MVP)
- Editor: Font size (Small, Medium, Large), Auto-save interval
- Notifications: Enable/disable writing streak reminders (future)
- Data: Clear cache button, Cache size display
- All settings persist to local storage

**Subtasks:**
1. Create `screens/Settings.tsx`
2. Create settings form with toggle switches, pickers
3. Persist settings to local storage (Tauri commands)
4. Implement "Clear cache" button → DELETE all cached data from SQLite
5. Display cache size (query SQLite database file size)
6. Test all settings (save, persist, reload)

---

### 7.9 Native Platform Features

#### Task 9.1: iOS-Specific Integrations
**Owner:** Backend-focused Engineer (Swift knowledge)
**Estimate:** 12 hours
**Priority:** P1
**Dependencies:** 1.1

**Acceptance Criteria:**
- Secure token storage (iOS Keychain)
- Biometric authentication (Face ID, Touch ID)
- Haptic feedback (via Tauri plugin or Swift command)
- Share sheet integration (share book as text file - future)
- System keyboard with suggestions

**Subtasks:**
1. Create Swift function: `saveToKeychain(key, value)` → Tauri command
2. Create Swift function: `getFromKeychain(key)` → Tauri command
3. Create Swift function: `authenticateBiometric()` → Tauri command (LocalAuthentication)
4. Create Swift function: `triggerHaptic(type)` → Tauri command (UIImpactFeedbackGenerator)
5. Test all functions on iOS device/simulator
6. Document Swift bridge in README

---

#### Task 9.2: Android-Specific Integrations
**Owner:** Backend-focused Engineer (Kotlin knowledge)
**Estimate:** 12 hours
**Priority:** P1
**Dependencies:** 1.1

**Acceptance Criteria:**
- Secure token storage (Android Keystore)
- Biometric authentication (Fingerprint, Face Unlock)
- Haptic feedback (via Tauri plugin or Kotlin command)
- Share sheet integration (share book as text file - future)
- System keyboard with suggestions

**Subtasks:**
1. Create Kotlin function: `saveToKeystore(key, value)` → Tauri command
2. Create Kotlin function: `getFromKeystore(key)` → Tauri command
3. Create Kotlin function: `authenticateBiometric()` → Tauri command (BiometricPrompt)
4. Create Kotlin function: `triggerHaptic(type)` → Tauri command (Vibrator)
5. Test all functions on Android device/emulator
6. Document Kotlin bridge in README

---

### 7.10 Testing & QA

#### Task 10.1: Unit Tests (Frontend)
**Owner:** Frontend Engineer
**Estimate:** 16 hours
**Priority:** P1
**Dependencies:** All frontend tasks

**Acceptance Criteria:**
- 80% coverage for critical paths: Auth, Save, Sync
- Test React components with React Testing Library
- Test React Query hooks
- Test Zustand stores
- CI runs tests on every PR

**Subtasks:**
1. Set up Jest + React Testing Library
2. Write tests for auth store (login, logout, token refresh)
3. Write tests for books store (CRUD operations)
4. Write tests for API client (interceptors, error handling)
5. Write tests for key components (Button, BookCard, Editor)
6. Add test script to `package.json` (`npm test`)
7. Configure CI to run tests

---

#### Task 10.2: Integration Tests (Rust)
**Owner:** Backend-focused Engineer
**Estimate:** 12 hours
**Priority:** P1
**Dependencies:** All Rust tasks

**Acceptance Criteria:**
- Test Tauri commands (database CRUD, sync queue)
- Test SQLite operations (insert, query, update, delete)
- Test sync queue logic (retry, backoff)
- CI runs tests on every PR

**Subtasks:**
1. Set up Rust test framework (built-in `cargo test`)
2. Write tests for database commands (`save_book_local`, `get_books_local`)
3. Write tests for sync queue (`add_to_sync_queue`, `process_sync_queue`)
4. Write tests for conflict resolution
5. Mock API responses for sync tests
6. Add test command to CI workflow

---

#### Task 10.3: End-to-End Tests (Optional - P2)
**Owner:** Both Engineers
**Estimate:** 20 hours
**Priority:** P2 (post-MVP)
**Dependencies:** All tasks

**Acceptance Criteria:**
- E2E tests for critical user journeys
- Test: Login → Create book → Add chapter → Write → Save → Sync
- Test: Offline write → Close app → Reopen → Sync
- Use Detox or Appium for mobile E2E testing

**Subtasks:**
1. Set up Detox (React Native) or Appium (cross-platform)
2. Write E2E test: Full book creation flow
3. Write E2E test: Offline editing + sync
4. Write E2E test: Conflict resolution
5. Run E2E tests in CI (on simulators/emulators)

---

#### Task 10.4: Manual QA & Device Testing
**Owner:** Both Engineers
**Estimate:** 16 hours
**Priority:** P0
**Dependencies:** All feature tasks

**Acceptance Criteria:**
- Test app on at least 3 iOS devices (iPhone SE, iPhone 14, iPad)
- Test app on at least 3 Android devices (various screen sizes, API levels)
- Test all critical flows (auth, create book, edit chapter, sync)
- Document bugs in GitHub Issues
- Verify accessibility (VoiceOver, TalkBack)

**Subtasks:**
1. Create manual QA checklist (Google Sheet or GitHub Issue template)
2. Test on iOS devices (physical or TestFlight)
3. Test on Android devices (physical or Firebase Test Lab)
4. Test edge cases (no network, slow network, app backgrounding)
5. Test accessibility (screen readers, font scaling)
6. Log bugs in GitHub Issues
7. Fix P0 bugs before launch

---

### 7.11 Deployment & Launch

#### Task 11.1: App Store Submission (iOS)
**Owner:** Senior Engineer / PM
**Estimate:** 12 hours
**Priority:** P0
**Dependencies:** All tasks, 10.4 (QA passed)

**Acceptance Criteria:**
- App Store Connect account set up
- App metadata: Name, description, screenshots, keywords
- Privacy policy URL (required)
- App binary uploaded via Xcode or fastlane
- Submitted for review
- App approved and live on App Store

**Subtasks:**
1. Create App Store Connect record
2. Generate app icons (1024x1024 + all sizes)
3. Take screenshots (6.5", 5.5", 12.9" iPad)
4. Write app description (focus on benefits, features)
5. Set keywords for SEO (author, writing, AI, book)
6. Upload privacy policy to website
7. Build release IPA (via CI or Xcode)
8. Upload via Xcode or Transporter
9. Submit for review
10. Respond to review feedback (if any)
11. Release app (manual or automatic)

---

#### Task 11.2: Google Play Submission (Android)
**Owner:** Senior Engineer / PM
**Estimate:** 10 hours
**Priority:** P0
**Dependencies:** All tasks, 10.4 (QA passed)

**Acceptance Criteria:**
- Google Play Console account set up
- App metadata: Name, description, screenshots, keywords
- Privacy policy URL (required)
- App bundle (AAB) uploaded
- Submitted for review
- App approved and live on Google Play

**Subtasks:**
1. Create Google Play Console record
2. Generate app icons and feature graphic
3. Take screenshots (phone, tablet)
4. Write app description (same as iOS, adapted)
5. Set keywords/categories
6. Upload privacy policy
7. Build release AAB (via CI or Android Studio)
8. Upload AAB to Google Play Console
9. Fill out content rating questionnaire
10. Submit for review
11. Release to production

---

#### Task 11.3: Beta Testing (TestFlight + Firebase App Distribution)
**Owner:** Both Engineers
**Estimate:** 8 hours
**Priority:** P1
**Dependencies:** All feature tasks

**Acceptance Criteria:**
- TestFlight set up for iOS beta testers
- Firebase App Distribution set up for Android beta testers
- Invite 10-20 beta testers
- Collect feedback via form or email
- Fix critical bugs before public launch

**Subtasks:**
1. Set up TestFlight in App Store Connect
2. Upload beta build to TestFlight
3. Invite beta testers (email or public link)
4. Set up Firebase App Distribution for Android
5. Upload APK/AAB to Firebase
6. Invite beta testers
7. Create feedback form (Google Form or Typeform)
8. Monitor feedback, fix critical bugs
9. Push updated beta builds as needed

---

## 8. Timeline & Milestones

### 8.1 Development Phases

#### Phase 1: Foundation (Weeks 1-3)
**Goal:** Set up infrastructure and core tech stack

**Milestones:**
- M1.1: Tauri project initialized, builds on iOS and Android ✅
- M1.2: CI/CD pipeline configured ✅
- M1.3: Design system and navigation implemented ✅
- M1.4: State management and API client configured ✅

**Tasks:** 1.1, 1.2, 1.3, 2.1, 2.2, 2.3, 2.4

---

#### Phase 2: Authentication & Core Screens (Weeks 4-6)
**Goal:** Users can log in and view dashboard/books

**Milestones:**
- M2.1: OAuth authentication working ✅
- M2.2: Dashboard screen complete ✅
- M2.3: Books list and detail screens complete ✅
- M2.4: Create book flow working ✅

**Tasks:** 3.1, 3.2, 4.1, 4.2, 5.1, 5.2, 5.3, 5.4

---

#### Phase 3: Editor & Offline (Weeks 7-10)
**Goal:** Core writing experience with offline support

**Milestones:**
- M3.1: Editor screen functional (load, edit, save) ✅
- M3.2: Auto-save and manual save working ✅
- M3.3: SQLite database and sync queue implemented ✅
- M3.4: Offline editing with background sync ✅

**Tasks:** 6.1, 6.2, 6.3, 6.4, 6.5, 7.1, 7.2, 7.3

---

#### Phase 4: Polish & Platform Features (Weeks 11-12)
**Goal:** Native integrations and UI polish

**Milestones:**
- M4.1: iOS-specific features complete (Keychain, Haptics) ✅
- M4.2: Android-specific features complete (Keystore, Haptics) ✅
- M4.3: Settings and profile screens complete ✅
- M4.4: UI polish, animations, error states ✅

**Tasks:** 8.1, 8.2, 9.1, 9.2, 4.3 (FAB)

---

#### Phase 5: Testing & QA (Weeks 13-14)
**Goal:** Comprehensive testing and bug fixing

**Milestones:**
- M5.1: Unit and integration tests complete ✅
- M5.2: Manual QA on 6+ devices complete ✅
- M5.3: Beta testing with 10-20 users ✅
- M5.4: All P0 and P1 bugs fixed ✅

**Tasks:** 10.1, 10.2, 10.4, 11.3

---

#### Phase 6: Launch (Weeks 15-16)
**Goal:** Apps live on App Store and Google Play

**Milestones:**
- M6.1: App Store submission approved ✅
- M6.2: Google Play submission approved ✅
- M6.3: Marketing assets ready (blog post, social media) ✅
- M6.4: Apps live and accessible to users 🚀

**Tasks:** 11.1, 11.2

---

### 8.2 Team Allocation

**Team Composition:**
- 1 Senior Engineer (Full-stack, Rust + React + Swift/Kotlin)
- 1 Frontend Engineer (React + TypeScript, mobile UX)

**Workload Distribution:**

| Phase   | Senior Engineer                          | Frontend Engineer                        |
|---------|------------------------------------------|------------------------------------------|
| 1-3     | Tauri setup, SQLite, Auth, Sync queue    | Design system, Navigation, Screens       |
| 4-6     | Offline sync, Conflict resolution        | Editor UI, Books management              |
| 7-10    | Platform bridges (Swift/Kotlin)          | Settings, Profile, Polish                |
| 11-12   | Testing (Rust), Beta testing             | Testing (React), QA                      |
| 13-16   | App Store submission, DevOps             | Google Play submission, Documentation    |

---

## 9. Risks & Mitigation

### 9.1 Technical Risks

#### Risk 1: Tauri 2.0 Mobile Maturity
**Description:** Tauri 2.0 mobile is relatively new (released 2024). May encounter undocumented bugs or missing features.

**Likelihood:** Medium
**Impact:** High
**Mitigation:**
- Prototype early (Week 1) to validate core functionality
- Join Tauri Discord for community support
- Contribute fixes upstream if needed
- Have fallback: Use React Native if Tauri blocks progress (major pivot)

---

#### Risk 2: Offline Sync Complexity
**Description:** Implementing robust offline sync with conflict resolution is complex and error-prone.

**Likelihood:** High
**Impact:** High
**Mitigation:**
- Start with simple last-write-wins strategy (MVP)
- Defer advanced conflict resolution to Phase 2
- Add extensive logging for debugging
- Test offline scenarios early and often
- Use SQLite transactions for data integrity

---

#### Risk 3: iOS Code Signing & Provisioning
**Description:** iOS code signing is notoriously difficult; can block development and deployment.

**Likelihood:** Medium
**Impact:** Medium
**Mitigation:**
- Use fastlane match for automated certificate management
- Senior engineer handles iOS setup (must have Mac + Xcode)
- Document process in README for future reference
- Use TestFlight early to validate deployment pipeline

---

#### Risk 4: Performance on Low-End Devices
**Description:** App may be slow on older Android devices or iPhones (e.g., iPhone SE 2016, Android API 26).

**Likelihood:** Medium
**Impact:** Medium
**Mitigation:**
- Profile early on target devices (iPhone SE, budget Android)
- Optimize bundle size (code splitting, lazy loading)
- Use virtualized lists for long book/chapter lists
- Set minimum device requirements if necessary

---

#### Risk 5: API Compatibility Issues
**Description:** Backend APIs may not be fully compatible with mobile use cases (e.g., pagination, filtering).

**Likelihood:** Low
**Impact:** Medium
**Mitigation:**
- Review API documentation early (Week 1)
- Request API changes if needed (coordinate with backend team)
- Implement client-side workarounds if backend changes delayed
- Use API mocking for development (MSW or similar)

---

### 9.2 Schedule Risks

#### Risk 6: Scope Creep
**Description:** Stakeholders request additional features mid-development, delaying MVP.

**Likelihood:** High
**Impact:** High
**Mitigation:**
- Lock MVP scope in this PRD (get sign-off)
- Create Phase 2 backlog for nice-to-have features
- Use prioritization framework (P0, P1, P2) to triage requests
- PM to manage stakeholder expectations

---

#### Risk 7: Dependency on Single Engineer for Swift/Kotlin
**Description:** If senior engineer is unavailable, platform-specific work blocks.

**Likelihood:** Low
**Impact:** High
**Mitigation:**
- Cross-train frontend engineer on basic Swift/Kotlin
- Document all platform bridges thoroughly
- Use pair programming for knowledge transfer
- Have backup contractor on standby

---

#### Risk 8: App Store/Google Play Rejections
**Description:** Apps may be rejected due to policy violations or technical issues.

**Likelihood:** Medium
**Impact:** Medium
**Mitigation:**
- Review App Store and Google Play guidelines early
- Ensure privacy policy and data handling are compliant
- Test IAP (if applicable) in sandbox before submission
- Submit early (Week 15) to allow time for revisions
- Have legal review privacy policy (data collection, GDPR)

---

### 9.3 Business Risks

#### Risk 9: Low User Adoption
**Description:** Users may not install mobile app or prefer web interface.

**Likelihood:** Medium
**Impact:** High
**Mitigation:**
- Conduct user research before launch (surveys, interviews)
- Offer mobile-exclusive features (e.g., offline mode, push notifications)
- Promote app prominently in web dashboard ("Get the mobile app")
- Track metrics (install rate, DAU, retention) and iterate
- Offer incentives (e.g., free AI words for mobile users)

---

#### Risk 10: Platform Policy Changes
**Description:** Apple or Google may change policies (e.g., IAP requirements, privacy rules).

**Likelihood:** Low
**Impact:** Medium
**Mitigation:**
- Stay updated on platform announcements (WWDC, Google I/O)
- Avoid contentious features (e.g., external payment links)
- Be prepared to adapt quickly to policy changes
- Use feature flags to disable features if needed

---

## 10. Success Criteria & KPIs

### 10.1 Launch Criteria (Go/No-Go)

**Must-Have for Launch:**
- ✅ All P0 features implemented and tested
- ✅ No P0 bugs (app-breaking, data loss, security issues)
- ✅ App builds and runs on iOS 13+ and Android 8.0+
- ✅ Offline mode works reliably (no data loss)
- ✅ App Store and Google Play submissions approved
- ✅ Privacy policy published and linked
- ✅ Crash rate < 1% (based on beta testing)

**Nice-to-Have (Can defer to post-launch):**
- P1 features (Reading mode, Biometric auth, AI enhancement)
- P2 features (Advanced conflict resolution, Push notifications)
- E2E test suite
- Accessibility audit (WCAG 2.1 AA)

---

### 10.2 Key Performance Indicators (KPIs)

**Acquisition (First 60 days)**
- Mobile app installs: 30% of active web users
- App Store rating: 4.0+ stars (50+ reviews)
- Google Play rating: 4.0+ stars (50+ reviews)

**Engagement**
- Daily Active Users (DAU): 20% of installers
- Average session duration: 10+ minutes
- Sessions per user per week: 3+
- Mobile-originated content: 15% of total platform words

**Retention**
- Day 7 retention: 40%
- Day 30 retention: 25%
- Mobile users show 25% higher retention than web-only users

**Monetization (Future)**
- Conversion to paid tiers: 10% of mobile users
- Mobile-driven revenue: 20% of total

**Technical Health**
- Crash-free sessions: 99.5%+
- App launch time: < 3 seconds (cold start)
- Sync success rate: 99.9%
- API error rate: < 1%

---

## 11. Post-MVP Roadmap (Phase 2)

### 11.1 Priority Enhancements

**P1 Features (Next 3 months)**
- Push notifications (writing streak reminders)
- Voice-to-text dictation (mobile-native)
- AI chapter generation (mobile-optimized)
- Reading mode (distraction-free)
- Biometric authentication (Face ID, Touch ID)
- Advanced conflict resolution (show diff, choose version)
- Chapter reordering (drag-and-drop)
- Export to PDF/EPUB

**P2 Features (Next 6 months)**
- Collaborative editing (real-time)
- Comments and annotations
- Dark/light theme toggle (user preference)
- Home screen widgets (iOS 14+, Android 12+)
- Siri Shortcuts (iOS) and Google Assistant actions (Android)
- Apple Pencil support (iPad)
- Split-screen multitasking (iPad)

**Future Exploration**
- Apple Watch app (writing streak, quick notes)
- Android Wear app
- CarPlay integration (dictation while driving)
- Localization (Spanish, French, German, etc.)

---

## 12. Appendices

### 12.1 Glossary

- **Tauri:** Cross-platform app framework using Rust backend + web frontend
- **OAuth2/OIDC:** Open standards for authentication and authorization
- **Logto:** Identity management platform (used by AuthorWorks)
- **JWT:** JSON Web Token (used for API authentication)
- **SQLite:** Embedded relational database (used for local storage)
- **Zustand:** Lightweight state management library for React
- **React Query:** Data fetching and caching library for React
- **FAB:** Floating Action Button (mobile UI pattern)
- **P0/P1/P2:** Priority levels (P0 = Must have, P1 = Should have, P2 = Nice to have)
- **LRU:** Least Recently Used (cache eviction strategy)
- **AAB:** Android App Bundle (Google Play deployment format)
- **IPA:** iOS App Archive (App Store deployment format)

---

### 12.2 References

**Tauri Documentation:**
- [Tauri 2.0 Official Site](https://v2.tauri.app/)
- [Tauri Mobile Development Guide](https://v2.tauri.app/develop/)
- [Tauri Mobile Plugin Development](https://v2.tauri.app/develop/plugins/develop-mobile/)

**Platform Guidelines:**
- [iOS Human Interface Guidelines](https://developer.apple.com/design/human-interface-guidelines/)
- [Material Design for Android](https://m3.material.io/)

**Backend API:**
- AuthorWorks API Documentation (internal)
- [Logto Documentation](https://docs.logto.io/)

**Libraries & Tools:**
- [React Navigation](https://reactnavigation.org/)
- [TanStack Query (React Query)](https://tanstack.com/query/)
- [Zustand](https://github.com/pmndrs/zustand)
- [Tailwind CSS](https://tailwindcss.com/)

---

### 12.3 Design Mockups

**Note:** Detailed Figma mockups should be created by a designer based on the wireframes in Section 6.2. This PRD provides layout specifications, but high-fidelity designs are recommended before implementation.

**Mockup Requirements:**
- All screens in light and dark mode
- Responsive layouts (phone, tablet)
- Interactive prototypes for user testing
- Exported assets (icons, images)

---

### 12.4 Change Log

| Version | Date       | Author     | Changes                                  |
|---------|------------|------------|------------------------------------------|
| 1.0     | 2025-12-18 | AI Team    | Initial PRD draft for review             |

---

## 13. Approval & Sign-Off

**Stakeholders:**
- [ ] Product Manager: _____________________ Date: _______
- [ ] Engineering Lead: ___________________ Date: _______
- [ ] Design Lead: _______________________ Date: _______
- [ ] CEO/Founder: ______________________ Date: _______

**Notes:**
This PRD is a living document. Changes should be tracked via version control (Git) and communicated to all stakeholders. Major scope changes require re-approval.

---

**End of PRD**

**Next Steps:**
1. Review PRD with stakeholders (design, engineering, PM)
2. Refine estimates and timeline based on team feedback
3. Create Figma mockups (based on wireframes in Section 6)
4. Set up project management board (GitHub Projects, Jira, Linear)
5. Kick off Sprint 0 (Week 1): Environment setup and prototyping
6. Begin Phase 1 development (Weeks 1-3)

---

**Questions or Feedback?**
Contact: [Product Manager Email] or [Engineering Lead Email]

**Project Repository:** `https://github.com/authorworks/authorworks-mobile`
**Documentation:** `https://docs.authorworks.io/mobile`
**Slack Channel:** `#authorworks-mobile`
