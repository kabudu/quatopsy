(function () {
  "use strict";

  function text(el, value) {
    el.textContent = value;
  }

  function readJson(id) {
    var node = document.getElementById(id);
    if (!node) {
      return null;
    }
    try {
      return JSON.parse(node.textContent);
    } catch (err) {
      return null;
    }
  }

  function schemaMajor(schema) {
    if (typeof schema !== "string") {
      return null;
    }
    var parts = schema.split("/");
    return parts.length === 2 ? parts[0] + "/" + parts[1] : schema;
  }

  var report = readJson("quatopsy-report");
  var view = readJson("quatopsy-view") || {
    samples: [],
    projection_warning: "No derived geometry is available.",
    downsample: {},
  };
  var banner = document.getElementById("result-banner");
  var findingsEl = document.getElementById("findings");
  var repairsEl = document.getElementById("repairs");
  var selectionEl = document.getElementById("selection");
  var slider = document.getElementById("sample-slider");
  var physical = document.getElementById("physical");
  var stereo = document.getElementById("stereo");
  var timeline = document.getElementById("timeline");
  var reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  var selected = 0;

  if (!report || schemaMajor(report.schema) !== "quatopsy.report/1") {
    var unknown = report && report.schema ? String(report.schema) : "missing schema";
    text(
      banner,
      "Viewer refused unknown report schema " +
        unknown +
        ". The file was not interpreted as a pass."
    );
    banner.className = "error";
    text(document.getElementById("projection-warning"), "No geometry is shown for an unsupported protocol.");
    return;
  }

  var result = report.result || "error";
  banner.className = result;
  text(
    banner,
    "Report result: " +
      result +
      " (from the canonical report; the viewer did not recompute rules)."
  );
  text(
    document.getElementById("projection-warning"),
    view.projection_warning ||
      "The S^3 panel is a stereographic projection artefact, not a physical trajectory."
  );
  var down = view.downsample || {};
  text(
    document.getElementById("downsample-note"),
    "Downsample " +
      (down.emitted_sample_count || 0) +
      " of " +
      (down.source_sample_count || 0) +
      " samples. Findings retained: " +
      String(down.retained_findings) +
      ". Extrema retained: " +
      String(down.retained_extrema) +
      "."
  );

  var samples = Array.isArray(view.samples) ? view.samples : [];
  if (slider) {
    slider.max = String(Math.max(0, samples.length - 1));
    slider.value = "0";
  }

  function sampleLabel(sample) {
    if (!sample) {
      return "No sample selected.";
    }
    var raw = sample.raw ? sample.raw.join(", ") : "unavailable";
    var lifted = sample.lifted ? sample.lifted.join(", ") : "unavailable";
    var proposed = sample.proposed ? sample.proposed.join(", ") : "none";
    return (
      "Sample identity source_row " +
      sample.source_row +
      ", t_ns " +
      sample.timestamp_ns +
      ". Raw (measured): " +
      raw +
      ". Derived lift: " +
      lifted +
      ". Proposed repair: " +
      proposed +
      "."
    );
  }

  function project3(p) {
    var x = p[0];
    var y = p[1];
    var z = p[2];
    return [x * 0.86 - z * 0.5, -y * 0.92 - x * 0.12 - z * 0.2];
  }

  function drawPhysical() {
    var ctx = physical.getContext("2d");
    var w = physical.width;
    var h = physical.height;
    ctx.clearRect(0, 0, w, h);
    ctx.strokeStyle = "#163f73";
    ctx.lineWidth = 2;
    ctx.beginPath();
    var started = false;
    samples.forEach(function (sample) {
      if (!sample.body_x) {
        return;
      }
      var p = project3(sample.body_x);
      var x = w / 2 + p[0] * 110;
      var y = h / 2 + p[1] * 110;
      if (!started) {
        ctx.moveTo(x, y);
        started = true;
      } else {
        ctx.lineTo(x, y);
      }
    });
    ctx.stroke();
    ctx.setLineDash(reduced ? [] : [3, 4]);
    ctx.strokeStyle = "#5b2d8a";
    ctx.beginPath();
    started = false;
    samples.forEach(function (sample) {
      if (!sample.proposed_body_x) {
        return;
      }
      var p = project3(sample.proposed_body_x);
      var x = w / 2 + p[0] * 110;
      var y = h / 2 + p[1] * 110;
      if (!started) {
        ctx.moveTo(x, y);
        started = true;
      } else {
        ctx.lineTo(x, y);
      }
    });
    ctx.stroke();
    ctx.setLineDash([]);
    var cur = samples[selected];
    if (cur && cur.body_x && cur.body_y && cur.body_z) {
      var origin = [w / 2, h / 2];
      function axis(vec, color, label) {
        var p = project3(vec);
        ctx.strokeStyle = color;
        ctx.beginPath();
        ctx.moveTo(origin[0], origin[1]);
        ctx.lineTo(origin[0] + p[0] * 90, origin[1] + p[1] * 90);
        ctx.stroke();
        ctx.fillStyle = "#161616";
        ctx.fillText(label, origin[0] + p[0] * 96, origin[1] + p[1] * 96);
      }
      axis(cur.body_x, "#8a1212", "+x");
      axis(cur.body_y, "#0b4d32", "+y");
      axis(cur.body_z, "#163f73", "+z");
    }
  }

  function drawStereo() {
    var ctx = stereo.getContext("2d");
    var w = stereo.width;
    var h = stereo.height;
    ctx.clearRect(0, 0, w, h);
    ctx.fillStyle = "#5a4630";
    ctx.font = "14px system-ui, sans-serif";
    ctx.fillText("Projection artefact", 12, 20);
    ctx.strokeStyle = "#5a4630";
    ctx.beginPath();
    var started = false;
    samples.forEach(function (sample, idx) {
      if (!sample.stereo) {
        return;
      }
      var p = project3(sample.stereo);
      var x = w / 2 + p[0] * 70;
      var y = h / 2 + p[1] * 70;
      if (!started) {
        ctx.moveTo(x, y);
        started = true;
      } else {
        ctx.lineTo(x, y);
      }
      if (idx === selected) {
        ctx.fillStyle = "#161616";
        ctx.fillRect(x - 3, y - 3, 6, 6);
      }
    });
    ctx.stroke();
  }

  function drawTimeline() {
    var ctx = timeline.getContext("2d");
    var w = timeline.width;
    var h = timeline.height;
    ctx.clearRect(0, 0, w, h);
    if (samples.length === 0) {
      return;
    }
    var maxA = 0.001;
    samples.forEach(function (sample) {
      if (sample.angle_rad && sample.angle_rad > maxA) {
        maxA = sample.angle_rad;
      }
    });
    ctx.strokeStyle = "#163f73";
    ctx.beginPath();
    samples.forEach(function (sample, idx) {
      var x = 20 + (idx / Math.max(samples.length - 1, 1)) * (w - 40);
      var angle = sample.angle_rad || 0;
      var y = h - 20 - (angle / maxA) * (h - 40);
      if (idx === 0) {
        ctx.moveTo(x, y);
      } else {
        ctx.lineTo(x, y);
      }
    });
    ctx.stroke();
    samples.forEach(function (sample, idx) {
      var x = 20 + (idx / Math.max(samples.length - 1, 1)) * (w - 40);
      if (sample.pinned_finding) {
        ctx.fillStyle = "#8a1212";
        ctx.fillRect(x - 4, 8, 8, 8);
      }
      if (idx === selected) {
        ctx.fillStyle = "#161616";
        ctx.fillRect(x - 2, 0, 4, h);
      }
    });
  }

  function renderLists() {
    findingsEl.textContent = "";
    (report.findings || []).forEach(function (finding, idx) {
      var li = document.createElement("li");
      li.className = "findings-item";
      li.tabIndex = 0;
      li.setAttribute("role", "button");
      text(
        li,
        finding.rule +
          " " +
          finding.reason_code +
          " rows " +
          finding.source_row_start +
          "-" +
          finding.source_row_end +
          " disposition " +
          finding.repair_disposition
      );
      li.addEventListener("click", function () {
        selectByRow(finding.source_row_start);
      });
      li.addEventListener("keydown", function (event) {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          selectByRow(finding.source_row_start);
        }
      });
      findingsEl.appendChild(li);
      if (idx === 0 && samples.length === 0) {
        selectByRow(finding.source_row_start);
      }
    });
    repairsEl.textContent = "";
    (report.repairs || []).forEach(function (repair) {
      var li = document.createElement("li");
      li.className = "repair-item";
      text(
        li,
        repair.id +
          " " +
          repair.algorithm +
          " " +
          repair.disposition +
          " affected_rows " +
          (repair.affected_rows || []).join(",")
      );
      repairsEl.appendChild(li);
    });
    if ((report.repairs || []).length === 0) {
      var empty = document.createElement("li");
      text(empty, "No proposed repairs.");
      repairsEl.appendChild(empty);
    }
  }

  function selectByRow(row) {
    var best = 0;
    var dist = Infinity;
    samples.forEach(function (sample, idx) {
      var d = Math.abs(sample.source_row - row);
      if (d < dist) {
        dist = d;
        best = idx;
      }
    });
    setSelected(best);
  }

  function setSelected(idx) {
    if (samples.length === 0) {
      text(selectionEl, sampleLabel(null));
      return;
    }
    selected = Math.max(0, Math.min(samples.length - 1, idx));
    if (slider) {
      slider.value = String(selected);
    }
    text(selectionEl, sampleLabel(samples[selected]));
    drawPhysical();
    drawStereo();
    drawTimeline();
  }

  if (slider) {
    slider.addEventListener("input", function () {
      setSelected(Number(slider.value));
    });
  }

  document.addEventListener("keydown", function (event) {
    if (event.target && event.target.tagName === "INPUT") {
      return;
    }
    if (event.key === "ArrowRight") {
      setSelected(selected + 1);
    } else if (event.key === "ArrowLeft") {
      setSelected(selected - 1);
    } else if (event.key === "Home") {
      setSelected(0);
    } else if (event.key === "End") {
      setSelected(samples.length - 1);
    }
  });

  renderLists();
  setSelected(0);
})();
