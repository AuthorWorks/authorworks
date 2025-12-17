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
  async rewrites() {
    const apiGateway = process.env.API_GATEWAY_URL || 'http://localhost:8080'
    return [
      {
        source: '/api/books/:path*',
        destination: `${apiGateway}/api/books/:path*`,
      },
      {
        source: '/api/chapters/:path*',
        destination: `${apiGateway}/api/chapters/:path*`,
      },
      {
        source: '/api/storage/:path*',
        destination: `${apiGateway}/api/storage/:path*`,
      },
      {
        source: '/api/subscription/:path*',
        destination: `${apiGateway}/api/subscription/:path*`,
      },
      {
        source: '/api/users/:path*',
        destination: `${apiGateway}/api/users/:path*`,
      },
      {
        source: '/api/search/:path*',
        destination: `${apiGateway}/api/search/:path*`,
      },
      {
        source: '/api/media/:path*',
        destination: `${apiGateway}/api/media/:path*`,
      },
      {
        source: '/api/generate/:path*',
        destination: `${apiGateway}/api/generate/:path*`,
      },
    ]
  },
}

module.exports = nextConfig

