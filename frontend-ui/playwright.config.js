import { defineConfig, devices } from '@playwright/test'
export default defineConfig({
  testDir: './tests', fullyParallel: true, forbidOnly: !!process.env.CI,
  reporter: 'html', use: { baseURL: process.env.VITE_APP_URL || 'http://localhost:5173', trace: 'on-first-retry' },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] }, testMatch: /.*@P0\.spec\.js/ },
    { name: 'webkit', use: { ...devices['Desktop Safari'] } },
    { name: 'mobile-chrome', use: { ...devices['Pixel 7'] } },
    { name: 'safari-mobile', use: { ...devices['iPhone 15'] } }
  ],
  webServer: { command: 'npm run preview -- --port 4173', url: 'http://localhost:4173', reuseExistingServer: !process.env.CI, timeout: 120000 }
})
