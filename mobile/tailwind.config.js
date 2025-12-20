/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        slate: {
          950: '#0F172A',
          900: '#1E293B',
          800: '#334155',
          700: '#475569',
          600: '#64748B',
          500: '#94A3B8',
          400: '#CBD5E1',
          300: '#E2E8F0',
          200: '#F1F5F9',
          100: '#F8FAFC',
        },
        indigo: {
          500: '#6366F1',
          400: '#818CF8',
          600: '#4F46E5',
        },
        purple: {
          500: '#A855F7',
          400: '#C084FC',
          600: '#9333EA',
        },
        pink: {
          500: '#EC4899',
          400: '#F472B6',
        },
        green: {
          500: '#10B981',
          400: '#34D399',
        },
      },
      fontFamily: {
        playfair: ['"Playfair Display"', 'serif'],
        serif: ['Georgia', 'serif'],
      },
    },
  },
  plugins: [],
}
