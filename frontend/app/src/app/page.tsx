'use client'

import Link from 'next/link'
import { BookOpen, Sparkles, PenTool, Zap, ArrowRight } from 'lucide-react'
import { useAuth } from './hooks/useAuth'

export default function Home() {
  const { isAuthenticated, login } = useAuth()

  return (
    <div className="min-h-screen">
      {/* Hero Section */}
      <section className="relative overflow-hidden">
        {/* Background gradient */}
        <div className="absolute inset-0 bg-gradient-to-br from-indigo-900/20 via-purple-900/20 to-slate-950" />
        <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_top,_var(--tw-gradient-stops))] from-indigo-600/10 via-transparent to-transparent" />
        
        {/* Grid pattern */}
        <div className="absolute inset-0 bg-[url('/grid.svg')] bg-center opacity-20" />

        <div className="relative max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 pt-32 pb-20">
          <div className="text-center space-y-8">
            <div className="inline-flex items-center gap-2 px-4 py-2 rounded-full bg-indigo-500/10 border border-indigo-500/20 text-indigo-400 text-sm">
              <Sparkles className="h-4 w-4" />
              <span>AI-Powered Book Creation Platform</span>
            </div>

            <h1 className="text-5xl sm:text-6xl lg:text-7xl font-playfair font-bold leading-tight">
              Write Your <span className="gradient-text">Masterpiece</span>
              <br />with AI Assistance
            </h1>

            <p className="max-w-2xl mx-auto text-xl text-slate-400">
              AuthorWorks combines powerful AI with intuitive writing tools to help you 
              create, edit, and publish your books faster than ever before.
            </p>

            <div className="flex flex-col sm:flex-row items-center justify-center gap-4">
              {isAuthenticated ? (
                <Link href="/dashboard" className="btn-primary text-lg px-8 py-4 group">
                  Go to Dashboard
                  <ArrowRight className="ml-2 h-5 w-5 group-hover:translate-x-1 transition-transform" />
                </Link>
              ) : (
                <button onClick={login} className="btn-primary text-lg px-8 py-4 group">
                  Start Writing Free
                  <ArrowRight className="ml-2 h-5 w-5 group-hover:translate-x-1 transition-transform" />
                </button>
              )}
              <Link href="#features" className="btn-secondary text-lg px-8 py-4">
                Learn More
              </Link>
            </div>
          </div>

          {/* Hero image/illustration */}
          <div className="mt-20 relative">
            <div className="absolute inset-0 bg-gradient-to-t from-slate-950 via-transparent to-transparent z-10" />
            <div className="relative rounded-2xl overflow-hidden border border-slate-800 shadow-2xl glow-indigo">
              <div className="bg-slate-900 p-4 flex items-center gap-2 border-b border-slate-800">
                <div className="flex gap-2">
                  <div className="w-3 h-3 rounded-full bg-red-500" />
                  <div className="w-3 h-3 rounded-full bg-yellow-500" />
                  <div className="w-3 h-3 rounded-full bg-green-500" />
                </div>
                <span className="text-slate-400 text-sm ml-4">AuthorWorks Editor</span>
              </div>
              <div className="bg-slate-950/80 p-8 min-h-[400px]">
                <div className="max-w-3xl mx-auto space-y-4">
                  <h2 className="font-playfair text-2xl text-white">Chapter 1: The Beginning</h2>
                  <div className="space-y-4 text-slate-400">
                    <p className="animate-fade-in" style={{ animationDelay: '0.2s' }}>
                      The morning sun cast long shadows across the empty street, its golden rays 
                      illuminating particles of dust suspended in the crisp autumn air.
                    </p>
                    <p className="animate-fade-in" style={{ animationDelay: '0.4s' }}>
                      Sarah stood at the window, coffee in hand, watching the world slowly wake up 
                      around her. Today was the day everything would change.
                    </p>
                    <div className="flex items-center gap-2 pt-4 animate-fade-in" style={{ animationDelay: '0.6s' }}>
                      <div className="h-6 w-6 rounded bg-indigo-500/20 flex items-center justify-center">
                        <Sparkles className="h-4 w-4 text-indigo-400" />
                      </div>
                      <span className="text-indigo-400 text-sm">AI suggesting: Continue with character introduction...</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Features Section */}
      <section id="features" className="py-24 bg-slate-950">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="text-center mb-16">
            <h2 className="text-4xl font-playfair font-bold mb-4">
              Everything You Need to Write
            </h2>
            <p className="text-xl text-slate-400">
              Powerful tools designed specifically for authors
            </p>
          </div>

          <div className="grid md:grid-cols-3 gap-8">
            <div className="card group hover:border-indigo-500/50 transition-all duration-300">
              <div className="h-12 w-12 rounded-xl bg-indigo-500/20 flex items-center justify-center mb-4 group-hover:scale-110 transition-transform">
                <Sparkles className="h-6 w-6 text-indigo-400" />
              </div>
              <h3 className="text-xl font-semibold mb-2">AI Writing Assistant</h3>
              <p className="text-slate-400">
                Get intelligent suggestions for plot development, dialogue, and descriptions. 
                Break through writer's block with AI-powered inspiration.
              </p>
            </div>

            <div className="card group hover:border-purple-500/50 transition-all duration-300">
              <div className="h-12 w-12 rounded-xl bg-purple-500/20 flex items-center justify-center mb-4 group-hover:scale-110 transition-transform">
                <PenTool className="h-6 w-6 text-purple-400" />
              </div>
              <h3 className="text-xl font-semibold mb-2">Professional Editor</h3>
              <p className="text-slate-400">
                A distraction-free writing environment with real-time collaboration, 
                version history, and powerful formatting tools.
              </p>
            </div>

            <div className="card group hover:border-pink-500/50 transition-all duration-300">
              <div className="h-12 w-12 rounded-xl bg-pink-500/20 flex items-center justify-center mb-4 group-hover:scale-110 transition-transform">
                <Zap className="h-6 w-6 text-pink-400" />
              </div>
              <h3 className="text-xl font-semibold mb-2">One-Click Publishing</h3>
              <p className="text-slate-400">
                Export your work to multiple formats or publish directly to popular 
                platforms with our integrated publishing tools.
              </p>
            </div>
          </div>
        </div>
      </section>

      {/* CTA Section */}
      <section className="py-24 bg-gradient-to-b from-slate-950 to-slate-900">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <h2 className="text-4xl font-playfair font-bold mb-4">
            Ready to Write Your Story?
          </h2>
          <p className="text-xl text-slate-400 mb-8">
            Join thousands of authors who are already creating with AuthorWorks.
          </p>
          {isAuthenticated ? (
            <Link href="/dashboard" className="btn-primary text-lg px-8 py-4">
              Go to Dashboard
            </Link>
          ) : (
            <button onClick={login} className="btn-primary text-lg px-8 py-4">
              Get Started for Free
            </button>
          )}
        </div>
      </section>

      {/* Footer */}
      <footer className="bg-slate-900 border-t border-slate-800 py-12">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex flex-col md:flex-row items-center justify-between gap-4">
            <div className="flex items-center gap-2">
              <BookOpen className="h-6 w-6 text-indigo-500" />
              <span className="font-playfair text-lg font-bold">AuthorWorks</span>
            </div>
            <p className="text-slate-400 text-sm">
              © {new Date().getFullYear()} AuthorWorks. All rights reserved.
            </p>
          </div>
        </div>
      </footer>
    </div>
  )
}

