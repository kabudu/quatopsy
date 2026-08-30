(function () {
  "use strict";

  function byId(id) { return document.getElementById(id); }
  function text(el, value) { if (el) { el.textContent = value; } }
  function readJson(id) {
    var node = byId(id);
    if (!node) { return null; }
    try { return JSON.parse(node.textContent); } catch (err) { return null; }
  }
  function schemaMajor(schema) {
    if (typeof schema !== "string") { return null; }
    var parts = schema.split("/");
    return parts.length === 2 ? parts[0] + "/" + parts[1] : schema;
  }
  function valueOr(value, fallback) { return value === null || value === undefined ? fallback : value; }
  function tuple(values, fallback) {
    return Array.isArray(values) ? values.map(function (v) { return Number(v).toPrecision(7); }).join("  ") : fallback;
  }

  var report = readJson("quatopsy-report");
  var view = readJson("quatopsy-view") || { samples: [], projection_warning: "No derived geometry is available.", downsample: {} };
  var banner = byId("result-banner");
  var findingsEl = byId("findings");
  var repairsEl = byId("repairs");
  var selectionEl = byId("selection");
  var slider = byId("sample-slider");
  var physical = byId("physical");
  var stereo = byId("stereo");
  var timeline = byId("timeline");
  var playButton = byId("play");
  var reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  var selected = 0;
  var playing = false;
  var playTimer = null;
  var findingLinks = Array.isArray(view.finding_links) ? view.finding_links : [];
  var maxRenderedFindings = 2000;
  var samples = Array.isArray(view.samples) ? view.samples : [];
  var reportFindings = report && Array.isArray(report.findings) ? report.findings : [];

  if (!report || schemaMajor(report.schema) !== "quatopsy.report/1") {
    var unknown = report && report.schema ? String(report.schema) : "missing schema";
    text(banner, "Viewer refused unknown report schema " + unknown + ". The file was not interpreted as a pass.");
    banner.className = "status-strip error";
    text(byId("case-result"), "Protocol refused");
    text(byId("projection-warning"), "No geometry is shown for an unsupported protocol.");
    document.querySelectorAll("button, input").forEach(function (control) { control.disabled = true; });
    return;
  }

  var result = report.result || "error";
  banner.className = "status-strip " + result;
  text(banner, "Canonical report: " + result.toUpperCase() + " / the viewer did not recompute rules");
  text(byId("case-result"), result);
  text(byId("metric-findings"), String(reportFindings.length));
  text(byId("analysis-id"), "ANALYSIS / " + valueOr(report.analysis_id, "unavailable"));
  text(byId("projection-warning"), view.projection_warning || "The S^3 panel is a stereographic projection artefact, not a physical trajectory.");
  var down = view.downsample || {};
  text(byId("metric-samples"), String(valueOr(down.emitted_sample_count, 0)) + " / " + String(valueOr(down.source_sample_count, 0)));
  text(byId("downsample-note"), "Bounded display geometry: " + String(valueOr(down.emitted_sample_count, 0)) + " of " + String(valueOr(down.source_sample_count, 0)) + " source samples. Finding links retained: " + String(valueOr(down.retained_findings, false)) + ". Extrema retained: " + String(valueOr(down.retained_extrema, false)) + ".");

  slider.max = String(Math.max(0, samples.length - 1));
  slider.value = "0";
  if (samples.length < 2) { playButton.disabled = true; }

  function sampleLabel(sample) {
    if (!sample) { return "No sample selected."; }
    return "Sample identity source_row " + sample.source_row + ", t_ns " + sample.timestamp_ns + ". Raw (measured): " + tuple(sample.raw, "unavailable") + ". Derived lift: " + tuple(sample.lifted, "unavailable") + ". Proposed repair: " + tuple(sample.proposed, "none") + ".";
  }

  function project3(point) {
    return [point[0] * 0.82 - point[2] * 0.48, -point[1] * 0.9 - point[0] * 0.12 - point[2] * 0.22];
  }
  function pointFor(canvas, point, scale) {
    var p = project3(point);
    return [canvas.width / 2 + p[0] * scale, canvas.height / 2 + p[1] * scale];
  }
  function drawGrid(ctx, canvas, spacing) {
    ctx.save();
    ctx.strokeStyle = "rgba(98,230,223,0.08)";
    ctx.lineWidth = 1;
    for (var x = 0; x < canvas.width; x += spacing) { ctx.beginPath(); ctx.moveTo(x, 0); ctx.lineTo(x, canvas.height); ctx.stroke(); }
    for (var y = 0; y < canvas.height; y += spacing) { ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(canvas.width, y); ctx.stroke(); }
    ctx.restore();
  }
  function glowStroke(ctx, colour, width) {
    ctx.strokeStyle = colour; ctx.lineWidth = width; ctx.shadowColor = colour; ctx.shadowBlur = reduced ? 0 : 10;
  }
  function drawPath(ctx, canvas, key, colour, scale, dashed) {
    ctx.save(); glowStroke(ctx, colour, 2); ctx.setLineDash(dashed ? [7, 7] : []); ctx.beginPath();
    var started = false;
    samples.forEach(function (sample) {
      if (!sample[key]) { return; }
      var p = pointFor(canvas, sample[key], scale);
      if (started) { ctx.lineTo(p[0], p[1]); } else { ctx.moveTo(p[0], p[1]); started = true; }
    });
    ctx.stroke(); ctx.restore();
  }
  function drawReticle(ctx, x, y, colour) {
    ctx.save(); ctx.strokeStyle = colour; ctx.lineWidth = 1.5; ctx.shadowColor = colour; ctx.shadowBlur = reduced ? 0 : 12;
    ctx.beginPath(); ctx.arc(x, y, 9, 0, Math.PI * 2); ctx.moveTo(x - 16, y); ctx.lineTo(x - 6, y); ctx.moveTo(x + 6, y); ctx.lineTo(x + 16, y); ctx.moveTo(x, y - 16); ctx.lineTo(x, y - 6); ctx.moveTo(x, y + 6); ctx.lineTo(x, y + 16); ctx.stroke(); ctx.restore();
  }

  function drawPhysical() {
    var ctx = physical.getContext("2d");
    ctx.clearRect(0, 0, physical.width, physical.height); drawGrid(ctx, physical, 46);
    ctx.save(); ctx.strokeStyle = "rgba(155,171,180,0.28)"; ctx.setLineDash([3, 8]);
    [60, 120, 180].forEach(function (radius) { ctx.beginPath(); ctx.ellipse(physical.width / 2, physical.height / 2, radius, radius * 0.55, -0.18, 0, Math.PI * 2); ctx.stroke(); }); ctx.restore();
    drawPath(ctx, physical, "body_x", "#62e6df", 175, false);
    drawPath(ctx, physical, "proposed_body_x", "#c8a4ff", 175, true);
    var cur = samples[selected];
    if (cur && cur.body_x && cur.body_y && cur.body_z) {
      var origin = [physical.width / 2, physical.height / 2];
      [[cur.body_x, "#ff7770", "+X"], [cur.body_y, "#74e6a1", "+Y"], [cur.body_z, "#78aaff", "+Z"]].forEach(function (axis) {
        var p = project3(axis[0]); var end = [origin[0] + p[0] * 145, origin[1] + p[1] * 145];
        ctx.save(); glowStroke(ctx, axis[1], 3); ctx.beginPath(); ctx.moveTo(origin[0], origin[1]); ctx.lineTo(end[0], end[1]); ctx.stroke(); ctx.shadowBlur = 0; ctx.fillStyle = axis[1]; ctx.font = "bold 14px ui-monospace, monospace"; ctx.fillText(axis[2], end[0] + 8, end[1] + 4); ctx.restore();
      });
      var currentPoint = pointFor(physical, cur.body_x, 175); drawReticle(ctx, currentPoint[0], currentPoint[1], "#eef4f6");
    }
  }

  function drawStereo() {
    var ctx = stereo.getContext("2d");
    ctx.clearRect(0, 0, stereo.width, stereo.height); drawGrid(ctx, stereo, 46);
    var centre = [stereo.width / 2, stereo.height / 2];
    ctx.save(); ctx.strokeStyle = "rgba(255,198,92,0.2)"; ctx.lineWidth = 1;
    [58, 116, 174].forEach(function (radius) { ctx.beginPath(); ctx.arc(centre[0], centre[1], radius, 0, Math.PI * 2); ctx.stroke(); });
    ctx.beginPath(); ctx.moveTo(centre[0], 30); ctx.lineTo(centre[0], stereo.height - 30); ctx.moveTo(30, centre[1]); ctx.lineTo(stereo.width - 30, centre[1]); ctx.stroke(); ctx.restore();
    drawPath(ctx, stereo, "stereo", "#ffc65c", 86, false);
    samples.forEach(function (sample, idx) {
      if (!sample.stereo || (!sample.pinned_finding && idx !== selected)) { return; }
      var p = pointFor(stereo, sample.stereo, 86);
      if (sample.pinned_finding) { ctx.save(); ctx.fillStyle = "#ff7770"; ctx.translate(p[0], p[1]); ctx.rotate(Math.PI / 4); ctx.fillRect(-4, -4, 8, 8); ctx.restore(); }
      if (idx === selected) { drawReticle(ctx, p[0], p[1], "#eef4f6"); }
    });
  }

  function timelinePoint(index, maxAngle) {
    var padX = 50, padY = 34;
    return [padX + (index / Math.max(samples.length - 1, 1)) * (timeline.width - padX * 2), timeline.height - padY - (valueOr(samples[index].angle_rad, 0) / maxAngle) * (timeline.height - padY * 2)];
  }
  function drawTimeline() {
    var ctx = timeline.getContext("2d"); ctx.clearRect(0, 0, timeline.width, timeline.height); drawGrid(ctx, timeline, 52);
    if (samples.length === 0) { ctx.fillStyle = "#9babb4"; ctx.fillText("NO GEOMETRY AVAILABLE", 48, timeline.height / 2); return; }
    var maxAngle = samples.reduce(function (max, sample) { return Math.max(max, valueOr(sample.angle_rad, 0)); }, 0.001);
    ctx.save(); ctx.fillStyle = "#6c7d86"; ctx.font = "18px ui-monospace, monospace"; ctx.fillText(maxAngle.toFixed(4) + " rad", 18, 24); ctx.fillText("0", 18, timeline.height - 14); ctx.restore();
    ctx.save(); glowStroke(ctx, "#62e6df", 3); ctx.beginPath();
    samples.forEach(function (_sample, idx) { var p = timelinePoint(idx, maxAngle); if (idx) { ctx.lineTo(p[0], p[1]); } else { ctx.moveTo(p[0], p[1]); } }); ctx.stroke(); ctx.restore();
    samples.forEach(function (sample, idx) {
      var p = timelinePoint(idx, maxAngle);
      if (sample.pinned_finding) { ctx.save(); ctx.translate(p[0], p[1]); ctx.rotate(Math.PI / 4); ctx.fillStyle = "#ff7770"; ctx.fillRect(-6, -6, 12, 12); ctx.restore(); }
    });
    var selectedPoint = timelinePoint(selected, maxAngle);
    ctx.save(); ctx.strokeStyle = "rgba(238,244,246,0.7)"; ctx.setLineDash([4, 5]); ctx.beginPath(); ctx.moveTo(selectedPoint[0], 0); ctx.lineTo(selectedPoint[0], timeline.height); ctx.stroke(); ctx.restore();
    drawReticle(ctx, selectedPoint[0], selectedPoint[1], "#eef4f6");
  }

  function updateFindingCurrent() {
    findingsEl.querySelectorAll("button[data-start]").forEach(function (button) {
      var start = Number(button.dataset.start), end = Number(button.dataset.end), row = samples[selected] ? samples[selected].source_row : -1;
      button.dataset.active = row >= start && row <= end ? "true" : "false";
    });
  }
  function renderLists() {
    findingsEl.textContent = "";
    reportFindings.slice(0, maxRenderedFindings).forEach(function (finding, idx) {
      var item = document.createElement("li"); item.className = "finding-item";
      var button = document.createElement("button"); button.type = "button"; button.dataset.start = String(finding.source_row_start); button.dataset.end = String(finding.source_row_end);
      var sequence = document.createElement("span"); sequence.className = "finding-sequence"; text(sequence, String(idx + 1).padStart(2, "0"));
      var copy = document.createElement("span"); copy.className = "finding-copy";
      var title = document.createElement("strong"); text(title, finding.rule); var reason = document.createElement("span"); text(reason, finding.reason_code + " / " + finding.repair_disposition); copy.appendChild(title); copy.appendChild(reason);
      var range = document.createElement("span"); range.className = "finding-range"; text(range, "ROWS " + finding.source_row_start + "-" + finding.source_row_end);
      button.appendChild(sequence); button.appendChild(copy); button.appendChild(range); button.addEventListener("click", function () { selectFinding(finding); }); item.appendChild(button); findingsEl.appendChild(item);
    });
    if (reportFindings.length === 0) { var noFindings = document.createElement("li"); noFindings.className = "repair-item"; text(noFindings, "No findings in the canonical report."); findingsEl.appendChild(noFindings); }
    if (reportFindings.length > maxRenderedFindings) { var truncated = document.createElement("li"); truncated.className = "repair-item"; text(truncated, String(reportFindings.length - maxRenderedFindings) + " additional findings remain in the canonical report and were omitted from the bounded DOM rendering."); findingsEl.appendChild(truncated); }
    repairsEl.textContent = "";
    (report.repairs || []).forEach(function (repair) { var item = document.createElement("li"); item.className = "repair-item"; text(item, repair.id + " / " + repair.algorithm + " / " + repair.disposition + " / affected rows " + (repair.affected_rows || []).join(", ")); repairsEl.appendChild(item); });
    if ((report.repairs || []).length === 0) { var empty = document.createElement("li"); empty.className = "repair-item"; text(empty, "No proposed repairs."); repairsEl.appendChild(empty); }
  }

  function selectByRow(row) {
    var best = 0, distance = Infinity;
    samples.forEach(function (sample, idx) { var next = Math.abs(sample.source_row - row); if (next < distance) { distance = next; best = idx; } });
    setSelected(best);
  }
  function selectFinding(finding) {
    var link = findingLinks.find(function (candidate) { return candidate.finding_id === finding.id; });
    selectByRow(link && typeof link.geometry_source_row === "number" ? link.geometry_source_row : finding.source_row_start);
  }
  function updateDetails(sample) {
    text(selectionEl, sampleLabel(sample));
    text(byId("detail-row"), sample ? String(sample.source_row) : "--"); text(byId("metric-row"), sample ? String(sample.source_row) : "--");
    text(byId("detail-time"), sample ? String(sample.timestamp_ns) + " ns" : "--"); text(byId("metric-time"), sample ? String(sample.timestamp_ns) + " ns" : "--");
    text(byId("detail-raw"), sample ? tuple(sample.raw, "Unavailable") : "Unavailable"); text(byId("detail-lift"), sample ? tuple(sample.lifted, "Unavailable") : "Unavailable"); text(byId("detail-proposed"), sample ? tuple(sample.proposed, "None") : "None");
  }
  function setSelected(index) {
    if (samples.length === 0) { updateDetails(null); drawPhysical(); drawStereo(); drawTimeline(); return; }
    selected = Math.max(0, Math.min(samples.length - 1, index)); slider.value = String(selected); text(byId("sample-position"), String(selected + 1) + " / " + String(samples.length));
    updateDetails(samples[selected]); updateFindingCurrent(); drawPhysical(); drawStereo(); drawTimeline();
  }
  function stopPlayback() {
    playing = false; if (playTimer !== null) { window.clearInterval(playTimer); playTimer = null; }
    playButton.setAttribute("aria-pressed", "false"); playButton.setAttribute("aria-label", "Play trajectory"); playButton.innerHTML = '<span aria-hidden="true">&#9654;</span> Play';
  }
  function togglePlayback() {
    if (playing) { stopPlayback(); return; }
    if (reduced || samples.length < 2) { setSelected(selected + 1 >= samples.length ? 0 : selected + 1); return; }
    playing = true; playButton.setAttribute("aria-pressed", "true"); playButton.setAttribute("aria-label", "Pause trajectory"); playButton.innerHTML = '<span aria-hidden="true">&#10074;&#10074;</span> Pause';
    playTimer = window.setInterval(function () { if (selected >= samples.length - 1) { stopPlayback(); } else { setSelected(selected + 1); } }, 240);
  }

  slider.addEventListener("input", function () { stopPlayback(); setSelected(Number(slider.value)); });
  byId("step-back").addEventListener("click", function () { stopPlayback(); setSelected(selected - 1); });
  byId("step-forward").addEventListener("click", function () { stopPlayback(); setSelected(selected + 1); });
  playButton.addEventListener("click", togglePlayback);
  timeline.addEventListener("click", function (event) {
    if (!samples.length) { return; }
    var rect = timeline.getBoundingClientRect(); var ratio = Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width)); stopPlayback(); setSelected(Math.round(ratio * (samples.length - 1)));
  });
  document.addEventListener("keydown", function (event) {
    if (event.target && (event.target.tagName === "INPUT" || event.target.tagName === "BUTTON")) { return; }
    if (event.key === "ArrowRight") { stopPlayback(); setSelected(selected + 1); }
    else if (event.key === "ArrowLeft") { stopPlayback(); setSelected(selected - 1); }
    else if (event.key === "Home") { stopPlayback(); setSelected(0); }
    else if (event.key === "End") { stopPlayback(); setSelected(samples.length - 1); }
  });
  window.addEventListener("pagehide", stopPlayback);

  renderLists(); setSelected(0);
}());
