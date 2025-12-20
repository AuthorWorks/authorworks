import React from 'react';
import { Link, useLocation } from 'react-router-dom';
import { Home, BookOpen, Edit, User } from 'lucide-react';

export const BottomNav: React.FC = () => {
  const location = useLocation();

  const navItems = [
    { path: '/dashboard', icon: Home, label: 'Home' },
    { path: '/books', icon: BookOpen, label: 'Books' },
    { path: '/write', icon: Edit, label: 'Write' },
    { path: '/profile', icon: User, label: 'Profile' },
  ];

  // Don't show bottom nav on certain routes
  const hideOnRoutes = ['/login', '/editor'];
  const shouldHide = hideOnRoutes.some(route => location.pathname.startsWith(route));

  if (shouldHide) return null;

  return (
    <nav className="fixed bottom-0 left-0 right-0 bg-slate-900/95 backdrop-blur-xl border-t border-slate-800 z-50">
      <div className="flex items-center justify-around py-2 px-4 safe-area-bottom">
        {navItems.map(({ path, icon: Icon, label }) => {
          const isActive = location.pathname === path ||
                          (path === '/books' && location.pathname.startsWith('/books'));

          return (
            <Link
              key={path}
              to={path}
              className={`flex flex-col items-center gap-1 px-4 py-2 rounded-lg transition-colors ${
                isActive
                  ? 'text-indigo-400'
                  : 'text-slate-500 hover:text-slate-300'
              }`}
            >
              <Icon className="h-6 w-6" />
              <span className="text-xs font-medium">{label}</span>
            </Link>
          );
        })}
      </div>
    </nav>
  );
};
