/* KNXBench project evolution: growing story graph, full-complexity explorer and motion control. */
(() => {
  "use strict";

  const SVG_NS = "http://www.w3.org/2000/svg";
  const data = JSON.parse(document.getElementById("story-data").textContent);
  const root = document.documentElement;
  root.classList.add("js");

  const events = new Map(data.events.map((event) => [event.id, event]));
  const strands = new Map(data.strands.map((strand) => [strand.id, strand]));
  const chapterIndex = new Map(data.chapters.map((chapter, index) => [chapter.id, index]));
  const geometry = data.graph;
  const rootIds = new Set(data.events.filter((event) => !data.relations.some((r) => r.to === event.id)).map((e) => e.id));

  const storage = {
    get(key) { try { return window.localStorage.getItem(key); } catch (_) { return null; } },
    set(key, value) { try { window.localStorage.setItem(key, value); } catch (_) { /* private mode */ } },
  };

  function svg(name, attributes, parent) {
    const node = document.createElementNS(SVG_NS, name);
    for (const [key, value] of Object.entries(attributes || {})) node.setAttribute(key, String(value));
    if (parent) parent.appendChild(node);
    return node;
  }

  /* ---------- Motion: user switch plus live OS preference, both cancel running effects ---------- */
  const motionBox = document.getElementById("motion-off");
  const textBox = document.getElementById("text-only");
  const reducedNote = document.getElementById("reduced-note");
  const reduceQuery = window.matchMedia("(prefers-reduced-motion: reduce)");

  function motionAllowed() { return !reduceQuery.matches && !motionBox.checked; }

  function settleEffects() {
    document.querySelectorAll(".is-new").forEach((node) => node.classList.remove("is-new"));
    if (document.getAnimations) {
      for (const animation of document.getAnimations()) {
        try { animation.finish(); } catch (_) { animation.cancel(); }
      }
    }
  }

  function applyMotion() {
    const allowed = motionAllowed();
    root.classList.toggle("motion-off", !allowed);
    reducedNote.hidden = !reduceQuery.matches;
    if (!allowed) settleEffects();
  }

  motionBox.checked = storage.get("knxbench-story-motion") === "off";
  motionBox.addEventListener("change", () => {
    storage.set("knxbench-story-motion", motionBox.checked ? "off" : "on");
    applyMotion();
  });
  if (reduceQuery.addEventListener) reduceQuery.addEventListener("change", applyMotion);
  else if (reduceQuery.addListener) reduceQuery.addListener(applyMotion);

  textBox.checked = storage.get("knxbench-story-text-only") === "on";
  function applyTextOnly() {
    root.classList.toggle("text-only", textBox.checked);
    if (!textBox.checked) { story.fit(); atlas.fit(); }
  }
  textBox.addEventListener("change", () => {
    storage.set("knxbench-story-text-only", textBox.checked ? "on" : "off");
    applyTextOnly();
  });

  /* ---------- Shared graph drawing ---------- */
  function edgePath(from, to) {
    const a = events.get(from).layout;
    const b = events.get(to).layout;
    if (a.x === b.x && Math.abs(a.row - b.row) > 1) {
      // Same lane, skipping rows: bow away from the label side so the line never runs
      // through the steps in between and suggests a connection that is not there.
      const bow = a.label_side === "start" ? 34 : -34;
      return `M${a.x} ${a.y}C${a.x + bow} ${a.y + 24} ${b.x + bow} ${b.y - 24} ${b.x} ${b.y}`;
    }
    const middle = (a.y + b.y) / 2;
    return `M${a.x} ${a.y}C${a.x} ${middle} ${b.x} ${middle} ${b.x} ${b.y}`;
  }

  function drawGraph(container, interactive) {
    const graph = svg("svg", {
      class: "graph",
      viewBox: `0 0 ${geometry.width} ${geometry.height}`,
      preserveAspectRatio: "xMidYMin meet",
    }, container);
    const lanes = svg("g", { class: "lanes", "aria-hidden": "true" }, graph);
    geometry.lanes.forEach((lane, index) => {
      svg("line", { class: "lane-line", x1: lane.x, x2: lane.x, y1: geometry.top_margin - 24, y2: geometry.height - 20 }, lanes);
      // Alternate header heights so long strand names on neighbouring lanes never overlap.
      const label = svg("text", { class: "lane-label", x: lane.x, y: geometry.top_margin - (index % 2 ? 36 : 62) }, lanes);
      label.textContent = strands.get(lane.strand).label;
    });
    const dates = svg("g", { class: "dates", "aria-hidden": "true" }, graph);
    const edgeLayer = svg("g", { class: "edges", "aria-hidden": "true" }, graph);
    const nodeLayer = svg("g", { class: "nodes" }, graph);
    const dateLabels = [];
    geometry.rows.forEach((row) => {
      if (!row.shows_date || !row.date) return;
      const label = svg("text", { class: "date-label", x: geometry.left_gutter - 14, y: geometry.top_margin + row.row * geometry.row_height }, dates);
      const day = new Date(`${row.date}T12:00:00Z`);
      label.textContent = day.toLocaleDateString("en-GB", { day: "numeric", month: "short", timeZone: "UTC" });
      dateLabels.push({ label, row: row.row });
    });
    const edges = new Map();
    data.relations.forEach((relation) => {
      const attributes = { class: `edge ${relation.type}`, d: edgePath(relation.from, relation.to) };
      if (relation.type === "documented_cause") attributes.pathLength = 1;
      edges.set(relation.id, svg("path", attributes, edgeLayer));
    });
    const nodes = new Map();
    data.events.forEach((event) => {
      const strand = strands.get(event.strand);
      const layout = event.layout;
      const node = svg("g", {
        class: `node accent-${strand.accent} status-${event.status} label-${layout.label_side}${rootIds.has(event.id) ? " is-root" : ""}`,
        "data-event": event.id,
      }, nodeLayer);
      svg("circle", { cx: layout.x, cy: layout.y, r: 7 }, node);
      const text = svg("text", { x: layout.label_side === "start" ? layout.x - 14 : layout.x + 14, y: layout.y }, node);
      text.textContent = event.short;
      if (interactive) {
        node.setAttribute("tabindex", "0");
        node.setAttribute("role", "button");
        node.setAttribute("aria-label", `${event.title}. ${strand.label}. ${event.date || "Date unknown"}.`);
      } else {
        node.setAttribute("aria-hidden", "true");
      }
      nodes.set(event.id, node);
    });
    return { graph, nodes, edges, dateLabels };
  }

  // Keeps labels legible at any scale: higher-priority labels win, overlapping ones are hidden
  // (the step stays visible as a dot and in the text), and date labels thin out when rows crowd.
  function declutter(graph, scale, candidates, priority, dateVisible) {
    const kept = [];
    const ordered = Array.from(candidates).sort((a, b) =>
      (priority(b) - priority(a)) || (events.get(a).layout.row - events.get(b).layout.row));
    graph.nodes.forEach((node) => node.classList.remove("label-suppressed"));
    ordered.forEach((id) => {
      const layout = events.get(id).layout;
      const width = events.get(id).short.length * 7.6 + 18;
      const x = layout.x * scale;
      const y = layout.y * scale;
      const box = layout.label_side === "start" ? [x - width, x] : [x, x + width];
      const clash = kept.some((other) => Math.abs(other.y - y) < 14 && other.box[0] < box[1] && box[0] < other.box[1]);
      if (clash) graph.nodes.get(id).classList.add("label-suppressed");
      else kept.push({ y, box });
    });
    let lastRow = -Infinity;
    graph.dateLabels.forEach(({ label, row }) => {
      if (!dateVisible(row)) return;
      const show = (row - lastRow) * geometry.row_height * scale >= 15;
      label.classList.toggle("date-thinned", !show);
      if (show) lastRow = row;
    });
  }

  function scaleGraph(graph, scale, nodeRadius) {
    const px = 1 / Math.max(scale, 0.05);
    graph.graph.style.setProperty("--px", `${px}px`);
    graph.graph.style.setProperty("--label", `${12 * px}px`);
    graph.nodes.forEach((node) => node.querySelector("circle").setAttribute("r", String(Math.max(7, nodeRadius * px))));
  }

  /* ---------- Guided story: the tree grows chapter by chapter ---------- */
  const story = (() => {
    const stage = document.getElementById("story-graph");
    const progress = document.getElementById("graph-progress");
    const prevButton = document.getElementById("prev-chapter");
    const nextButton = document.getElementById("next-chapter");
    const replayButton = document.getElementById("replay-growth");
    const sections = Array.from(document.querySelectorAll(".chapter-section"));
    const graph = drawGraph(stage, false);
    graph.graph.setAttribute("role", "img");
    const rowEvent = new Map(data.events.map((event) => [event.layout.row, event.id]));
    let current = -1;
    let storyScale = 1;
    let activeIds = new Set();
    let visibleIds = new Set();

    function tidy() {
      declutter(graph, storyScale, activeIds, () => 0, (row) => visibleIds.has(rowEvent.get(row)));
    }

    function fit() {
      const width = stage.clientWidth;
      const height = stage.clientHeight;
      if (!width || !height) return;
      storyScale = Math.min(width / geometry.width, height / geometry.height);
      scaleGraph(graph, storyScale, 4.2);
      tidy();
    }

    function stateFor(index) {
      const visible = new Set();
      const active = new Set();
      data.events.forEach((event) => {
        const chapter = chapterIndex.get(event.chapter);
        if (chapter <= index) visible.add(event.id);
        if (chapter === index) active.add(event.id);
      });
      return { visible, active };
    }

    function show(index, animate) {
      const previous = current;
      current = index;
      const before = previous >= 0 ? stateFor(previous).visible : new Set();
      const { visible, active } = stateFor(index);
      const grow = animate && motionAllowed() && index > previous;
      let edgeOrder = 0;
      data.relations.forEach((relation) => {
        const edge = graph.edges.get(relation.id);
        const shown = visible.has(relation.from) && visible.has(relation.to);
        const wasShown = before.has(relation.from) && before.has(relation.to);
        edge.classList.toggle("is-hidden", !shown);
        edge.classList.toggle("is-past", shown && !active.has(relation.from) && !active.has(relation.to));
        edge.classList.remove("is-new");
        if (shown && grow && !wasShown) {
          edge.style.animationDelay = `${Math.min(edgeOrder, 8) * 110}ms`;
          void edge.getBoundingClientRect();
          edge.classList.add("is-new");
          edgeOrder += 1;
        }
      });
      let nodeOrder = 0;
      data.events.forEach((event) => {
        const node = graph.nodes.get(event.id);
        const shown = visible.has(event.id);
        node.classList.toggle("is-hidden", !shown);
        node.classList.toggle("is-active", active.has(event.id));
        node.classList.toggle("is-past", shown && !active.has(event.id));
        node.classList.remove("is-new");
        if (shown && grow && !before.has(event.id)) {
          node.style.animationDelay = `${300 + Math.min(nodeOrder, 10) * 90}ms`;
          void node.getBoundingClientRect();
          node.classList.add("is-new");
          nodeOrder += 1;
        }
      });
      graph.dateLabels.forEach(({ label, row }) => {
        label.classList.toggle("is-hidden", !visible.has(rowEvent.get(row)));
      });
      activeIds = active;
      visibleIds = visible;
      tidy();
      document.querySelectorAll(".event-card.is-current").forEach((card) => card.classList.remove("is-current"));
      active.forEach((id) => {
        const card = document.getElementById(`ev-${id}`);
        if (card && !card.closest(".inspector")) card.classList.add("is-current");
      });
      const chapter = data.chapters[index];
      const titles = Array.from(active, (id) => events.get(id).title);
      progress.textContent = `Chapter ${chapter.number} of ${data.chapters.length} · ${visible.size} of ${data.events.length} steps`;
      graph.graph.setAttribute("aria-label",
        `Ancestry graph, chapter ${chapter.number}: ${chapter.title}. ${visible.size} of ${data.events.length} steps shown. ` +
        `Highlighted: ${titles.join("; ")}.`);
      prevButton.disabled = index === 0;
      nextButton.disabled = index === data.chapters.length - 1;
    }

    // While a button scrolls to a chapter, the observer must not re-show the chapters the
    // scroll passes on the way; that would restart or cancel the growth that just began.
    let navigatingTo = null;
    let navigationTimer = 0;

    function goTo(index) {
      const section = sections[index];
      if (!section) return;
      navigatingTo = index;
      window.clearTimeout(navigationTimer);
      navigationTimer = window.setTimeout(() => { navigatingTo = null; }, 1500);
      section.scrollIntoView({ behavior: motionAllowed() ? "smooth" : "auto", block: "start" });
      const heading = section.querySelector("h2");
      heading.setAttribute("tabindex", "-1");
      heading.focus({ preventScroll: true });
      show(index, true);
    }

    prevButton.addEventListener("click", () => goTo(Math.max(0, current - 1)));
    nextButton.addEventListener("click", () => goTo(Math.min(data.chapters.length - 1, current + 1)));
    replayButton.addEventListener("click", () => {
      const target = current;
      if (target > 0) show(target - 1, false);
      else current = -1;
      show(target, true);
    });

    if ("IntersectionObserver" in window) {
      const observer = new IntersectionObserver((entries) => {
        entries.forEach((entry) => {
          if (!entry.isIntersecting) return;
          const index = sections.indexOf(entry.target);
          if (navigatingTo !== null) {
            if (index === navigatingTo) navigatingTo = null;
            return;
          }
          if (index !== current) show(index, true);
        });
      }, { rootMargin: window.innerWidth <= 980 ? "-58% 0px -37% 0px" : "-35% 0px -60% 0px" });
      sections.forEach((section) => observer.observe(section));
    }
    show(0, false);
    return { fit, show, get current() { return current; } };
  })();

  /* ---------- Full complexity view: pan, zoom, search, strand focus, inspector ---------- */
  const atlas = (() => {
    const canvas = document.getElementById("atlas-canvas");
    const inspector = document.getElementById("inspector");
    const searchInput = document.getElementById("atlas-search");
    const searchStatus = document.getElementById("search-status");
    const searchResults = document.getElementById("search-results");
    const chips = Array.from(document.querySelectorAll(".strand-chips .chip"));
    const graph = drawGraph(canvas, true);
    const view = { x: 0, y: 0, w: geometry.width, h: geometry.height };
    let focusStrand = "";
    let matches = null;
    let selected = null;

    function scale() { return canvas.clientWidth / view.w; }

    function clampAxis(start, size, extent) {
      const slack = 60;
      if (size >= extent + 2 * slack) return (extent - size) / 2;
      return Math.min(Math.max(start, -slack), extent - size + slack);
    }

    function tidy() {
      const priority = (id) => (id === selected ? 3 : 0) + (matches && matches.has(id) ? 2 : 0) +
        (focusStrand && events.get(id).strand === focusStrand ? 1 : 0);
      declutter(graph, scale(), graph.nodes.keys(), priority, () => true);
    }

    function apply() {
      view.x = clampAxis(view.x, view.w, geometry.width);
      view.y = clampAxis(view.y, view.h, geometry.height);
      graph.graph.setAttribute("viewBox", `${view.x} ${view.y} ${view.w} ${view.h}`);
      scaleGraph(graph, scale(), 5);
      tidy();
    }

    function fit() {
      const width = canvas.clientWidth;
      const height = canvas.clientHeight;
      if (!width || !height) return;
      const aspect = height / width;
      if (width < 700) {
        view.w = Math.min(geometry.width, width / 0.62);
        view.x = Math.max(0, geometry.lanes[2].x - view.w / 2);
      } else {
        view.w = geometry.width;
        view.x = 0;
      }
      view.h = view.w * aspect;
      view.y = 0;
      apply();
    }

    function zoom(factor, cx, cy) {
      const width = Math.min(Math.max(view.w / factor, 160), geometry.width * 3);
      const ratio = width / view.w;
      const focusX = cx === undefined ? view.x + view.w / 2 : cx;
      const focusY = cy === undefined ? view.y + view.h / 2 : cy;
      view.x = focusX - (focusX - view.x) * ratio;
      view.y = focusY - (focusY - view.y) * ratio;
      view.w = width;
      view.h = view.w * (canvas.clientHeight / canvas.clientWidth);
      apply();
    }

    function pan(dx, dy) {
      view.x += dx;
      view.y += dy;
      apply();
    }

    function toGraph(clientX, clientY) {
      const box = canvas.getBoundingClientRect();
      return { x: view.x + (clientX - box.left) / scale(), y: view.y + (clientY - box.top) / scale() };
    }

    function centreOn(id) {
      const layout = events.get(id).layout;
      const margin = view.w * 0.12;
      const inside = layout.x > view.x + margin && layout.x < view.x + view.w - margin &&
        layout.y > view.y + margin && layout.y < view.y + view.h - margin;
      if (!inside) {
        view.x = layout.x - view.w / 2;
        view.y = layout.y - view.h / 2;
        apply();
      }
    }

    function refreshEmphasis() {
      graph.nodes.forEach((node, id) => {
        const event = events.get(id);
        const inStrand = !focusStrand || event.strand === focusStrand;
        const matching = !matches || matches.has(id);
        node.classList.toggle("is-dimmed", !(inStrand && matching));
        node.classList.toggle("is-match", Boolean(matches && matches.has(id)));
      });
      data.relations.forEach((relation) => {
        const edge = graph.edges.get(relation.id);
        const touches = !focusStrand || events.get(relation.from).strand === focusStrand || events.get(relation.to).strand === focusStrand;
        const matching = !matches || matches.has(relation.from) || matches.has(relation.to);
        edge.classList.toggle("is-dimmed", !(touches && matching));
      });
      tidy();
    }

    function select(id, moveFocus) {
      selected = id;
      graph.nodes.forEach((node, key) => node.classList.toggle("is-selected", key === id));
      const original = document.getElementById(`ev-${id}`);
      inspector.replaceChildren();
      const heading = document.createElement("p");
      heading.className = "inspector-heading";
      heading.textContent = "Selected step";
      inspector.appendChild(heading);
      if (original) {
        const copy = original.cloneNode(true);
        copy.removeAttribute("id");
        copy.removeAttribute("tabindex");
        copy.classList.remove("is-current");
        const details = copy.querySelector("details");
        if (details) details.open = true;
        inspector.appendChild(copy);
      }
      const link = document.createElement("a");
      link.className = "inspector-link";
      link.href = `#ev-${id}`;
      link.textContent = "Read this step in the story";
      inspector.appendChild(link);
      centreOn(id);
      tidy();
      if (moveFocus) graph.nodes.get(id).focus({ preventScroll: true });
    }

    function search(query) {
      const terms = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
      searchResults.replaceChildren();
      if (!terms.length) {
        matches = null;
        searchStatus.textContent = "";
        refreshEmphasis();
        return;
      }
      matches = new Set();
      data.events.forEach((event) => {
        const haystack = [event.title, event.short, event.summary, event.why, event.aside || "",
          strands.get(event.strand).label, ...event.excerpts.map((x) => x.text), ...event.evidence.map((x) => x.label)]
          .join(" ").toLowerCase();
        if (terms.every((term) => haystack.includes(term))) matches.add(event.id);
      });
      searchStatus.textContent = matches.size === 1 ? "1 step matches." : `${matches.size} steps match.`;
      matches.forEach((id) => {
        const item = document.createElement("li");
        const button = document.createElement("button");
        button.type = "button";
        button.textContent = events.get(id).title;
        button.addEventListener("click", () => select(id, true));
        item.appendChild(button);
        searchResults.appendChild(item);
      });
      refreshEmphasis();
    }

    searchInput.addEventListener("input", () => search(searchInput.value));
    searchInput.addEventListener("keydown", (event) => {
      if (event.key === "Enter" && matches && matches.size) {
        event.preventDefault();
        select(matches.values().next().value, true);
      }
    });

    chips.forEach((chip) => chip.addEventListener("click", () => {
      focusStrand = chip.dataset.strand;
      chips.forEach((other) => other.setAttribute("aria-pressed", String(other === chip)));
      refreshEmphasis();
    }));

    document.getElementById("zoom-in").addEventListener("click", () => zoom(1.35));
    document.getElementById("zoom-out").addEventListener("click", () => zoom(1 / 1.35));
    document.getElementById("zoom-reset").addEventListener("click", fit);

    graph.nodes.forEach((node, id) => node.addEventListener("click", () => {
      if (!dragMoved) select(id, false);
    }));

    canvas.addEventListener("keydown", (event) => {
      const step = view.w * 0.08;
      const target = event.target.closest ? event.target.closest(".node") : null;
      if ((event.key === "Enter" || event.key === " ") && target) {
        event.preventDefault();
        select(target.dataset.event, false);
        return;
      }
      const moves = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] };
      if (moves[event.key] && !target) {
        event.preventDefault();
        pan(...moves[event.key]);
      } else if (event.key === "+" || event.key === "=") {
        event.preventDefault();
        zoom(1.25);
      } else if (event.key === "-" || event.key === "_") {
        event.preventDefault();
        zoom(1 / 1.25);
      } else if (event.key === "0") {
        event.preventDefault();
        fit();
      }
    });

    canvas.addEventListener("wheel", (event) => {
      if (!event.ctrlKey && !event.metaKey) return;
      event.preventDefault();
      const point = toGraph(event.clientX, event.clientY);
      zoom(event.deltaY < 0 ? 1.15 : 1 / 1.15, point.x, point.y);
    }, { passive: false });

    const pointers = new Map();
    let dragMoved = false;
    let pinchDistance = 0;
    canvas.addEventListener("pointerdown", (event) => {
      pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
      dragMoved = false;
      if (pointers.size === 2) {
        const [a, b] = Array.from(pointers.values());
        pinchDistance = Math.hypot(a.x - b.x, a.y - b.y);
      }
    });
    canvas.addEventListener("pointermove", (event) => {
      const last = pointers.get(event.pointerId);
      if (!last) return;
      const dx = event.clientX - last.x;
      const dy = event.clientY - last.y;
      if (!dragMoved && Math.hypot(dx, dy) < 4) return;
      if (!dragMoved) {
        dragMoved = true;
        canvas.classList.add("is-dragging");
        try { canvas.setPointerCapture(event.pointerId); } catch (_) { /* pointer already gone */ }
      }
      pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
      if (pointers.size === 2) {
        const [a, b] = Array.from(pointers.values());
        const distance = Math.hypot(a.x - b.x, a.y - b.y);
        if (pinchDistance) {
          const middle = toGraph((a.x + b.x) / 2, (a.y + b.y) / 2);
          zoom(distance / pinchDistance, middle.x, middle.y);
        }
        pinchDistance = distance;
      } else {
        pan(-dx / scale(), -dy / scale());
      }
    });
    function release(event) {
      pointers.delete(event.pointerId);
      if (pointers.size < 2) pinchDistance = 0;
      if (!pointers.size) canvas.classList.remove("is-dragging");
    }
    canvas.addEventListener("pointerup", release);
    canvas.addEventListener("pointercancel", release);

    fit();
    return { fit, select, search, get selected() { return selected; }, view };
  })();

  if ("ResizeObserver" in window) {
    new ResizeObserver(() => story.fit()).observe(document.getElementById("story-graph"));
    let lastWidth = 0;
    new ResizeObserver((entries) => {
      const width = entries[0].contentRect.width;
      if (Math.abs(width - lastWidth) > 1) { lastWidth = width; atlas.fit(); }
    }).observe(document.getElementById("atlas-canvas"));
  }
  story.fit();
  applyMotion();
  applyTextOnly();

  // Narrow test hook for automated browser checks; exposes state, never content mutation.
  window.__storyState = Object.freeze({
    chapter: () => story.current,
    view: () => ({ ...atlas.view }),
    selected: () => atlas.selected,
    motionOff: () => root.classList.contains("motion-off"),
  });
})();
