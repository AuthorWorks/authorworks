/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  output: 'standalone',
  images: {
    domains: ['localhost', 'minio', 'logto', 'localist-minio-local'],
    remotePatterns: [
      {
        protocol: 'https',
        hostname: '**',
      },
      {
        protocol: 'http',
        hostname: '**',
      },
    ],
  },
  // API routes are now handled directly by Next.js API routes in /app/api
  // Removed rewrites to external gateway - books, chapters, generate all handled locally
};

module.exports = nextConfig;
