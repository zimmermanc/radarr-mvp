/// <reference types="vitest/config" />
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// https://vite.dev/config/
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { storybookTest } from '@storybook/addon-vitest/vitest-plugin';
const dirname = typeof __dirname !== 'undefined' ? __dirname : path.dirname(fileURLToPath(import.meta.url));

// More info at: https://storybook.js.org/docs/next/writing-tests/integrations/vitest-addon
export default defineConfig({
  plugins: [react()],
  build: {
    sourcemap: false, // Disable source maps for production
    minify: true // Enable minification for production
  },
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
        rewrite: path => path
      },
      // Also proxy the health endpoint
      '/health': {
        target: 'http://localhost:7878',
        changeOrigin: true,
        secure: false
      }
    }
  },
  preview: {
    host: '0.0.0.0',
    port: 5173
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    globals: true,
    projects: [
      // Unit/Component testing project
      {
        name: 'unit',
        test: {
          include: ['src/**/*.{test,spec}.{ts,tsx}'],
          environment: 'jsdom',
          setupFiles: ['./src/test/setup.ts'],
          coverage: {
            reporter: ['text', 'json', 'html'],
            thresholds: {
              lines: 85,
              branches: 75,
              functions: 80,
              statements: 85
            }
          }
        }
      },
      // Storybook testing project  
      {
        extends: true,
        plugins: [
          storybookTest({
            configDir: path.join(dirname, '.storybook')
          })
        ],
        test: {
          name: 'storybook',
          browser: {
            enabled: true,
            headless: true,
            provider: 'playwright',
            instances: [{
              browser: 'chromium'
            }]
          },
          setupFiles: ['.storybook/vitest.setup.ts']
        }
      }
    ]
  }
});