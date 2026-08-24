// T11 R4 B-3 Spark Connector Reader + Writer JS Harness (同构算法, round-trip)
const assert = require("assert");
const {describe, it} = require("mocha");

class GraphSparkStore {
  constructor(){ this.nodes = new Map(); this.edges = new Map(); }
  idempNodeKey(n){ return BigInt(n.id); }
  idempEdgeKey(e){ return `${e.source}:${e.target}:${e.label}`; }
  bulk(rows){
    const stats = { nodes_inserted:0, nodes_updated:0, edges_inserted:0, edges_updated:0, duplicates_skipped:0, failed_rows:0 };
    for (const r of rows) {
      if (r.kind === "node") {
        const v = r.row;
        if (!v.id || !v.label || !v.type_) { stats.failed_rows++; continue; }
        const k = this.idempNodeKey(v);
        if (this.nodes.has(k)) stats.nodes_updated++; else stats.nodes_inserted++;
        this.nodes.set(k, v);
      } else {
        const e = r.row;
        if (!e.source || !e.target || !e.label) { stats.failed_rows++; continue; }
        const k = this.idempEdgeKey(e);
        if (this.edges.has(k)) stats.edges_updated++; else stats.edges_inserted++;
        this.edges.set(k, {...e});
      }
    }
    return stats;
  }
  pagedNodes(page, size){
    const arr = [...this.nodes.values()].sort((a,b)=>Number(a.id-b.id));
    const start = (page-1)*size;
    return { page, size, total: arr.length, schema:["id","label","type_","attr"], rows: arr.slice(start, start+size) };
  }
  countNodes(){ return this.nodes.size; }
  countEdges(){ return this.edges.size; }
  nodeSet(){ return new Set([...this.nodes.values()].map(n=>JSON.stringify(n))); }
  edgeSet(){ return new Set([...this.edges.values()].map(e=>JSON.stringify({s:e.source,t:e.target,l:e.label}))); }
}

describe("T11-R4 / B-3 Spark Connector (8 it)", function(){
  it("schema: pagedNodes includes required 4 fields", () => {
    const s = new GraphSparkStore();
    s.bulk([{kind:"node", row:{id:1,label:"L1",type_:"Person",attr:{age:"1"}}}]);
    const f = s.pagedNodes(1, 10);
    assert.deepStrictEqual(f.schema, ["id","label","type_","attr"]);
  });

  it("bulk write nodes then pagedNodes page 1 size 10 returns subset", () => {
    const s = new GraphSparkStore();
    const rows = [];
    for (let i=1;i<=2000;i++) rows.push({kind:"node",row:{id:i,label:"u"+i,type_:"Person",attr:{}}});
    for (let i=1;i<=3000;i++) rows.push({kind:"edge",row:{source: (i*13)%2000+1, target:(i*17)%2000+1, label:"e"+i, props:{}}});
    const st = s.bulk(rows);
    assert.strictEqual(st.nodes_inserted, 2000);
    assert.strictEqual(st.edges_inserted, 3000);
    const p1 = s.pagedNodes(1, 100);
    assert.strictEqual(p1.total, 2000);
    assert.strictEqual(p1.rows.length, 100);
    assert.strictEqual(p1.rows[0].id, 1);
  });

  it("round-trip set symmetric diff empty (nodes+edges)", () => {
    const s = new GraphSparkStore();
    const expNodes = new Set(), expEdges = new Set();
    const rows = [];
    for (let i=1;i<=100;i++) {
      const n = {id:i,label:"u"+i,type_:"Person",attr:{}};
      expNodes.add(JSON.stringify(n));
      rows.push({kind:"node",row:n});
    }
    for (let i=1;i<=150;i++) {
      const src=(i*13)%100+1, tgt=(i*7)%100+1, lab="e"+i;
      expEdges.add(JSON.stringify({s:src,t:tgt,l:lab}));
      rows.push({kind:"edge",row:{source:src,target:tgt,label:lab,props:{}}});
    }
    s.bulk(rows);
    assert.deepStrictEqual(s.nodeSet(), expNodes);
    assert.deepStrictEqual(s.edgeSet(), expEdges);
  });

  it("idempotency key: repeated same edge gives edges_updated > 0", () => {
    const s = new GraphSparkStore();
    const r1 = s.bulk([{kind:"edge",row:{source:1,target:2,label:"knows",props:{}}}]);
    const r2 = s.bulk([{kind:"edge",row:{source:1,target:2,label:"knows",props:{}}}]);
    assert.strictEqual(r1.edges_inserted, 1);
    assert.strictEqual(r2.edges_updated, 1);
    assert.strictEqual(s.countEdges(), 1);
  });

  it("empty write → 0 inserted, 0 failed", () => {
    const s = new GraphSparkStore();
    const st = s.bulk([]);
    assert.deepStrictEqual(st, {nodes_inserted:0,nodes_updated:0,edges_inserted:0,edges_updated:0,duplicates_skipped:0,failed_rows:0});
  });

  it("large page size: page=1 size=9999 includes all <=2000 rows", () => {
    const s = new GraphSparkStore();
    const rows = [];
    for (let i=1;i<=137;i++) rows.push({kind:"node",row:{id:i,label:"L"+i,type_:"T",attr:{}}});
    s.bulk(rows);
    const p1 = s.pagedNodes(1, 9999);
    assert.strictEqual(p1.rows.length, 137);
    assert.strictEqual(p1.total, 137);
  });

  it("invalid id/label rows counted in failed_rows", () => {
    const s = new GraphSparkStore();
    const st = s.bulk([{kind:"node",row:{id:0,label:"",type_:"",attr:{}}}, {kind:"edge",row:{source:0,target:0,label:"",props:{}}}]);
    assert.strictEqual(st.failed_rows, 2);
    assert.strictEqual(s.countNodes(), 0);
    assert.strictEqual(s.countEdges(), 0);
  });

  it("中文type_（type_汉字） preserved in roundtrip", () => {
    const s = new GraphSparkStore();
    s.bulk([{kind:"node",row:{id:1,label:"员工",type_:"人员",attr:{部门:"研发"}}}]);
    const p = s.pagedNodes(1, 10);
    assert.strictEqual(p.rows[0].type_, "人员");
    assert.strictEqual(p.rows[0].attr.部门, "研发");
  });
});
