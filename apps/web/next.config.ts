import type {NextConfig} from 'next';

const nextConfig: NextConfig = {
  transpilePackages: ['@stash/shared'],
  serverExternalPackages: ['pg'],
  images: {unoptimized: true},
};

export default nextConfig;
