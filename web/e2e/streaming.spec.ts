import { test, expect } from '@playwright/test';

test.describe('Streaming Page', () => {
  test.beforeEach(async ({ page }) => {
    // Go to streaming page
    await page.goto('/streaming');
  });

  test('should load streaming page without JavaScript errors', async ({ page }) => {
    // Listen for console errors
    const errors: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'error') {
        errors.push(msg.text());
      }
    });

    // Wait for page to load
    await page.waitForLoadState('networkidle');

    // Check that page loaded
    await expect(page.locator('h1, h2')).toBeVisible();

    // Check for the specific error we're debugging
    const hasIterableError = errors.some(error => 
      error.includes('d is not iterable') || error.includes('TypeError')
    );
    
    if (hasIterableError) {
      console.error('Found iterable error in console:', errors);
    }

    // This test will fail if the "d is not iterable" error occurs
    expect(hasIterableError).toBe(false);
  });

  test('should display trending content or loading state', async ({ page }) => {
    // Wait for either trending content or loading indicator
    await expect(
      page.locator('[data-testid="trending-content"], [data-testid="loading-indicator"], .animate-spin')
    ).toBeVisible({ timeout: 10000 });

    // If content loaded, verify structure
    const trendingContent = page.locator('[data-testid="trending-content"]');
    if (await trendingContent.isVisible()) {
      await expect(trendingContent).toBeVisible();
      
      // Check for movie cards or content
      const movieCards = page.locator('[data-testid="movie-card"], .movie-card, .trending-item');
      if (await movieCards.count() > 0) {
        await expect(movieCards.first()).toBeVisible();
      }
    }
  });

  test('should handle WebSocket connection gracefully', async ({ page }) => {
    // Monitor WebSocket errors
    const wsErrors: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'error' && msg.text().includes('WebSocket')) {
        wsErrors.push(msg.text());
      }
    });

    // Wait for page to load
    await page.waitForLoadState('networkidle');

    // WebSocket errors shouldn't crash the page
    await expect(page.locator('body')).toBeVisible();
    
    // If WebSocket errors occur, they should be handled gracefully
    if (wsErrors.length > 0) {
      console.warn('WebSocket errors detected:', wsErrors);
      
      // Page should still be functional despite WebSocket issues
      await expect(page.locator('nav, header, main')).toBeVisible();
    }
  });

  test('should allow navigation without breaking', async ({ page }) => {
    // Navigate to different sections
    const sections = ['Movies', 'Queue', 'Dashboard', 'Settings'];
    
    for (const section of sections) {
      const link = page.locator(`nav a:has-text("${section}"), a[href*="${section.toLowerCase()}"]`);
      if (await link.isVisible()) {
        await link.click();
        await page.waitForLoadState('networkidle');
        
        // Check that navigation worked and no errors occurred
        await expect(page.locator('body')).toBeVisible();
      }
    }

    // Return to streaming page
    await page.goto('/streaming');
    await expect(page.locator('body')).toBeVisible();
  });
});