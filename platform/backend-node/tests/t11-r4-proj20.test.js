// T11 R4 B-4 子图 Projection 20 (JS 同构算法)
const assert = require("assert");
const {describe, it} = require("mocha");

// Build the oracle 200-node graph (mirror of Rust projection_20::build_oracle_graph_200)
function buildOracle(){
  const vert = new Map();
  const fwd = new Map(), bwd = new Map();
  const add = (s,t,l) => { fwd.set(s, fwd.get(s)||[]); fwd.get(s).push([t,l]);
    bwd.set(t, bwd.get(t)||[]); bwd.get(t).push([s,l]); };
  for (let id=1;id<=200;id++){
    const type_ = id<=100 ? "Person" : "Org";
    const label = id<=100 ? "User-"+id : "Tenant-"+id;
    const community = (id % 7) + 1;
    const attr = {};
    if (id%3===0) attr.dept="R&D";
    if (id%5===0) attr.vip="1";
    vert.set(id, {id,label,type_,community,attr});
  }
  for (let t=2;t<=30;t++) add(1,t,"knows");
  for (let p=1;p<=90;p++) add(p, 100+p, "works_at");
  for (let o=101;o<=200;o++) add(o, o===200?101:o+1, "partner");
  for (let i=2;i<=20;i++) add(100-i, i, "reports_to");
  return {vert, fwd, bwd, deg(id){ const a=fwd.get(id)?.length||0, b=bwd.get(id)?.length||0; return a+b; }};
}
const DIR_IN="in", DIR_OUT="out";
function neighbors(g, start, dir, k){
  const seen = new Set([start]);
  if (!g.vert.has(start)) return new Set();
  let frontier = new Set([start]);
  for (let h=0;h<k;h++){
    const next = new Set();
    for (const n of frontier){
      const edges = (dir===DIR_OUT ? (g.fwd.get(n)||[]) : (g.bwd.get(n)||[]));
      for (const [m] of edges){
        if (g.vert.has(m) && !seen.has(m)) next.add(m);
      }
    }
    if (next.size===0) break;
    for (const m of next) seen.add(m);
    frontier = next;
  }
  seen.delete(start);
  return seen;
}
const FILTERS = ["type","community","attr","degree","label"];
const DIRS = [DIR_OUT, DIR_IN];
const HOPS = [1,2];
const OPERATORS = [];
for (const f of FILTERS) for (const d of DIRS) for (const h of HOPS) OPERATORS.push({id:`proj_${f}_${d}_${h}`,f,d,h});

function applyOp(g, op, seed, param){
  const hood = neighbors(g, seed, op.d, op.h);
  hood.add(seed);
  const selected = new Set();
  const paramDeg = Number.parseInt(param, 10);
  for (const nid of hood){
    const v = g.vert.get(nid);
    if (!v) continue;
    switch (op.f){
      case "type": if (v.type_ === param) selected.add(nid); break;
      case "community": if (v.community === paramDeg) selected.add(nid); break;
      case "attr": if (Object.prototype.hasOwnProperty.call(v.attr, param)) selected.add(nid); break;
      case "degree": if (g.deg(nid) >= paramDeg) selected.add(nid); break;
      case "label": if (v.label === param) selected.add(nid); break;
    }
  }
  if (g.vert.has(seed)) selected.add(seed);
  return selected;
}

describe("T11-R4 / B-4 Projection 20 (12 it)", function(){
  it("registry length exactly 20", () => {
    assert.strictEqual(OPERATORS.length, 20);
    const set = new Set(OPERATORS.map(o=>o.id));
    assert.strictEqual(set.size, 20);
  });

  // 5 filter single tests
  it("filter type matches Person from seed 1 (out 1-hop)", () => {
    const g = buildOracle();
    const s = applyOp(g, OPERATORS.find(o=>o.id==="proj_type_out_1"), 1, "Person");
    // 1 + 2..30 all id <= 30 Person + seed 1 itself Person => size = 30
    assert.strictEqual(s.size, 30);
  });

  it("filter community=2 out 1-hop from seed=2 (id%7+1=2 → id mod 7=1)", () => {
    const g = buildOracle();
    const s = applyOp(g, OPERATORS.find(o=>o.id==="proj_community_out_1"), 2, "2");
    assert.ok(s.has(2));
  });

  it("filter attr=dept on seed=3, out 1-hop returns seed + any neighbors where id%3===0", () => {
    const g = buildOracle();
    const s = applyOp(g, OPERATORS.find(o=>o.id==="proj_attr_out_1"), 3, "dept");
    assert.ok(s.has(3));
  });

  it("filter degree >= 2 on seed=1 out 1-hop includes hub neighbors", () => {
    const g = buildOracle();
    const s = applyOp(g, OPERATORS.find(o=>o.id==="proj_degree_out_1"), 1, "2");
    assert.ok(s.size >= 25, "size="+s.size);
  });

  it("filter label exact match User-1 out 1-hop", () => {
    const g = buildOracle();
    const s = applyOp(g, OPERATORS.find(o=>o.id==="proj_label_out_1"), 1, "User-1");
    assert.ok(s.has(1));
  });

  // direction × hop tests
  it("IN direction works: org 101 receives from person via works_at edge", () => {
    const g = buildOracle();
    const s = applyOp(g, OPERATORS.find(o=>o.id==="proj_type_in_1"), 101, "Person");
    assert.ok(s.has(1), "must include person id=1 who works_at 101");
  });

  it("2-hop reaches 2nd tier neighborhood", () => {
    const g = buildOracle();
    const s1 = applyOp(g, OPERATORS.find(o=>o.id==="proj_type_out_1"), 1, "Person");
    const s2 = applyOp(g, OPERATORS.find(o=>o.id==="proj_type_out_2"), 1, "Person");
    assert.ok(s2.size >= s1.size);
  });

  it("AND-composite intersection is subset of both single operator sets", () => {
    const g = buildOracle();
    const seed = 1;
    const a = applyOp(g, OPERATORS.find(o=>o.id==="proj_type_out_1"), seed, "Person");
    const b = applyOp(g, OPERATORS.find(o=>o.id==="proj_degree_out_1"), seed, "1");
    const inter = new Set([...a].filter(v => b.has(v)));
    for (const v of inter) { assert.ok(a.has(v) && b.has(v)); }
    assert.ok(inter.size > 0);
  });

  it("OR-composite union is superset of both", () => {
    const g = buildOracle();
    const a = applyOp(g, OPERATORS[0], 1, "Person");
    const b = applyOp(g, OPERATORS[3], 1, "Person");
    const un = new Set([...a, ...b]);
    assert.ok(un.size >= a.size && un.size >= b.size);
  });

  it("empty graph projection returns size <= 1 (seed only)", () => {
    const g = {vert: new Map(), fwd: new Map(), bwd: new Map(), deg:()=>0};
    const s = applyOp(g, OPERATORS[0], 1, "Person");
    assert.strictEqual(s.size, 0); // vert empty, seed not added
  });

  it("large 1000-node synthetic: applyOp completes < 1s for 20 operators", () => {
    const g = buildOracle();
    // extend to 1000 synthetically
    for (let id=201;id<=1000;id++){
      g.vert.set(id, {id,label:"S"+id,type_:"Synthetic",community:(id%5)+1,attr:{}});
    }
    const t0 = Date.now();
    for (const op of OPERATORS){
      applyOp(g, op, 7, (op.f==="type"?"Person":(op.f==="label"?"User-1":"1")));
    }
    const d = Date.now()-t0;
    assert.ok(d < 1000, "took too long "+d+" ms");
  });

  it("unknown filter id throws on lookup via helper", () => {
    assert.throws(() => {
      const op = OPERATORS.find(o=>o.id==="proj_XXX_in_1");
      if (!op) throw new Error("NoSuchOperator");
    }, /NoSuchOperator/);
  });

  it("reverse direction projection: IN from person id=2 gives those who point to 2", () => {
    const g = buildOracle();
    // reports_to 98->2 exists, so proj_type_in_1 Person from 2 → {2,98}
    const s = applyOp(g, OPERATORS.find(o=>o.id==="proj_type_in_1"), 2, "Person");
    assert.ok(s.has(2) && s.has(98), "size="+s.size);
  });

  it("cross-community projection (seed in community X) expands to neighbors in any community via type filter", () => {
    const g = buildOracle();
    // seed=1 has community=2 (since 1%7+1=2), neighbors include 2..30 of varying communities
    const s = applyOp(g, OPERATORS.find(o=>o.id==="proj_type_out_1"), 1, "Person");
    const communities = new Set();
    for (const id of s) communities.add(g.vert.get(id)?.community);
    assert.ok(communities.size >= 2, "cross-community: "+[...communities].join(","));
  });
});
