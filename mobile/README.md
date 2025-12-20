# AuthorWorks Mobile

Native iOS and Android mobile applications for AuthorWorks, built with Tauri 2.0, React, and TypeScript.

## Features

### ✅ Implemented (MVP)

- **Authentication**: OAuth2/OIDC login via Logto
- **Dashboard**: Statistics overview, recent books, quick actions
- **Book Management**: Create, edit, delete, search, and filter books
- **Chapter Management**: Create, edit, delete chapters
- **Mobile Editor**: Distraction-free writing interface with:
  - Auto-save (2-second debounce)
  - Word count tracking
  - Text formatting (Bold, Italic, Headings, Lists)
  - Keyboard shortcuts (Cmd+S, Cmd+B, Cmd+I)
- **Profile & Settings**: User profile management, app settings
- **Mobile-Optimized UX**:
  - Bottom tab navigation
  - Swipe gestures
  - Pull-to-refresh
  - Toast notifications
  - Loading states

### 🚧 Coming Soon (Phase 2)

- **Offline Mode**: SQLite local database with background sync
- **AI Enhancement**: Text improvement suggestions
- **Push Notifications**: Writing streak reminders
- **Voice-to-Text**: Mobile dictation support
- **Biometric Auth**: Face ID / Touch ID
- **Reading Mode**: Distraction-free chapter reading
- **Export**: PDF / EPUB generation

## Tech Stack

- **Framework**: [Tauri 2.0](https://v2.tauri.app/) (Rust + WebView)
- **Frontend**: React 19 + TypeScript
- **Styling**: Tailwind CSS
- **State Management**: Zustand
- **Data Fetching**: TanStack Query (React Query)
- **Routing**: React Router v7
- **Icons**: Lucide React
- **Build Tool**: Vite

## Prerequisites

### For All Platforms

- **Node.js**: v18+ ([Download](https://nodejs.org/))
- **Rust**: 1.75+ ([Install](https://rustup.rs/))
- **Tauri CLI**: v2+ (installed via npm)

### For iOS Development (macOS only)

- **macOS**: Ventura (13.0)+
- **Xcode**: 14+ ([Download from App Store](https://apps.apple.com/us/app/xcode/id497799835))
- **Xcode Command Line Tools**:
  ```bash
  xcode-select --install
  ```

### For Android Development

- **Android Studio**: Latest version ([Download](https://developer.android.com/studio))
- **Android SDK**: API Level 26+ (Android 8.0+)
- **Android NDK**: Latest version
- **Java JDK**: 17+

## Getting Started

### 1. Clone and Install

```bash
cd /path/to/authorworks/mobile
npm install
```

### 2. Environment Configuration

Create `.env` file:

```bash
cp .env.example .env
```

Edit `.env` and configure API endpoint:

```env
VITE_API_URL=https://api.authorworks.io/v1
# or for local development:
# VITE_API_URL=http://localhost:3100/v1
```

### 3. Development

#### Desktop (for testing)

```bash
npm run tauri:dev
```

This opens the app in a desktop window for rapid development.

#### iOS (macOS only)

**First time setup:**

```bash
npm run tauri:ios
```

This initializes the iOS project in `src-tauri/gen/apple/`.

**Run on iOS Simulator:**

```bash
npm run tauri:ios:dev
```

**Run on physical iPhone:**

1. Connect iPhone via USB
2. Trust computer on iPhone
3. Select device in Xcode
4. Run: `npm run tauri:ios:dev`

#### Android

**First time setup:**

```bash
npm run tauri:android
```

This initializes the Android project in `src-tauri/gen/android/`.

**Run on Android Emulator:**

1. Start Android Emulator from Android Studio
2. Run:
   ```bash
   npm run tauri:android:dev
   ```

**Run on physical Android device:**

1. Enable Developer Options on device
2. Enable USB Debugging
3. Connect via USB
4. Run: `npm run tauri:android:dev`

## Project Structure

```
mobile/
├── src/
│   ├── components/       # Reusable UI components
│   │   ├── Button.tsx
│   │   ├── Card.tsx
│   │   ├── Input.tsx
│   │   ├── Toast.tsx
│   │   └── BottomNav.tsx
│   ├── screens/          # Main application screens
│   │   ├── Login.tsx
│   │   ├── Dashboard.tsx
│   │   ├── Books.tsx
│   │   ├── BookDetail.tsx
│   │   ├── CreateBook.tsx
│   │   ├── CreateChapter.tsx
│   │   ├── Editor.tsx
│   │   ├── Profile.tsx
│   │   └── Settings.tsx
│   ├── stores/           # Zustand state management
│   │   ├── authStore.ts
│   │   └── uiStore.ts
│   ├── lib/              # Utilities and API client
│   │   └── api.ts
│   ├── App.tsx           # Main app component with routing
│   ├── main.tsx          # Entry point
│   └── index.css         # Global styles + Tailwind
├── src-tauri/            # Tauri Rust backend
│   ├── src/
│   │   └── main.rs       # Rust entry point
│   ├── Cargo.toml        # Rust dependencies
│   └── tauri.conf.json   # Tauri configuration
├── public/               # Static assets
├── package.json
├── tailwind.config.js
├── tsconfig.json
├── vite.config.ts
└── README.md
```

## Key Screens

### Dashboard
- Statistics cards (Books, Words, AI Usage, Streak)
- Recent books list
- Quick actions (Create Book, Continue Writing)

### Books
- Search and filter books by status
- Create new books
- Navigate to book details

### Book Detail
- View book metadata
- List all chapters
- Add, edit, delete chapters
- Navigate to editor

### Editor
- Full-screen writing interface
- Auto-save with debounce
- Word count tracking
- Text formatting toolbar
- Keyboard shortcuts

### Profile
- User information
- Settings access
- Logout

## Build for Production

### iOS (App Store)

```bash
npm run tauri:ios:build
```

This creates an IPA file in `src-tauri/gen/apple/build/`.

**Next steps:**
1. Open `src-tauri/gen/apple/[app-name].xcodeproj` in Xcode
2. Configure signing & capabilities
3. Archive and upload to App Store Connect

### Android (Google Play)

```bash
npm run tauri:android:build
```

This creates an APK/AAB in `src-tauri/gen/android/app/build/outputs/`.

**Next steps:**
1. Sign the APK/AAB with your keystore
2. Upload to Google Play Console

## Development Tips

### Hot Reload

Frontend changes (React, CSS) hot-reload automatically. Rust changes require app restart.

### Debugging

**Frontend (React):**
- Open DevTools in the Tauri window (Right-click → Inspect)
- Use React DevTools browser extension

**Rust:**
- Add `println!()` or `dbg!()` statements
- View output in terminal running `tauri:dev`

### Styling

This app uses **Tailwind CSS**. Key custom classes:

- `.btn-primary` - Primary button (indigo/purple gradient)
- `.btn-secondary` - Secondary button (slate)
- `.card` - Card component (slate background, border)
- `.input` - Text input field
- `.textarea` - Textarea field

Brand colors:
- Primary: `indigo-500` (#6366F1)
- Secondary: `purple-500` (#A855F7)
- Background: `slate-950` (#0F172A)

## Troubleshooting

### iOS Build Fails

**Error: "No signing certificate found"**

Solution:
1. Open Xcode
2. Preferences → Accounts → Add Apple ID
3. Select team in project settings
4. Let Xcode manage signing automatically

### Android Build Fails

**Error: "SDK location not found"**

Solution:
1. Open Android Studio
2. Tools → SDK Manager
3. Note SDK location path
4. Set `ANDROID_HOME` environment variable:
   ```bash
   export ANDROID_HOME=/path/to/Android/sdk
   ```

### App Crashes on Launch

1. Check API endpoint in `.env` is accessible
2. Check Rust console for errors
3. Clear app data and reinstall

## Contributing

### Git Workflow

1. Create feature branch: `git checkout -b feature/my-feature`
2. Make changes and test thoroughly
3. Commit: `git commit -m "feat: add my feature"`
4. Push: `git push origin feature/my-feature`
5. Create Pull Request

## License

Proprietary - AuthorWorks © 2025

## Support

- **Documentation**: [Full PRD](../docs/mobile-prd.md)
- **Issues**: GitHub Issues
- **Email**: support@authorworks.io

---

Built with ❤️ by the AuthorWorks team
