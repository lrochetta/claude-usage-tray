//! Render analytics to a self-contained HTML document.

use crate::analytics::Summary;
use crate::model::{format_reset_countdown, UsageSnapshot};

pub fn render_html(summary: &Summary) -> String {
    let last = summary.last_snapshot.as_ref();

    let (session_pct, weekly_all, reset_str, weekly_reset_str) = match last {
        Some(s) => (
            format!("{:.1}", s.session_pct),
            format!("{:.1}", s.weekly_all_pct),
            s.session_resets_at_ms
                .map(format_reset_countdown)
                .unwrap_or_else(|| "—".into()),
            s.weekly_resets_at_ms
                .map(format_reset_countdown)
                .unwrap_or_else(|| "—".into()),
        ),
        None => ("—".into(), "—".into(), "—".into(), "—".into()),
    };

    let velocity_str = summary
        .velocity
        .as_ref()
        .map(|v| format!("{:+.2} %/h", v.pct_per_hour))
        .unwrap_or_else(|| "—".into());

    let eta_str = summary
        .velocity
        .as_ref()
        .and_then(|v| v.eta_to_100_ms)
        .map(format_reset_countdown)
        .unwrap_or_else(|| "∞".into());

    let series_json = serde_json::to_string(&series_points(&summary.recent_series))
        .unwrap_or_else(|_| "[]".into());
    let heatmap_json = serde_json::to_string(&summary.heatmap).unwrap_or_else(|_| "[]".into());

    let total = summary.total_snapshots;

    format!(
        r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>claude-usage-tray — stats</title>
<meta name="viewport" content="width=device-width,initial-scale=1">
<script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.4/dist/chart.umd.min.js"></script>
<style>
  :root {{
    --bg: #0b0f17;
    --panel: #131826;
    --fg: #e5e7eb;
    --muted: #94a3b8;
    --accent: #60a5fa;
    --green: #22c55e;
    --yellow: #eab308;
    --orange: #f97316;
    --red: #dc2626;
  }}
  * {{ box-sizing: border-box; }}
  html, body {{ margin: 0; padding: 0; background: var(--bg); color: var(--fg);
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif; }}
  .container {{ max-width: 1100px; margin: 0 auto; padding: 24px; }}
  h1 {{ font-size: 22px; font-weight: 600; margin: 0 0 4px 0; }}
  .subtitle {{ color: var(--muted); font-size: 13px; margin-bottom: 24px; }}
  .kpis {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 12px; margin-bottom: 24px; }}
  .kpi {{ background: var(--panel); border-radius: 8px; padding: 16px; }}
  .kpi .label {{ color: var(--muted); font-size: 12px; text-transform: uppercase; letter-spacing: .05em; }}
  .kpi .value {{ font-size: 28px; font-weight: 600; margin-top: 4px; }}
  .kpi .sub {{ color: var(--muted); font-size: 12px; margin-top: 2px; }}
  .panel {{ background: var(--panel); border-radius: 8px; padding: 16px; margin-bottom: 16px; }}
  .panel h2 {{ font-size: 14px; text-transform: uppercase; letter-spacing: .05em; color: var(--muted); margin: 0 0 12px 0; font-weight: 500; }}
  canvas {{ max-height: 320px; }}
  .heatmap {{ display: grid; grid-template-columns: 40px repeat(24, 1fr); gap: 2px; font-size: 10px; }}
  .heatmap .dow, .heatmap .hr {{ color: var(--muted); text-align: center; padding: 2px; }}
  .heatmap .cell {{ aspect-ratio: 1; border-radius: 2px; background: #1e293b; }}
  footer {{ color: var(--muted); font-size: 11px; text-align: center; margin-top: 32px; }}
</style>
</head>
<body>
<div class="container">
  <h1>claude-usage-tray</h1>
  <div class="subtitle">Live Claude Code usage analytics · {total} snapshots stored</div>

  <div class="kpis">
    <div class="kpi">
      <div class="label">Current session</div>
      <div class="value">{session_pct}%</div>
      <div class="sub">resets in {reset_str}</div>
    </div>
    <div class="kpi">
      <div class="label">Weekly (all models)</div>
      <div class="value">{weekly_all}%</div>
      <div class="sub">resets in {weekly_reset_str}</div>
    </div>
    <div class="kpi">
      <div class="label">Velocity</div>
      <div class="value">{velocity_str}</div>
      <div class="sub">ETA to 100%: {eta_str}</div>
    </div>
  </div>

  <div class="panel">
    <h2>Session % over time</h2>
    <canvas id="seriesChart"></canvas>
  </div>

  <div class="panel">
    <h2>Hourly heatmap (last 30 days, average session %)</h2>
    <div id="heatmap" class="heatmap"></div>
  </div>

  <footer>claude-usage-tray · <a href="https://github.com/lrochetta/claude-usage-tray" style="color:var(--accent)">github</a></footer>
</div>

<script>
const series = {series_json};
const heatmap = {heatmap_json};

// Line chart
if (series.length > 0) {{
  const ctx = document.getElementById('seriesChart').getContext('2d');
  new Chart(ctx, {{
    type: 'line',
    data: {{
      datasets: [
        {{
          label: 'Session %',
          data: series.map(p => ({{ x: p.t, y: p.session }})),
          borderColor: '#60a5fa',
          backgroundColor: 'rgba(96,165,250,0.15)',
          tension: 0.25,
          fill: true,
        }},
        {{
          label: 'Weekly %',
          data: series.map(p => ({{ x: p.t, y: p.weekly }})),
          borderColor: '#a78bfa',
          borderDash: [4, 4],
          tension: 0.25,
          fill: false,
        }},
      ],
    }},
    options: {{
      responsive: true,
      interaction: {{ intersect: false, mode: 'index' }},
      scales: {{
        x: {{ type: 'time', time: {{ unit: 'hour' }}, ticks: {{ color: '#94a3b8' }}, grid: {{ color: '#1e293b' }} }},
        y: {{ min: 0, max: 100, ticks: {{ color: '#94a3b8' }}, grid: {{ color: '#1e293b' }} }},
      }},
      plugins: {{ legend: {{ labels: {{ color: '#e5e7eb' }} }} }},
    }},
  }});
}}

// Heatmap
const heatEl = document.getElementById('heatmap');
const DOW_NAMES = ['S','M','T','W','T','F','S'];
const grid = {{}};
heatmap.forEach(c => {{ grid[`${{c.day_of_week}}-${{c.hour}}`] = c.avg_session_pct; }});
heatEl.innerHTML = '';
// header row
heatEl.insertAdjacentHTML('beforeend', '<div class="hr"></div>' + Array.from({{length:24}}, (_,h)=>`<div class="hr">${{h}}</div>`).join(''));
for (let d = 0; d < 7; d++) {{
  heatEl.insertAdjacentHTML('beforeend', `<div class="dow">${{DOW_NAMES[d]}}</div>`);
  for (let h = 0; h < 24; h++) {{
    const v = grid[`${{d}}-${{h}}`] ?? 0;
    const intensity = Math.min(1, v / 100);
    const bg = v > 0 ? `rgba(220, 38, 38, ${{0.15 + intensity*0.7}})` : '#1e293b';
    heatEl.insertAdjacentHTML('beforeend', `<div class="cell" style="background:${{bg}}" title="${{DOW_NAMES[d]}} ${{h}}h — ${{v.toFixed(1)}}%"></div>`);
  }}
}}
</script>
</body>
</html>"##
    )
}

#[derive(serde::Serialize)]
struct Point {
    t: i64,
    session: f32,
    weekly: f32,
}

fn series_points(snaps: &[UsageSnapshot]) -> Vec<Point> {
    snaps
        .iter()
        .map(|s| Point {
            t: s.timestamp_ms,
            session: s.session_pct,
            weekly: s.weekly_all_pct,
        })
        .collect()
}
