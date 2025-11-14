import './globals.css'
import type { Metadata } from "next";
import Link from 'next/link';

export const metadata: Metadata = {
  title: "author.works",
  description: "Exploring the future of human-AI creative collaboration.",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className="flex flex-col min-h-screen font-mono">
        <header className="border-b border-primary">
          <div className="container mx-auto flex justify-between items-center p-4">
            <h1 className="text-2xl font-bold">
              <Link href="/" className="hover:text-accent transition-colors">
                author.works
              </Link>
            </h1>
            <nav>
              <ul className="flex space-x-8">
                <li><Link href="/" className="nav-link">Home</Link></li>
                <li><Link href="/about" className="nav-link">About</Link></li>
                <li><Link href="/roadmap" className="nav-link">Roadmap</Link></li>
                <li><Link href="/contact" className="nav-link">Contact</Link></li>
              </ul>
            </nav>
          </div>
        </header>
        <main className="flex-grow container mx-auto px-4 py-8">
          {children}
        </main>
        <footer className="border-t border-primary">
          <div className="container mx-auto text-center p-4">
            <p>&copy; {new Date().getFullYear()} author.works. Embracing tomorrow.</p>
          </div>
        </footer>
      </body>
    </html>
  );
}
