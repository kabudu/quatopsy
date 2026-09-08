// DOM/canvas simulation for viewer interactions. This is not browser visual QA.
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";
const code = fs.readFileSync(
  new URL("../crates/quatopsy-cli/viewer/viewer.js", import.meta.url),
  "utf8",
);
class Element {
  constructor(tag = "div") {
    this.tagName = tag.toUpperCase();
    this.children = [];
    this.dataset = {};
    this.attributes = {};
    this.events = {};
    this.value = "";
    this.textContent = "";
    this.disabled = false;
    this.width = 720;
    this.height = 440;
    this.options = [{}, {}];
  }
  append(...children) {
    children.forEach((c) => {
      c.parent = this;
      this.children.push(c);
    });
  }
  appendChild(c) {
    this.append(c);
    return c;
  }
  replaceChildren(...children) {
    this.children = [];
    this.textContent = "";
    this.append(...children);
  }
  get lastChild() {
    return this.children.at(-1);
  }
  remove() {
    this.parent.children = this.parent.children.filter((c) => c !== this);
  }
  setAttribute(k, v) {
    this.attributes[k] = v;
  }
  addEventListener(k, fn) {
    this.events[k] = fn;
  }
  fire(k, event = {}) {
    this.events[k]?.(event);
  }
  getBoundingClientRect() {
    return { left: 0, width: this.width };
  }
  getContext() {
    return (this.ctx ||= new Context());
  }
}
class Context {
  constructor() {
    this.operations = [];
    this.paths = [];
    this.start = 0;
  }
  beginPath() {
    this.start = this.operations.length;
    this.operations.push(["begin"]);
  }
  moveTo(...v) {
    this.operations.push(["move", ...v]);
  }
  lineTo(...v) {
    this.operations.push(["line", ...v]);
  }
  fillText(...v) {
    this.operations.push(["text", ...v]);
  }
  drawImage(...v) {
    this.operations.push(["image", ...v]);
  }
  clearRect() {}
  fillRect() {}
  stroke() {
    this.paths.push({
      colour: this.strokeStyle,
      ops: this.operations.slice(this.start),
    });
  }
  fill() {}
  closePath() {}
  arc() {}
  setLineDash() {}
}
function fixture(n = 3) {
  const samples = Array.from({ length: n }, (_, i) => ({
    source_row: i + 2,
    timestamp_ns_exact: String(9007199254740993n + BigInt(i)),
    time_valid: true,
    elapsed_s: i === 2 ? 10 : i,
    raw: [i % 2 ? -1 : 1, 0, 0, 0],
    lifted: [1, 0, 0, 0],
    sign_lift: [1, 0, 0, 0],
    normalised: [i % 2 ? -1 : 1, 0, 0, 0],
    body_x: [1, 0, 0],
    body_y: [0, 1, 0],
    body_z: [0, 0, 1],
    stereo: [0, 0, 0],
    angle_rad: i ? 0 : null,
    rate_rad_s: i ? 0 : null,
    geometry_segment: 0,
    stereo_segment: 0,
    pinned_finding: true,
  }));
  const findings = Array.from({ length: 2501 }, (_, i) => ({
    id: "f" + i,
    rule: "QAT-SIGN-001",
    summary: "Raw sign change " + i,
    severity: "medium",
    confidence: "exact",
    source_row_start: 2,
    source_row_end: 3,
    repair_disposition: "proposed",
    evidence: [{ name: "dot", unit: "1", number: -1 }],
  }));
  return {
    report: {
      schema: "quatopsy.report/1",
      result: "findings",
      analysis_id: "bound",
      declarations: { frame_from: "BODY", frame_to: "J2000" },
      findings,
      repairs: [
        {
          id: "repair:sign-lift:1",
          algorithm: "sign-lift",
          disposition: "proposed",
          physical_orientation_equivalent: true,
          affected_rows: [3],
          preconditions: ["finite"],
          numeric_tolerance: 1e-6,
        },
      ],
    },
    view: {
      schema: "quatopsy.view/1",
      analysis_id: "bound",
      samples,
      downsample: {
        source_sample_count: n,
        retained_findings: true,
        exact_finding_endpoints: false,
        retained_extrema: true,
      },
      finding_links: findings.map((f) => ({
        finding_id: f.id,
        geometry_source_row: 2,
        exact_geometry: false,
        context_source_rows: [2, 3],
      })),
      context: [
        { source_row: 2, timestamp_ns: "9007199254740993", raw: [1, 0, 0, 0] },
        { source_row: 3, timestamp_ns: "9007199254740994", raw: [-1, 0, 0, 0] },
      ],
    },
  };
}
function launch(data = fixture(), reduced = false) {
  const elements = new Map();
  const get = (id) => {
    if (!elements.has(id)) elements.set(id, new Element());
    return elements.get(id);
  };
  const html = fs.readFileSync(
    new URL("../crates/quatopsy-cli/viewer/index.html", import.meta.url),
    "utf8",
  );
  for (const match of html.matchAll(/id="([^"]+)"/g)) get(match[1]);
  for (const id of ["physical", "stereo", "timeline", "components"]) {
    get(id).tagName = "CANVAS";
    if (id === "timeline" || id === "components") {
      get(id).width = 1440;
      get(id).height = 240;
    }
  }
  for (const id of ["detail-raw", "detail-lift", "detail-proposed"])
    get(id).append(new Element("th"));
  const values = {
    "trail-axis": "x",
    "stereo-layer": "lifted",
    "timeline-axis": "time",
    "timeline-value": "angle_rad",
    "playback-mode": "step",
  };
  Object.entries(values).forEach(([id, value]) => {
    get(id).value = value;
  });
  get("quatopsy-report").textContent = JSON.stringify(data.report);
  get("quatopsy-view").textContent = JSON.stringify(data.view);
  let scheduled = null;
  let now = 0;
  const document = {
    getElementById: (id) => {
      assert(elements.has(id), "Missing HTML element " + id);
      return elements.get(id);
    },
    createElement: (tag) => new Element(tag),
    createTextNode: (text) =>
      Object.assign(new Element(), { textContent: text }),
    querySelectorAll: () => [...elements.values()],
    addEventListener() {},
    hidden: false,
  };
  vm.runInNewContext(code, {
    document,
    window: {
      matchMedia: () => ({ matches: reduced, addEventListener() {} }),
      addEventListener() {},
    },
    performance: { now: () => now },
    requestAnimationFrame: (fn) => {
      scheduled = fn;
      return 1;
    },
    cancelAnimationFrame: () => {
      scheduled = null;
    },
    console,
  });
  return {
    get,
    advance(ms) {
      now = ms;
      scheduled?.(now);
    },
  };
}
{
  const app = launch();
  const e = app.get;
  assert.equal(e("detail-time").textContent, "9007199254740993");
  assert.match(e("finding-link").textContent, /Approximate/);
  assert.equal(
    e("finding-evidence").children[0].children[1].textContent,
    "-1 1",
  );
  assert.equal(e("findings").children.length, 20);
  for (let i = 0; i < 125; i++) e("findings-next").fire("click");
  assert.equal(e("findings").children.length, 1);
  assert.equal(e("findings").children[0].children[0].dataset.id, "f2500");
  e("finding-search").value = "Raw sign change 2500";
  e("finding-search").fire("input");
  assert.equal(e("findings").children.length, 1);
  let prevented = false;
  e("timeline").fire("keydown", {
    key: "ArrowRight",
    preventDefault() {
      prevented = true;
    },
  });
  assert(prevented);
  assert.equal(e("detail-row").textContent, 3);
  e("repair-select").value = "repair:sign-lift:1";
  e("repair-select").fire("change");
  assert.equal(e("detail-proposed").children[1].className, "changed");
  // Irregular time samples are positioned at 56, 188.8, 1384, not uniformly.
  const layer = e("timeline")
    .ctx.operations.filter((op) => op[0] === "image")
    .at(-1)[1];
  assert(
    layer.ctx.operations.some(
      (op) => op[0] === "move" && Math.abs(op[1] - 188.8) < 1e-6,
    ),
  );
  assert(
    !layer.ctx.operations.some(
      (op) => op[0] === "move" && op[1] === 56 && op[2] === 198,
    ),
  ); // first angle is absent, never zero
  e("playback-mode").value = "time";
  e("playback-mode").fire("change");
  e("play").fire("click");
  app.advance(5000);
  assert.equal(e("detail-row").textContent, 3);
  app.advance(10000);
  assert.equal(e("detail-row").textContent, 4);
}
{
  const data = fixture();
  data.view.samples[1].stereo = null;
  data.view.samples[1].stereo_segment = 1;
  data.view.samples[2].stereo_segment = 1;
  const app = launch(data);
  const layer = app
    .get("stereo")
    .ctx.operations.find((op) => op[0] === "image")[1];
  assert.equal(
    layer.ctx.paths
      .find((p) => p.colour === "#ffc65c")
      .ops.filter((op) => op[0] === "line").length,
    0,
  );
}
{
  const app = launch(fixture(), true);
  app.get("play").fire("click");
  assert.equal(app.get("detail-row").textContent, 3);
  assert.equal(app.get("play").attributes["aria-pressed"], "false");
  const data = fixture();
  data.view.analysis_id = "mismatch";
  const empty = launch(data);
  assert.equal(empty.get("play").disabled, true);
  assert.match(empty.get("finding-link").textContent, /unavailable/);
  const invalid = fixture();
  invalid.report.schema = "quatopsy.report/99";
  const refused = launch(invalid);
  assert.match(refused.get("result-banner").textContent, /refused unknown/);
}
console.log(
  "viewer DOM/canvas simulation: passed (pagination, selection, precision, irregular time, gaps, repair, reduced motion, schema binding)",
);
