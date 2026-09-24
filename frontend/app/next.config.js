/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  output: 'standalone',
  // The app renders images with plain <img> (nothing imports next/image), so
  // the Image Optimization API is pure attack surface. The old wildcard
  // http(s) remotePatterns let /_next/image fetch any URL, including
  // in-cluster services (blind SSRF), and fed untrusted files into the
  // optimizer (Next 14.x has an unauthenticated AVIF RCE there, fixed only in
  // 15.5.24). unoptimized disables the optimizer outright.
  images: {
    unoptimized: true,
  },
  // API routes are now handled directly by Next.js API routes in /app/api
  // Removed rewrites to external gateway - books, chapters, generate all handled locally
};

module.exports = nextConfig;
