import React from 'react';
import { useNavigate } from 'react-router-dom';
import { Mail, LogOut, Settings } from 'lucide-react';
import { useAuthStore } from '../stores/authStore';
import { Card } from '../components/Card';

export const Profile: React.FC = () => {
  const { user, clearAuth } = useAuthStore();
  const navigate = useNavigate();

  const handleLogout = () => {
    if (window.confirm('Are you sure you want to logout?')) {
      clearAuth();
      navigate('/login');
    }
  };

  return (
    <div className="min-h-screen p-4 pb-24">
      <h1 className="text-2xl font-playfair font-bold mb-6">Profile</h1>

      {/* User Info Card */}
      <Card className="mb-4">
        <div className="flex items-center gap-4 mb-4">
          <div className="h-16 w-16 rounded-full bg-gradient-to-br from-indigo-500 to-purple-500 flex items-center justify-center text-2xl font-bold">
            {user?.name?.charAt(0).toUpperCase() || 'U'}
          </div>
          <div className="flex-1 min-w-0">
            <h2 className="text-xl font-bold truncate">{user?.name || 'User'}</h2>
            <p className="text-slate-400 text-sm truncate">@{user?.username || 'username'}</p>
          </div>
        </div>

        <div className="space-y-2">
          <div className="flex items-center gap-2 text-sm text-slate-400">
            <Mail className="h-4 w-4" />
            <span>{user?.email || 'email@example.com'}</span>
          </div>
        </div>
      </Card>

      {/* Actions */}
      <div className="space-y-2">
        <Card onClick={() => navigate('/settings')} className="cursor-pointer">
          <div className="flex items-center gap-3">
            <Settings className="h-5 w-5 text-indigo-400" />
            <span>Settings</span>
          </div>
        </Card>

        <Card onClick={handleLogout} className="cursor-pointer border-red-500/20 hover:!border-red-500/50">
          <div className="flex items-center gap-3">
            <LogOut className="h-5 w-5 text-red-400" />
            <span className="text-red-400">Logout</span>
          </div>
        </Card>
      </div>

      {/* App Info */}
      <div className="mt-8 text-center text-slate-500 text-sm">
        <p>AuthorWorks Mobile v1.0.0</p>
        <p className="mt-1">Built with Tauri 2.0</p>
      </div>
    </div>
  );
};
