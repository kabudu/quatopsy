(function () {
  "use strict";
  const $ = (id) => document.getElementById(id);
  const text = (id, value) => {
    $(id).textContent = value;
  };
  const read = (id) => {
    try {
      return JSON.parse($(id).textContent);
    } catch (_) {
      return null;
    }
  };
  const report = read("quatopsy-report");
  const view = read("quatopsy-view");
  if (
    !report ||
    report.schema !== "quatopsy.report/1" ||
    !["pass", "findings", "refused", "error"].includes(report.result)
  ) {
    text(
      "result-banner",
      "Viewer refused unknown report schema or invalid result. No pass was inferred.",
    );
    document.querySelectorAll("button,input,select").forEach((el) => {
      el.disabled = true;
    });
    return;
  }
  const bound =
    view &&
    view.schema === "quatopsy.view/1" &&
    view.analysis_id === report.analysis_id;
  const samples = bound && Array.isArray(view.samples) ? view.samples : [];
  const findings = report.findings || [];
  const repairs = report.repairs || [];
  const links = new Map(
    (bound ? view.finding_links || [] : []).map((link) => [
      link.finding_id,
      link,
    ]),
  );
  const context = bound ? view.context || [] : [];
  const names = {
    "QAT-SIGN-001": "Quaternion sign discontinuity",
    "QAT-NORM-001": "Quaternion norm defect",
    "QAT-TIME-001": "Timestamp inconsistency",
    "QAT-PI-001": "Near-half-turn ambiguity",
    "QAT-CONV-001": "Convention mismatch",
    "QAT-OMEGA-001": "Body-rate inconsistency",
    "QAT-UNWIND-001": "Commanded-path discrepancy",
  };
  const colours = {
    raw: "#fff1d6",
    derived: "#c778ff",
    proposed: "#c8a4ff",
    projection: "#ffc65c",
    finding: "#ff8d86",
    muted: "#c0b7c9",
  };
  const componentColours = [colours.raw, colours.finding, "#74e6a1", "#78aaff"];
  const finite = (value) => typeof value === "number" && Number.isFinite(value);
  const number = (value) =>
    finite(value)
      ? Math.abs(value) < 0.0001 && value !== 0
        ? value.toExponential(3)
        : Number(value.toPrecision(6)).toString()
      : "Unavailable";
  const exactTime = (sample) =>
    sample
      ? sample.time_valid === false
        ? "Unavailable"
        : sample.timestamp_ns_exact || String(sample.timestamp_ns)
      : "--";
  let selected = 0,
    selectedFinding = null,
    page = 0,
    filtered = findings;
  const pageSize = 20;
  let yaw = 0.55,
    stereoScale = null,
    frame = null,
    playing = false,
    started = 0,
    playbackOrigin = 0;
  const motion = window.matchMedia("(prefers-reduced-motion: reduce)");
  const canvases = ["physical", "stereo", "timeline", "components"];
  const layers = new Map();
  const timeReady =
    bound &&
    view.time_axis_valid !== false &&
    samples.length > 0 &&
    samples.every(
      (s, i) =>
        finite(s.elapsed_s) &&
        s.time_valid !== false &&
        (!i || BigInt(exactTime(s)) > BigInt(exactTime(samples[i - 1]))),
    );
  if (!timeReady) {
    $("timeline-axis").value = "index";
    $("timeline-axis").options[0].disabled = true;
    $("playback-mode").options[1].disabled = true;
  }
  text("case-result", report.result);
  text("metric-findings", findings.length);
  $("result-banner").className = "status-strip " + report.result;
  text(
    "result-banner",
    report.result[0].toUpperCase() +
      report.result.slice(1) +
      " · " +
      findings.length +
      " findings",
  );
  text(
    "metric-samples",
    samples.length +
      " / " +
      (bound ? view.downsample.source_sample_count : 0) +
      " samples displayed",
  );
  const onlySigns =
    findings.length > 0 && findings.every((f) => f.rule === "QAT-SIGN-001");
  text(
    "case-explanation",
    !samples.length
      ? "No bound geometry is available. Canonical findings remain accessible."
      : onlySigns
        ? "Quaternion signs change. Sign changes alone do not change the represented orientation; compare raw and lifted components below."
        : "Select a finding to inspect its canonical evidence, exact source context and separately labelled candidate.",
  );
  text(
    "projection-warning",
    "Stereographic projection artefact, not a physical trajectory.",
  );
  const declarations = report.declarations || {};
  text(
    "frame-label",
    (declarations.frame_from || "Declared source") +
      " → " +
      (declarations.frame_to || "Declared target"),
  );
  const down = bound ? view.downsample : {};
  text(
    "downsample-note",
    "Geometry: " +
      samples.length +
      " of " +
      (down.source_sample_count || 0) +
      ". Finding links retained: " +
      Boolean(down.retained_findings) +
      ". Exact finding endpoints retained: " +
      Boolean(down.exact_finding_endpoints) +
      ". Global angle/rate extrema retained: " +
      Boolean(down.retained_extrema) +
      ".",
  );
  text("analysis-id", "Analysis: " + report.analysis_id);
  $("sample-slider").max = Math.max(0, samples.length - 1);
  function candidate(sample) {
    if (!sample) return null;
    const repair = repairs.find((r) => r.id === $("repair-select").value);
    if (!repair) return null;
    if (repair.algorithm === "sign-lift")
      return sample.sign_lift || sample.proposed;
    if (
      repair.algorithm === "normalise" ||
      repair.algorithm === "normalization" ||
      repair.algorithm === "normalisation"
    )
      return sample.normalised;
    return null;
  }
  repairs
    .filter((r) => r.disposition === "proposed")
    .forEach((r) => {
      const option = document.createElement("option");
      option.value = r.id;
      option.textContent =
        r.algorithm + " · " + r.affected_rows.length + " changed rows";
      $("repair-select").appendChild(option);
    });
  function renderRepair() {
    const root = $("repairs");
    root.replaceChildren();
    const r = repairs.find((r) => r.id === $("repair-select").value);
    if (!r) {
      root.textContent = repairs.length
        ? "Select a named candidate to compare its representation with the source."
        : "No repair candidates in this report.";
      return;
    }
    const title = document.createElement("p");
    title.className = "repair-title";
    title.textContent = r.algorithm + " · " + r.disposition;
    const effect = document.createElement("p");
    effect.className = "repair-data";
    effect.textContent = r.physical_orientation_equivalent
      ? "Physical orientation preserved within the canonical tolerance. Representation values may change."
      : "Physical equivalence is not asserted by this candidate.";
    const details = document.createElement("p");
    details.className = "repair-data";
    details.textContent =
      "Tolerance: " +
      number(r.numeric_tolerance) +
      ". Changed rows: " +
      r.affected_rows.slice(0, 24).join(", ") +
      (r.affected_rows.length > 24
        ? " … (" +
          r.affected_rows.length +
          " total; complete list in canonical report)"
        : "") +
      ". Preconditions: " +
      r.preconditions.join(", ");
    root.append(title, effect, details);
  }
  function renderFindings() {
    const root = $("findings");
    root.replaceChildren();
    page = Math.min(
      page,
      Math.max(0, Math.ceil(filtered.length / pageSize) - 1),
    );
    filtered.slice(page * pageSize, (page + 1) * pageSize).forEach((f) => {
      const li = document.createElement("li");
      li.className = "finding-item";
      const button = document.createElement("button");
      button.type = "button";
      button.dataset.id = f.id;
      button.setAttribute("aria-current", String(selectedFinding === f));
      const title = document.createElement("strong");
      title.textContent = names[f.rule] || f.summary || f.rule;
      const meta = document.createElement("span");
      meta.className = "meta";
      meta.textContent =
        f.rule + " · rows " + f.source_row_start + "–" + f.source_row_end;
      const severity = document.createElement("span");
      severity.className = "severity";
      severity.textContent = f.severity + " · " + f.repair_disposition;
      button.append(title, meta, severity);
      button.addEventListener("click", () => selectFinding(f));
      li.append(button);
      root.append(li);
    });
    if (!filtered.length)
      root.textContent = findings.length
        ? "No matching findings."
        : "No findings in the canonical report.";
    text(
      "findings-page",
      filtered.length
        ? page + 1 + " / " + Math.ceil(filtered.length / pageSize)
        : "0 / 0",
    );
    $("findings-prev").disabled = page === 0;
    $("findings-next").disabled = (page + 1) * pageSize >= filtered.length;
  }
  function nearestRow(row) {
    let lo = 0,
      hi = samples.length;
    while (lo < hi) {
      const mid = (lo + hi) >>> 1;
      if (samples[mid].source_row < row) lo = mid + 1;
      else hi = mid;
    }
    if (lo === samples.length) return Math.max(0, lo - 1);
    if (lo && row - samples[lo - 1].source_row <= samples[lo].source_row - row)
      return lo - 1;
    return lo;
  }
  function selectFinding(f) {
    stop();
    selectedFinding = f;
    text("finding-title", names[f.rule] || f.rule);
    text("finding-summary", f.summary);
    text(
      "finding-meta",
      f.rule +
        " · " +
        f.severity +
        " · confidence: " +
        f.confidence +
        " · rows " +
        f.source_row_start +
        "–" +
        f.source_row_end,
    );
    const link = links.get(f.id);
    text(
      "finding-link",
      !samples.length
        ? "Geometry unavailable; the canonical evidence below is unchanged."
        : link && link.exact_geometry
          ? "Both interval endpoints are retained in the plots."
          : "Approximate geometry link. The selected plotted row may be outside this finding; exact source context is below.",
    );
    const evidence = $("finding-evidence");
    evidence.replaceChildren();
    (f.evidence || []).forEach((e) => {
      const row = document.createElement("div"),
        dt = document.createElement("dt"),
        dd = document.createElement("dd");
      dt.textContent = e.name;
      dd.textContent = String(e.number) + " " + e.unit;
      row.append(dt, dd);
      evidence.append(row);
    });
    const table = document.createElement("table"),
      caption = document.createElement("caption");
    caption.textContent =
      "Source rows around the finding endpoints (unaltered input values)";
    table.append(caption);
    const head = document.createElement("tr");
    ["Row", "Time (ns)", "w, x, y, z"].forEach((label) => {
      const th = document.createElement("th");
      th.scope = "col";
      th.textContent = label;
      head.append(th);
    });
    table.append(head);
    const contextRows = new Set(link ? link.context_source_rows || [] : []);
    const nearby = context.filter((s) => contextRows.has(s.source_row));
    nearby.forEach((s) => {
      const tr = document.createElement("tr");
      [
        s.source_row,
        s.timestamp_ns,
        s.raw ? s.raw.map(String).join(", ") : "Unavailable",
      ].forEach((value) => {
        const td = document.createElement("td");
        td.textContent = value;
        tr.append(td);
      });
      table.append(tr);
    });
    $("source-context").replaceChildren(
      nearby.length
        ? table
        : document.createTextNode(
            "Source context unavailable for this report-only bundle.",
          ),
    );
    renderFindings();
    setSelected(
      nearestRow(
        link && link.geometry_source_row !== null
          ? link.geometry_source_row
          : f.source_row_start,
      ),
    );
  }
  function rowValues(id, values, raw) {
    const row = $(id);
    while (row.children.length > 1) row.lastChild.remove();
    for (let i = 0; i < 4; i++) {
      const td = document.createElement("td");
      td.textContent = values ? number(values[i]) : "--";
      td.title = values ? String(values[i]) : "Unavailable";
      if (values && raw && values[i] !== raw[i]) {
        td.className = "changed";
        td.setAttribute(
          "aria-label",
          ["w", "x", "y", "z"][i] + " changed to " + values[i],
        );
      }
      row.append(td);
    }
  }
  function details() {
    const s = samples[selected];
    text("detail-row", s ? s.source_row : "--");
    text("metric-row", s ? "Row " + s.source_row : "No sample");
    text("detail-time", exactTime(s));
    text("metric-time", s ? exactTime(s) + " ns" : "--");
    text(
      "sample-position",
      samples.length ? selected + 1 + " / " + samples.length : "0 / 0",
    );
    rowValues("detail-raw", s && s.raw);
    rowValues("detail-lift", s && s.lifted, s && s.raw);
    rowValues("detail-proposed", candidate(s), s && s.raw);
    if (!playing)
      text(
        "selection",
        s
          ? "Selected source row " +
              s.source_row +
              ", timestamp " +
              exactTime(s) +
              " nanoseconds."
          : "No geometry available.",
      );
  }
  function project(v) {
    const c = Math.cos(yaw),
      s = Math.sin(yaw);
    return [v[0] * c - v[2] * s, -v[1] * 0.9 - (v[0] * s + v[2] * c) * 0.25];
  }
  function point(canvas, v, scale) {
    const p = project(v);
    return [canvas.width / 2 + p[0] * scale, canvas.height / 2 + p[1] * scale];
  }
  function axisFrom(q, axis) {
    if (!q) return null;
    const n = Math.hypot(...q);
    if (!n) return null;
    const [w, x, y, z] = q.map((v) => v / n);
    return axis === "x"
      ? [1 - 2 * (y * y + z * z), 2 * (x * y + w * z), 2 * (x * z - w * y)]
      : axis === "y"
        ? [2 * (x * y - w * z), 1 - 2 * (x * x + z * z), 2 * (y * z + w * x)]
        : [2 * (x * z + w * y), 2 * (y * z - w * x), 1 - 2 * (x * x + y * y)];
  }
  function stereoValue(sample) {
    if ($("stereo-layer").value === "lifted") return sample.stereo;
    const q = sample.raw;
    if (!q) return null;
    const norm = Math.hypot(...q);
    if (!norm) return null;
    const denominator = 1 + q[0] / norm;
    return Math.abs(denominator) <= 1e-12
      ? null
      : q.slice(1).map((v) => v / norm / denominator);
  }
  function grid(ctx, canvas) {
    ctx.fillStyle = "#100e16";
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    ctx.strokeStyle = "#241e2e";
    ctx.lineWidth = 1;
    ctx.beginPath();
    for (let x = 40; x < canvas.width; x += 80) {
      ctx.moveTo(x, 0);
      ctx.lineTo(x, canvas.height);
    }
    for (let y = 40; y < canvas.height; y += 80) {
      ctx.moveTo(0, y);
      ctx.lineTo(canvas.width, y);
    }
    ctx.stroke();
  }
  function reticle(ctx, p) {
    ctx.strokeStyle = colours.raw;
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.arc(p[0], p[1], 7, 0, 2 * Math.PI);
    ctx.stroke();
  }
  function path(ctx, points, colour, dash) {
    ctx.strokeStyle = colour;
    ctx.lineWidth = 2;
    ctx.setLineDash(dash || []);
    ctx.beginPath();
    let previous = null;
    points.forEach((p) => {
      if (!p) {
        previous = null;
        return;
      }
      if (previous && previous[2] === p[2]) ctx.lineTo(p[0], p[1]);
      else ctx.moveTo(p[0], p[1]);
      previous = p;
    });
    ctx.stroke();
    ctx.setLineDash([]);
  }
  function marker(ctx, p) {
    ctx.fillStyle = colours.finding;
    ctx.beginPath();
    ctx.moveTo(p[0], p[1] - 5);
    ctx.lineTo(p[0] + 5, p[1]);
    ctx.lineTo(p[0], p[1] + 5);
    ctx.lineTo(p[0] - 5, p[1]);
    ctx.closePath();
    ctx.fill();
  }
  function xAt(index, canvas) {
    const pad = 56;
    if ($("timeline-axis").value === "time" && timeReady) {
      const total =
        samples[samples.length - 1].elapsed_s - samples[0].elapsed_s;
      return (
        pad +
        (total
          ? (samples[index].elapsed_s - samples[0].elapsed_s) / total
          : 0) *
          (canvas.width - 2 * pad)
      );
    }
    return (
      pad + (index / Math.max(1, samples.length - 1)) * (canvas.width - 2 * pad)
    );
  }
  let maxValue = 1,
    fittedScale = 86;
  function rebuild() {
    layers.clear();
    canvases.forEach((id) => {
      const canvas = $(id),
        layer = document.createElement("canvas");
      layer.width = canvas.width;
      layer.height = canvas.height;
      const ctx = layer.getContext("2d");
      grid(ctx, layer);
      layers.set(id, layer);
    });
    const physical = $("physical"),
      pc = layers.get("physical").getContext("2d"),
      axis = $("trail-axis").value;
    path(
      pc,
      samples.map((s) =>
        s["body_" + axis]
          ? [...point(physical, s["body_" + axis], 155), s.geometry_segment]
          : null,
      ),
      colours.derived,
    );
    path(
      pc,
      samples.map((s) => {
        const v = axisFrom(candidate(s), axis);
        return v ? [...point(physical, v, 155), s.geometry_segment] : null;
      }),
      colours.proposed,
      [5, 5],
    );
    const stereo = $("stereo"),
      sc = layers.get("stereo").getContext("2d");
    const stereoPoints = samples.map(stereoValue);
    const extent = stereoPoints.reduce(
      (max, v) => (v ? Math.max(max, ...project(v).map(Math.abs)) : max),
      0,
    );
    fittedScale =
      stereoScale === null
        ? Math.min(
            150,
            (Math.min(stereo.width, stereo.height) / 2 - 35) /
              Math.max(extent, 0.01),
          )
        : stereoScale;
    path(
      sc,
      stereoPoints.map((v, i) =>
        v
          ? [
              ...point(stereo, v, fittedScale),
              $("stereo-layer").value === "raw"
                ? samples[i].raw_stereo_segment
                : samples[i].stereo_segment,
            ]
          : null,
      ),
      colours.projection,
    );
    stereoPoints.forEach((v, i) => {
      if (v && samples[i].pinned_finding)
        marker(sc, point(stereo, v, fittedScale));
    });
    const timeline = $("timeline"),
      tc = layers.get("timeline").getContext("2d"),
      key = $("timeline-value").value;
    maxValue = samples.reduce(
      (max, s) => (finite(s[key]) ? Math.max(max, s[key]) : max),
      0,
    );
    if (maxValue === 0) maxValue = 1;
    path(
      tc,
      samples.map((s, i) =>
        finite(s[key])
          ? [
              xAt(i, timeline),
              timeline.height -
                42 -
                (s[key] / maxValue) * (timeline.height - 75),
              s.geometry_segment,
            ]
          : null,
      ),
      colours.derived,
    );
    tc.fillStyle = colours.muted;
    tc.font = "18px system-ui";
    tc.fillText(
      number(maxValue) + (key === "angle_rad" ? " rad" : " rad/s"),
      8,
      22,
    );
    tc.fillText("0", 12, timeline.height - 35);
    if (samples.length) {
      tc.fillText(
        $("timeline-axis").value === "time" ? "0 s" : "1",
        56,
        timeline.height - 10,
      );
      tc.textAlign = "right";
      tc.fillText(
        $("timeline-axis").value === "time"
          ? number(samples[samples.length - 1].elapsed_s) + " s"
          : String(samples.length),
        timeline.width - 56,
        timeline.height - 10,
      );
      tc.textAlign = "left";
    }
    samples.forEach((s, i) => {
      if (s.pinned_finding)
        marker(tc, [
          xAt(i, timeline),
          finite(s[key])
            ? timeline.height -
              42 -
              (s[key] / maxValue) * (timeline.height - 75)
            : timeline.height - 28,
        ]);
    });
    const comp = $("components"),
      cc = layers.get("components").getContext("2d");
    const extentQ = samples.reduce(
      (max, s) =>
        Math.max(
          max,
          ...[s.raw, s.lifted, candidate(s)].flatMap((q) =>
            q ? q.map(Math.abs) : [],
          ),
        ),
      1,
    );
    for (let j = 0; j < 4; j++)
      for (const [kind, dash] of [
        ["raw", []],
        ["lifted", [7, 5]],
        ["candidate", [2, 5]],
      ])
        path(
          cc,
          samples.map((s, i) => {
            const q = kind === "candidate" ? candidate(s) : s[kind];
            return q
              ? [
                  xAt(i, comp),
                  comp.height / 2 - (q[j] / extentQ) * (comp.height / 2 - 28),
                  s.geometry_segment,
                ]
              : null;
          }),
          componentColours[j],
          dash,
        );
    cc.fillStyle = colours.muted;
    cc.font = "18px system-ui";
    cc.fillText("+" + number(extentQ), 6, 22);
    cc.fillText("−" + number(extentQ), 6, comp.height - 10);
    paint();
  }
  function paint() {
    canvases.forEach((id) => {
      const c = $(id),
        ctx = c.getContext("2d");
      ctx.clearRect(0, 0, c.width, c.height);
      ctx.drawImage(layers.get(id), 0, 0);
      if (!samples.length) {
        ctx.fillStyle = colours.muted;
        ctx.font = "20px system-ui";
        ctx.fillText("No bound geometry", 40, c.height / 2);
      }
    });
    const s = samples[selected];
    if (!s) {
      text("pole-info", "No geometry available.");
      return;
    }
    const canvas = $("physical"),
      ctx = canvas.getContext("2d");
    ["x", "y", "z"].forEach((axis, i) => {
      const v = s["body_" + axis];
      if (!v) return;
      const p = point(canvas, v, 135);
      ctx.strokeStyle = componentColours[i + 1];
      ctx.lineWidth = 3;
      ctx.beginPath();
      ctx.moveTo(canvas.width / 2, canvas.height / 2);
      ctx.lineTo(...p);
      ctx.stroke();
      ctx.fillStyle = componentColours[i + 1];
      ctx.font = "18px system-ui";
      ctx.fillText("+" + axis.toUpperCase(), p[0] + 7, p[1] - 7);
    });
    const v = stereoValue(s);
    if (v)
      reticle($("stereo").getContext("2d"), point($("stereo"), v, fittedScale));
    const q = $("stereo-layer").value === "raw" ? s.raw : s.lifted;
    const pole = q ? 1 + q[0] / Math.hypot(...q) : null;
    text(
      "pole-info",
      v
        ? "Pole denominator: " +
            number(pole) +
            ". Scale: " +
            number(fittedScale) +
            " px/unit."
        : "Selected sample is unavailable or at the projection pole; no point is drawn.",
    );
    ["timeline", "components"].forEach((id) => {
      const c = $(id),
        cx = c.getContext("2d"),
        x = xAt(selected, c);
      cx.strokeStyle = colours.raw;
      cx.lineWidth = 1;
      cx.setLineDash([4, 4]);
      cx.beginPath();
      cx.moveTo(x, 26);
      cx.lineTo(x, c.height - 30);
      cx.stroke();
      cx.setLineDash([]);
    });
  }
  function setSelected(index) {
    selected = Math.max(0, Math.min(Math.max(0, samples.length - 1), index));
    $("sample-slider").value = selected;
    details();
    paint();
  }
  function stop() {
    playing = false;
    if (frame !== null) cancelAnimationFrame(frame);
    frame = null;
    $("play").setAttribute("aria-pressed", "false");
    $("play").setAttribute("aria-label", "Play trajectory");
    text("play", "Play");
  }
  function tick(now) {
    if (!playing) return;
    let next = selected;
    if ($("playback-mode").value === "time") {
      const target = samples[playbackOrigin].elapsed_s + (now - started) / 1000;
      let lo = playbackOrigin,
        hi = samples.length;
      while (lo < hi) {
        const mid = (lo + hi) >>> 1;
        if (samples[mid].elapsed_s <= target) lo = mid + 1;
        else hi = mid;
      }
      next = Math.max(playbackOrigin, lo - 1);
    } else
      next = Math.min(
        samples.length - 1,
        playbackOrigin + Math.floor((now - started) / 240),
      );
    if (next !== selected) setSelected(next);
    if (selected === samples.length - 1) {
      stop();
      details();
    } else frame = requestAnimationFrame(tick);
  }
  function toggle() {
    if (playing) {
      stop();
      details();
      return;
    }
    if (samples.length < 2) return;
    if (motion.matches) {
      setSelected(Math.min(samples.length - 1, selected + 1));
      return;
    }
    if (selected === samples.length - 1) setSelected(0);
    playing = true;
    started = performance.now();
    playbackOrigin = selected;
    $("play").setAttribute("aria-pressed", "true");
    $("play").setAttribute("aria-label", "Pause trajectory");
    text("play", "Pause");
    frame = requestAnimationFrame(tick);
  }
  $("findings-prev").addEventListener("click", () => {
    page--;
    renderFindings();
  });
  $("findings-next").addEventListener("click", () => {
    page++;
    renderFindings();
  });
  $("finding-search").addEventListener("input", () => {
    const term = $("finding-search").value.toLowerCase();
    filtered = findings.filter((f) =>
      [
        f.rule,
        f.summary,
        names[f.rule] || "",
        String(f.source_row_start),
        String(f.source_row_end),
      ]
        .join(" ")
        .toLowerCase()
        .includes(term),
    );
    page = 0;
    renderFindings();
  });
  $("sample-slider").addEventListener("input", () => {
    stop();
    setSelected(Number($("sample-slider").value));
  });
  $("step-back").addEventListener("click", () => {
    stop();
    setSelected(selected - 1);
  });
  $("step-forward").addEventListener("click", () => {
    stop();
    setSelected(selected + 1);
  });
  $("play").addEventListener("click", toggle);
  $("timeline").addEventListener("click", (event) => {
    if (!samples.length) return;
    const rect = $("timeline").getBoundingClientRect();
    const x = ((event.clientX - rect.left) / rect.width) * $("timeline").width;
    let best = 0;
    for (let i = 1; i < samples.length; i++)
      if (
        Math.abs(xAt(i, $("timeline")) - x) <
        Math.abs(xAt(best, $("timeline")) - x)
      )
        best = i;
    stop();
    setSelected(best);
  });
  $("timeline").addEventListener("keydown", (event) => {
    const moves = {
      ArrowRight: selected + 1,
      ArrowLeft: selected - 1,
      Home: 0,
      End: samples.length - 1,
    };
    if (event.key in moves) {
      event.preventDefault();
      stop();
      setSelected(moves[event.key]);
    }
  });
  ["trail-axis", "stereo-layer", "timeline-axis", "timeline-value"].forEach(
    (id) =>
      $(id).addEventListener("change", () => {
        stop();
        rebuild();
      }),
  );
  $("repair-select").addEventListener("change", () => {
    stop();
    renderRepair();
    details();
    rebuild();
  });
  $("playback-mode").addEventListener("change", stop);
  $("rotate-left").addEventListener("click", () => {
    yaw -= Math.PI / 12;
    rebuild();
  });
  $("rotate-right").addEventListener("click", () => {
    yaw += Math.PI / 12;
    rebuild();
  });
  $("camera-reset").addEventListener("click", () => {
    yaw = 0.55;
    rebuild();
  });
  $("stereo-fit").addEventListener("click", () => {
    stereoScale = null;
    rebuild();
  });
  $("stereo-reset").addEventListener("click", () => {
    stereoScale = 86;
    rebuild();
  });
  window.addEventListener("pagehide", stop);
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) stop();
  });
  motion.addEventListener("change", () => {
    stop();
    $("play").title = motion.matches
      ? "Reduced motion: advance one sample"
      : "";
  });
  ["play", "step-back", "step-forward", "sample-slider"].forEach((id) => {
    $(id).disabled = samples.length < 2;
  });
  renderRepair();
  renderFindings();
  rebuild();
  setSelected(0);
  if (findings.length) selectFinding(findings[0]);
})();
