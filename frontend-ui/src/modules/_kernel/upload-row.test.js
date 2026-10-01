// C 族私有词汇副本收口的钉子：views/workspace/panels/{CollaborationPanel,FilePanel}.vue 里
// getFileType/formatFileSize/占位行三处函数体原本逐字符各两份（574/210/197 B），现并入 _kernel/upload-row.js。
import { describe, it, expect, vi, afterEach } from 'vitest'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { getFileType, formatFileSize, makeUploadRow } from '@/modules/_kernel/upload-row'

const SRC = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..')

describe('upload-row 单源', () => {
  afterEach(() => vi.useRealTimers())

  it('getFileType 八个分支与合并前的副本逐字一致', () => {
    expect(getFileType('a.pdf')).toBe('pdf')
    expect(getFileType('a.DOCX')).toBe('doc')
    expect(getFileType('a.SVG')).toBe('image')
    expect(getFileType('a.csv')).toBe('excel')
    expect(getFileType('a.ppt')).toBe('ppt')
    expect(getFileType('a.tar')).toBe('zip')
    expect(getFileType('a.vue')).toBe('code')
    expect(getFileType('a.unknownext')).toBe('other')
    expect(getFileType('no-extension')).toBe('other')
  })

  it('formatFileSize 的档位与零值空态不变（零值走文案档不是 0 B）', () => {
    expect(formatFileSize(0)).toBe('未知')
    expect(formatFileSize(null)).toBe('未知')
    expect(formatFileSize(512)).toBe('512 B')
    expect(formatFileSize(2048)).toBe('2.0 KB')
    expect(formatFileSize(3145728)).toBe('3.0 MB')
  })

  it('makeUploadRow 产出合并前那 6 个键与同一取值口径', () => {
    vi.useFakeTimers()
    vi.setSystemTime(1790510400000)
    const row = makeUploadRow({ name: 'spec.pdf', size: 2048 })
    expect(row).toEqual({
      id: 'f-1790510400000',
      name: 'spec.pdf',
      type: 'pdf',
      size: '2.0 KB',
      uploader: '我',
      time: '刚刚',
    })
    expect(Object.keys(row)).toEqual(['id', 'name', 'type', 'size', 'uploader', 'time'])
  })

  it('两份面板都改走这一把出口，且各自不再留本地副本', () => {
    for (const rel of ['views/workspace/panels/CollaborationPanel.vue', 'views/workspace/panels/FilePanel.vue']) {
      const src = fs.readFileSync(path.join(SRC, rel), 'utf8')
      expect(src, rel).toContain("import { makeUploadRow } from '@/modules/_kernel/upload-row'")
      expect(src, rel).toContain('const newFile = makeUploadRow(file)')
      expect(src, rel).not.toMatch(/function (getFileType|formatFileSize)\b/)
      expect(src, rel).not.toContain("uploader: '")
    }
  })

  it('棘轮：占位行字面量全库只许活在登记口', () => {
    const needle = "'f-' + " + "Date.now()"
    const hits = []
    const walk = (dir) => {
      for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
        const p = path.join(dir, e.name)
        if (e.isDirectory()) { walk(p); continue }
        if (!/\.(vue|js|ts)$/.test(e.name)) continue
        if (fs.readFileSync(p, 'utf8').includes(needle)) hits.push(path.relative(SRC, p).split(path.sep).join('/'))
      }
    }
    walk(SRC)
    expect(hits).toEqual(['modules/_kernel/upload-row.js'])
  })
})
