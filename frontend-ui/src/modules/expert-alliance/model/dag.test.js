// DAG 分层与依赖计数的同源守卫：页脚报的数字必须就是图上画出来的那件事。
import { describe, it, expect } from 'vitest'
import { layoutDag } from './dag.js'
import { normDag, normNode } from './normalize.js'

const node = (id, deps = [], status = 'completed') => ({ id, status, dependencies: deps, label: id })

describe('layoutDag 分层', () => {
  it('链式依赖逐层递进，深度从 0 起', () => {
    const ns = [node('a'), node('b', ['a']), node('c', ['b'])]
    const r = layoutDag(ns, [{ source: 'a', target: 'b' }, { source: 'b', target: 'c' }])
    expect(r.layers.map((l) => l.nodes.map((n) => n.id))).toEqual([['a'], ['b'], ['c']])
    expect(r.layers.map((l) => l.depth)).toEqual([0, 1, 2])
    expect({ ...r, layers: undefined }).toMatchObject({ declared: 2, drawn: 2, dangling: 0, backEdges: 0, edgeDelta: 0 })
  })

  it('取最深父依赖，不用首个', () => {
    const r = layoutDag([node('a'), node('b', ['a']), node('c', ['a']), node('d', ['a', 'b', 'c'])])
    expect(r.layers.find((l) => l.nodes.some((n) => n.id === 'd')).depth).toBe(2)
    const deep = layoutDag([node('a'), node('b', ['a']), node('c', ['b']), node('d', ['a', 'c'])])
    expect(deep.layers.find((l) => l.nodes.some((n) => n.id === 'd')).depth).toBe(3)
  })

  it('同一入参两次调用结果相同（纯函数，快照可用）', () => {
    const ns = [node('a'), node('b', ['a']), node('c', ['a', 'b'])]
    expect(layoutDag(ns)).toEqual(layoutDag(ns))
  })
})

describe('layoutDag 依赖记账', () => {
  it('悬空依赖参与声明数但不参与分层，单独记账', () => {
    const ns = [node('a'), node('b', ['a', 'ghost'])]
    const r = layoutDag(ns, [{ source: 'a', target: 'b' }, { source: 'ghost', target: 'b' }])
    expect({ ...r, layers: undefined }).toEqual({
      declared: 2, drawn: 1, dangling: 1, backEdges: 0, edgeTotal: 2, edgeDelta: 0, tally: { completed: 2 }
    })
    expect(r.layers.map((l) => l.depth)).toEqual([0, 1])
  })

  it('环不抛错、不丢节点，并计入 backEdges', () => {
    let r
    expect(() => { r = layoutDag([node('a', ['b']), node('b', ['a'])]) }).not.toThrow()
    expect(r.backEdges).toBeGreaterThan(0)
    expect(r.layers.flatMap((l) => l.nodes.map((n) => n.id)).sort()).toEqual(['a', 'b'])
  })

  it('自依赖算一条回边，节点仍在层里', () => {
    const r = layoutDag([node('a', ['a'])])
    expect(r.backEdges).toBe(1)
    expect(r.dangling).toBe(0)
    expect(r.layers[0].nodes.map((n) => n.id)).toEqual(['a'])
  })

  it('后端 edges 与依赖清单不再 1:1 时露出差值（alliance.rs:1565-1569 的不变量破了就报警）', () => {
    const ns = [node('a'), node('b', ['a'])]
    expect(layoutDag(ns, [{ source: 'a', target: 'b' }]).edgeDelta).toBe(0)
    expect(layoutDag(ns, [{ source: 'a', target: 'b' }, { source: 'x', target: 'y' }]).edgeDelta).toBe(1)
  })

  it('跳过与取消分开计数，不并入一个桶', () => {
    const r = layoutDag([node('a', [], 'skipped'), node('b', [], 'cancelled'), node('c', [], 'pending')])
    expect(r.tally).toEqual({ skipped: 1, cancelled: 1, pending: 1 })
  })

  it('畸形入参不炸：无 id 节点连同其依赖一并剔除，依赖里的空值不计', () => {
    const r = layoutDag([{ status: 'completed', dependencies: ['x'] }, node('a', ['', null])])
    expect(r.layers.flatMap((l) => l.nodes.map((n) => n.id))).toEqual(['a'])
    expect({ ...r, layers: undefined, tally: undefined }).toMatchObject({ declared: 0, drawn: 0, dangling: 0 })
  })
})

describe('layoutDag 与后端出参形状', () => {
  it('吃 normDag 的产物即可分层（键名以归一化后为准）', () => {
    const d = normDag({
      nodes: [
        { id: 'n1', label: '需求', status: 'completed', dependencies: [], position: { x: 0, y: 0 } },
        { id: 'n2', label: '方案', status: 'running', dependencies: ['n1'] }
      ],
      edges: [{ source: 'n1', target: 'n2', label: '依赖' }],
      stats: { total: 2, completed: 1, running: 1 }
    })
    const r = layoutDag(d.nodes, d.edges)
    expect(r.layers.map((l) => l.nodes.map((n) => n.id))).toEqual([['n1'], ['n2']])
    expect(r.tally).toEqual({ completed: 1, running: 1 })
  })

  it('normNode 给未标状态的节点落到 pending，不落到空串', () => {
    expect(normNode({ id: 'x' }).status).toBe('pending')
    expect(layoutDag([normNode({ id: 'x' })]).tally).toEqual({ pending: 1 })
  })
})
