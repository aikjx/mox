module.exports = {
  ci: { collect: { url: ['http://localhost:4173/'], numberOfRuns: 3, settings: { preset: 'desktop' } },
    assert: {
      preset: 'lighthouse:recommended',
      assertions: {
        'categories:performance': ['error', { minScore: 0.90 }],
        'categories:accessibility': ['error', { minScore: 0.90 }],
        'categories:best-practices': ['error', { minScore: 0.90 }],
        'categories:seo': ['warn', { minScore: 0.80 }]
      }
    },
    upload: { target: 'temporary-public-storage' }
  }
}
