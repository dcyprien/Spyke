// API utilities

/**
 * Get the API base URL from environment variables
 */
export function getApiUrl(): string {
  return process.env.NEXT_PUBLIC_API_URL || "http://localhost:3000";
}

/**
 * Build a complete API URL by combining base URL with a path
 * Handles trailing slashes correctly to avoid double slashes
 */
export function buildApiUrl(path: string): string {
  const baseUrl = getApiUrl().replace(/\/$/, ''); // Remove trailing slash
  const cleanPath = path.startsWith('/') ? path : `/${path}`; // Ensure leading slash
  return `${baseUrl}${cleanPath}`;
}
