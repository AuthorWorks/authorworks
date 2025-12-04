'use client'

import { useEffect, useState } from 'react'
import { useRouter } from 'next/navigation'
import { User, CreditCard, Bell, Shield, Moon, Sun, Save, Loader2 } from 'lucide-react'
import { useAuth } from '../hooks/useAuth'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'

export default function SettingsPage() {
  const router = useRouter()
  const queryClient = useQueryClient()
  const { isAuthenticated, isLoading: authLoading, accessToken, user } = useAuth()
  
  const [activeTab, setActiveTab] = useState('profile')
  const [name, setName] = useState('')
  const [bio, setBio] = useState('')
  const [website, setWebsite] = useState('')

  useEffect(() => {
    if (!authLoading && !isAuthenticated) {
      router.push('/')
    }
  }, [authLoading, isAuthenticated, router])

  useEffect(() => {
    if (user) {
      setName(user.name || '')
    }
  }, [user])

  // Fetch profile
  const { data: profile } = useQuery({
    queryKey: ['profile'],
    queryFn: async () => {
      const response = await fetch('/api/users/profile', {
        headers: { Authorization: `Bearer ${accessToken}` },
      })
      if (!response.ok) return null
      return response.json()
    },
    enabled: isAuthenticated,
  })

  useEffect(() => {
    if (profile) {
      setBio(profile.bio || '')
      setWebsite(profile.website || '')
    }
  }, [profile])

  // Fetch subscription
  const { data: subscription } = useQuery({
    queryKey: ['subscription'],
    queryFn: async () => {
      const response = await fetch('/api/subscription', {
        headers: { Authorization: `Bearer ${accessToken}` },
      })
      if (!response.ok) throw new Error('Failed to fetch subscription')
      return response.json()
    },
    enabled: isAuthenticated && activeTab === 'billing',
  })

  // Update profile
  const updateProfileMutation = useMutation({
    mutationFn: async () => {
      const response = await fetch('/api/users/profile', {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${accessToken}`,
        },
        body: JSON.stringify({ name, bio, website }),
      })
      if (!response.ok) throw new Error('Failed to update')
      return response.json()
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['profile'] })
    },
  })

  if (authLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-indigo-500"></div>
      </div>
    )
  }

  if (!isAuthenticated) return null

  const tabs = [
    { id: 'profile', name: 'Profile', icon: User },
    { id: 'billing', name: 'Billing', icon: CreditCard },
    { id: 'notifications', name: 'Notifications', icon: Bell },
    { id: 'security', name: 'Security', icon: Shield },
  ]

  return (
    <div className="max-w-5xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <h1 className="text-3xl font-playfair font-bold mb-8">Settings</h1>

      <div className="flex flex-col md:flex-row gap-8">
        {/* Sidebar */}
        <div className="w-full md:w-64 shrink-0">
          <nav className="space-y-1">
            {tabs.map((tab) => (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={`w-full flex items-center gap-3 px-4 py-3 rounded-lg transition-all ${
                  activeTab === tab.id
                    ? 'bg-indigo-600/20 text-indigo-400'
                    : 'text-slate-400 hover:text-white hover:bg-slate-800'
                }`}
              >
                <tab.icon className="h-5 w-5" />
                {tab.name}
              </button>
            ))}
          </nav>
        </div>

        {/* Content */}
        <div className="flex-1">
          {activeTab === 'profile' && (
            <div className="card space-y-6">
              <h2 className="text-xl font-semibold">Profile Settings</h2>
              
              <div>
                <label className="block text-sm font-medium text-slate-300 mb-2">
                  Display Name
                </label>
                <input
                  type="text"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  className="input"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-slate-300 mb-2">
                  Email
                </label>
                <input
                  type="email"
                  value={user?.email || ''}
                  disabled
                  className="input opacity-60 cursor-not-allowed"
                />
                <p className="text-slate-500 text-sm mt-1">
                  Email is managed by your authentication provider
                </p>
              </div>

              <div>
                <label className="block text-sm font-medium text-slate-300 mb-2">
                  Bio
                </label>
                <textarea
                  value={bio}
                  onChange={(e) => setBio(e.target.value)}
                  rows={4}
                  className="textarea"
                  placeholder="Tell readers about yourself..."
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-slate-300 mb-2">
                  Website
                </label>
                <input
                  type="url"
                  value={website}
                  onChange={(e) => setWebsite(e.target.value)}
                  className="input"
                  placeholder="https://your-website.com"
                />
              </div>

              <button
                onClick={() => updateProfileMutation.mutate()}
                disabled={updateProfileMutation.isPending}
                className="btn-primary"
              >
                {updateProfileMutation.isPending ? (
                  <>
                    <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                    Saving...
                  </>
                ) : (
                  <>
                    <Save className="h-4 w-4 mr-2" />
                    Save Changes
                  </>
                )}
              </button>
            </div>
          )}

          {activeTab === 'billing' && (
            <div className="space-y-6">
              <div className="card">
                <h2 className="text-xl font-semibold mb-4">Current Plan</h2>
                <div className="flex items-center justify-between p-4 bg-slate-800/50 rounded-lg">
                  <div>
                    <h3 className="text-lg font-semibold capitalize">
                      {subscription?.plan_id || 'Free'} Plan
                    </h3>
                    <p className="text-slate-400 text-sm">
                      {subscription?.plan_id === 'free' 
                        ? 'Basic features for getting started'
                        : subscription?.plan_id === 'pro'
                        ? 'Full access to all features'
                        : 'Enterprise features for teams'}
                    </p>
                  </div>
                  <a
                    href="/api/subscription/portal"
                    className="btn-secondary"
                  >
                    Manage Subscription
                  </a>
                </div>
              </div>

              <div className="card">
                <h2 className="text-xl font-semibold mb-4">Usage This Month</h2>
                <div className="space-y-4">
                  <div>
                    <div className="flex justify-between text-sm mb-1">
                      <span className="text-slate-400">AI Words</span>
                      <span className="text-slate-300">
                        {subscription?.usage?.ai_words?.toLocaleString() || 0} / {subscription?.limits?.ai_words_per_month?.toLocaleString() || '5,000'}
                      </span>
                    </div>
                    <div className="h-2 bg-slate-800 rounded-full overflow-hidden">
                      <div
                        className="h-full bg-gradient-to-r from-indigo-500 to-purple-500 rounded-full"
                        style={{
                          width: `${Math.min(((subscription?.usage?.ai_words || 0) / (subscription?.limits?.ai_words_per_month || 5000)) * 100, 100)}%`,
                        }}
                      />
                    </div>
                  </div>

                  <div>
                    <div className="flex justify-between text-sm mb-1">
                      <span className="text-slate-400">Storage</span>
                      <span className="text-slate-300">
                        {(subscription?.usage?.storage_gb || 0).toFixed(2)} GB / {subscription?.limits?.storage_gb || 1} GB
                      </span>
                    </div>
                    <div className="h-2 bg-slate-800 rounded-full overflow-hidden">
                      <div
                        className="h-full bg-gradient-to-r from-green-500 to-emerald-500 rounded-full"
                        style={{
                          width: `${Math.min(((subscription?.usage?.storage_gb || 0) / (subscription?.limits?.storage_gb || 1)) * 100, 100)}%`,
                        }}
                      />
                    </div>
                  </div>
                </div>
              </div>
            </div>
          )}

          {activeTab === 'notifications' && (
            <div className="card space-y-6">
              <h2 className="text-xl font-semibold">Notification Preferences</h2>
              
              {[
                { id: 'email_updates', label: 'Email updates', desc: 'Receive updates about your books and account' },
                { id: 'ai_complete', label: 'AI generation complete', desc: 'Notify when AI content generation finishes' },
                { id: 'comments', label: 'Comments and mentions', desc: 'Notify when someone comments on your work' },
                { id: 'newsletter', label: 'Newsletter', desc: 'Tips and news about AuthorWorks' },
              ].map((item) => (
                <div key={item.id} className="flex items-center justify-between py-3 border-b border-slate-800 last:border-0">
                  <div>
                    <h3 className="font-medium text-white">{item.label}</h3>
                    <p className="text-slate-500 text-sm">{item.desc}</p>
                  </div>
                  <label className="relative inline-flex items-center cursor-pointer">
                    <input type="checkbox" defaultChecked className="sr-only peer" />
                    <div className="w-11 h-6 bg-slate-700 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-indigo-500/25 rounded-full peer peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-indigo-600"></div>
                  </label>
                </div>
              ))}
            </div>
          )}

          {activeTab === 'security' && (
            <div className="space-y-6">
              <div className="card">
                <h2 className="text-xl font-semibold mb-4">Authentication</h2>
                <p className="text-slate-400 mb-4">
                  Your account is managed through our secure authentication provider (Logto).
                </p>
                <a
                  href={`${process.env.NEXT_PUBLIC_LOGTO_ENDPOINT}/profile`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="btn-secondary"
                >
                  Manage Account Security
                </a>
              </div>

              <div className="card">
                <h2 className="text-xl font-semibold mb-4">Sessions</h2>
                <p className="text-slate-400 mb-4">
                  Manage your active sessions and sign out from other devices.
                </p>
                <button className="btn-secondary text-red-400 border-red-400/30 hover:bg-red-500/10">
                  Sign Out All Devices
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

