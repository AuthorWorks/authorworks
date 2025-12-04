/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  images: {
    domains: ['localhost', 'minio', 'logto'],
    remotePatterns: [
      {
        protocol: 'https',
        hostname: '**',
      },
    ],
  },
  async rewrites() {
    return [
      {
        source: '/api/books/:path*',
        destination: `${process.env.CONTENT_SERVICE_URL || 'http://localhost:8080'}/books/:path*`,
      },
      {
        source: '/api/chapters/:path*',
        destination: `${process.env.CONTENT_SERVICE_URL || 'http://localhost:8080'}/chapters/:path*`,
      },
      {
        source: '/api/storage/:path*',
        destination: `${process.env.STORAGE_SERVICE_URL || 'http://localhost:8081'}/:path*`,
      },
      {
        source: '/api/subscription/:path*',
        destination: `${process.env.SUBSCRIPTION_SERVICE_URL || 'http://localhost:8083'}/:path*`,
      },
      {
        source: '/api/users/:path*',
        destination: `${process.env.USER_SERVICE_URL || 'http://localhost:8084'}/:path*`,
      },
    ]
  },
}

module.exports = nextConfig

