(() => {
  "use strict";

  if (window.top !== window || document.getElementById("fiveplay-performance-hud")) {
    return;
  }

  const HISTORY_SIZE = 54;
  const history = Array(HISTORY_SIZE).fill(0);
  let frames = 0;
  let lastSample = performance.now();
  let fps = 0;
  let longTaskMs = 0;

  const host = document.createElement("div");
  host.id = "fiveplay-performance-hud";
  const shadow = host.attachShadow({ mode: "closed" });
  shadow.innerHTML = `
    <style>
      :host { all: initial; }
      * { box-sizing: border-box; }
      .hud {
        --mint: #71e6a6;
        --panel: rgba(7, 12, 17, .94);
        position: fixed;
        z-index: 2147483647;
        top: 50%; right: 14px;
        width: 274px;
        transform: translateY(-50%);
        color: #f5f8f7;
        font: 600 12px/1.35 system-ui, sans-serif;
        filter: drop-shadow(0 18px 38px rgba(0, 0, 0, .42));
        transition: width .22s ease, transform .22s ease;
        user-select: none;
      }
      .panel {
        overflow: hidden;
        border: 1px solid rgba(113, 230, 166, .24);
        border-radius: 18px;
        background: linear-gradient(150deg, rgba(20, 34, 37, .97), var(--panel) 55%);
        backdrop-filter: blur(18px);
      }
      .head { display: flex; align-items: center; gap: 10px; padding: 13px 13px 11px; }
      .mark { display: grid; place-items: center; width: 30px; height: 30px; border-radius: 9px; background: var(--mint); color: #07140d; font: 900 16px/1 system-ui; }
      .identity { min-width: 0; flex: 1; }
      .mini { display: none; color: var(--mint); font: 850 11px/1 system-ui; font-variant-numeric: tabular-nums; }
      .eyebrow { color: var(--mint); font-size: 9px; letter-spacing: 1.5px; }
      .title { margin-top: 1px; font-size: 12px; font-weight: 850; }
      .toggle { width: 29px; height: 29px; border: 0; border-radius: 9px; color: #cbd5d2; background: rgba(255,255,255,.07); cursor: pointer; font: 800 14px/1 system-ui; }
      .toggle:hover { background: rgba(113,230,166,.16); color: var(--mint); }
      .body { padding: 0 13px 13px; opacity: 1; transition: opacity .15s ease; }
      .hero { display: grid; grid-template-columns: 1.2fr .8fr; gap: 9px; }
      .metric { padding: 11px; border: 1px solid rgba(255,255,255,.07); border-radius: 12px; background: rgba(255,255,255,.035); }
      .metric.primary { border-color: rgba(113,230,166,.2); background: rgba(113,230,166,.07); }
      .label { color: #81908b; font-size: 9px; letter-spacing: 1.2px; }
      .value { margin-top: 3px; font: 850 23px/1 system-ui; font-variant-numeric: tabular-nums; }
      .unit { color: #91a19b; font-size: 9px; margin-left: 3px; }
      .small-value { margin-top: 5px; font: 800 15px/1 system-ui; font-variant-numeric: tabular-nums; }
      canvas { display: block; width: 100%; height: 58px; margin: 10px 0; border-radius: 11px; background: rgba(0,0,0,.18); }
      .row { display: flex; align-items: center; justify-content: space-between; padding: 7px 2px; border-top: 1px solid rgba(255,255,255,.06); }
      .row span:first-child { color: #86958f; font-size: 10px; }
      .row span:last-child { font-variant-numeric: tabular-nums; }
      .state { color: var(--mint); }
      .hint { padding-top: 9px; color: #64716d; font-size: 9px; text-align: center; }
      .hud.collapsed { width: 88px; }
      .collapsed .identity, .collapsed .body { display: none; }
      .collapsed .mini { display: block; }
      .collapsed .head { padding: 9px; gap: 7px; }
      .collapsed .toggle { transform: rotate(180deg); }
      @media (max-width: 700px) { .hud { right: 8px; width: 248px; } }
    </style>
    <aside class="hud" aria-label="Performance HUD">
      <div class="panel">
        <div class="head">
          <div class="mark">V</div>
          <div class="identity"><div class="eyebrow">LIVE TELEMETRY</div><div class="title">PERFORMANCE HUD</div></div>
          <div class="mini"><span data-mini-fps>--</span> FPS</div>
          <button class="toggle" type="button" title="Collapse performance panel" aria-label="Collapse performance panel">›</button>
        </div>
        <div class="body">
          <div class="hero">
            <div class="metric primary"><div class="label">DISPLAY FPS</div><div class="value"><span data-fps>--</span><span class="unit">FPS</span></div></div>
            <div class="metric"><div class="label">FRAME</div><div class="small-value"><span data-frame>--</span><span class="unit">MS</span></div></div>
          </div>
          <canvas width="496" height="116" aria-label="Recent FPS graph"></canvas>
          <div class="row"><span>JS memory</span><span data-memory>Unavailable</span></div>
          <div class="row"><span>Long tasks</span><span data-tasks>0 ms</span></div>
          <div class="row"><span>Session state</span><span class="state" data-state>Monitoring</span></div>
          <div class="hint">ALT + P TO COLLAPSE</div>
        </div>
      </div>
    </aside>`;

  const hud = shadow.querySelector(".hud");
  const toggle = shadow.querySelector(".toggle");
  const canvas = shadow.querySelector("canvas");
  const context = canvas.getContext("2d");
  const fpsNode = shadow.querySelector("[data-fps]");
  const miniFpsNode = shadow.querySelector("[data-mini-fps]");
  const frameNode = shadow.querySelector("[data-frame]");
  const memoryNode = shadow.querySelector("[data-memory]");
  const taskNode = shadow.querySelector("[data-tasks]");
  const stateNode = shadow.querySelector("[data-state]");

  const setCollapsed = (collapsed) => {
    hud.classList.toggle("collapsed", collapsed);
    toggle.title = collapsed ? "Expand performance panel" : "Collapse performance panel";
    try { localStorage.setItem("fiveplay-hud-collapsed", collapsed ? "1" : "0"); } catch (_) {}
  };

  toggle.addEventListener("click", () => setCollapsed(!hud.classList.contains("collapsed")));
  addEventListener("keydown", (event) => {
    if (event.altKey && event.code === "KeyP") {
      event.preventDefault();
      setCollapsed(!hud.classList.contains("collapsed"));
    }
  }, true);

  try { setCollapsed(localStorage.getItem("fiveplay-hud-collapsed") === "1"); } catch (_) {}

  try {
    new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) longTaskMs += entry.duration;
    }).observe({ type: "longtask", buffered: true });
  } catch (_) {}

  const drawGraph = () => {
    context.clearRect(0, 0, canvas.width, canvas.height);
    const gradient = context.createLinearGradient(0, 0, 0, canvas.height);
    gradient.addColorStop(0, "rgba(113,230,166,.32)");
    gradient.addColorStop(1, "rgba(113,230,166,0)");
    context.beginPath();
    history.forEach((value, index) => {
      const x = index * (canvas.width / (HISTORY_SIZE - 1));
      const y = canvas.height - Math.min(value, 120) / 120 * (canvas.height - 12) - 6;
      if (index === 0) context.moveTo(x, y); else context.lineTo(x, y);
    });
    context.lineTo(canvas.width, canvas.height);
    context.lineTo(0, canvas.height);
    context.closePath();
    context.fillStyle = gradient;
    context.fill();
    context.beginPath();
    history.forEach((value, index) => {
      const x = index * (canvas.width / (HISTORY_SIZE - 1));
      const y = canvas.height - Math.min(value, 120) / 120 * (canvas.height - 12) - 6;
      if (index === 0) context.moveTo(x, y); else context.lineTo(x, y);
    });
    context.strokeStyle = "#71e6a6";
    context.lineWidth = 3;
    context.stroke();
  };

  const tick = (now) => {
    frames += 1;
    const elapsed = now - lastSample;
    if (elapsed >= 750) {
      fps = Math.round(frames * 1000 / elapsed);
      frames = 0;
      lastSample = now;
      history.push(fps);
      history.shift();
      fpsNode.textContent = String(fps);
      miniFpsNode.textContent = String(fps);
      frameNode.textContent = fps > 0 ? (1000 / fps).toFixed(1) : "--";
      taskNode.textContent = `${Math.round(longTaskMs)} ms`;
      const pressure = longTaskMs > 220 || fps < 30;
      stateNode.textContent = pressure ? "Under load" : fps < 50 ? "Tuning" : "Smooth";
      stateNode.style.color = pressure ? "#ffad72" : fps < 50 ? "#f1cf6c" : "#71e6a6";
      longTaskMs = 0;
      if (performance.memory) {
        memoryNode.textContent = `${Math.round(performance.memory.usedJSHeapSize / 1048576)} MB`;
      }
      drawGraph();
    }
    requestAnimationFrame(tick);
  };

  const mount = () => {
    (document.documentElement || document.body).appendChild(host);
    requestAnimationFrame(tick);
  };
  if (document.documentElement) mount(); else addEventListener("DOMContentLoaded", mount, { once: true });
})();
