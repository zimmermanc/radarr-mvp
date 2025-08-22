import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  server: {
    host: '0.0.0.0',
    port: 5173,
    strictPort: true,
    // Only use proxy if no VITE_API_BASE_URL is set (local development)
    proxy: process.env.VITE_API_BASE_URL ? {} : {
      // Proxy API requests to the backend server
      '/api': {
        target: 'http://localhost:7878',
        changeOrigin: true,
        secure: false,
        rewrite: (path) => path,
      },
      // Also proxy the health endpoint
      '/health': {
        target: 'http://localhost:7878',
        changeOrigin: true,
        secure: false,
      }
    },
  },
  preview: {
    host: '0.0.0.0',
    port: 5173,
  }
})
