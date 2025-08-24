/// <reference types="vitest" />
import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    globals: true,
    css: true,
    coverage: {
      reporter: ['text', 'json', 'html'],
      exclude: [
        'node_modules/',
        '.storybook/',
        'e2e/',
        'src/test/',
        '**/*.stories.tsx',
        '**/*.config.ts',
        'dist/',
      ],
      thresholds: {
        lines: 85,
        branches: 75,
        functions: 80,
        statements: 85,
      },
    },
  },
});