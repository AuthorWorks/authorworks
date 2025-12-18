'use client'

import dynamic from 'next/dynamic'

// Dynamically import the component with SSR disabled
const NewBookContent = dynamic(() => import('./NewBookContent'), {
  ssr: false,
  loading: () => (
    <div className="max-w-3xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <div className="animate-pulse">
        <div className="h-8 bg-slate-700 rounded w-32 mb-4"></div>
        <div className="h-12 bg-slate-700 rounded w-64 mb-2"></div>
        <div className="h-4 bg-slate-700 rounded w-48"></div>
      </div>
    </div>
  ),
})

export default function NewBookPage() {
  return <NewBookContent />
}
