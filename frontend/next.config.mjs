/** @type {import('next').NextConfig} */
const nextConfig = {
    images: {
        unoptimized: true,
    },
    eslint: {
    // ⚠️ Permet au build de passer même s'il y a des erreurs ESLint.
    ignoreDuringBuilds: true,
    },
    typescript: {
        // ⚠️ Permet au build de passer même s'il y a des erreurs de typage.
        ignoreBuildErrors: true,
    },
    output: 'export',
};

export default nextConfig;
