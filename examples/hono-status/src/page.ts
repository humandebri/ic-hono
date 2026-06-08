// examples/hono-status/src/page.ts renders the human status page.
// Markup stays server-rendered so the canister works without client assets.

import type { Incident } from './types'

export function renderPage(incidents: Incident[], now: string): string {
  const open = incidents.filter((incident) => incident.status !== 'resolved')
  const rows =
    incidents.length === 0
      ? '<tr><td colspan="4">No incidents recorded.</td></tr>'
      : incidents.map(renderIncidentRow).join('')

  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Edge Status</title>
  <style>${styles()}</style>
</head>
<body>
  <main>
    <header>
      <p class="eyebrow">ICP canister / Hono</p>
      <h1>Edge Status</h1>
      <p class="summary">${open.length === 0 ? 'All systems operational.' : `${open.length} active incident${open.length === 1 ? '' : 's'}.`}</p>
    </header>
    <section class="metrics" aria-label="status metrics">
      <div><span>${incidents.length}</span><small>Total incidents</small></div>
      <div><span>${open.length}</span><small>Open incidents</small></div>
      <div><span>${now}</span><small>IC time ns</small></div>
    </section>
    <section>
      <h2>Incidents</h2>
      <table>
        <thead><tr><th>Title</th><th>Severity</th><th>Status</th><th>Updated</th></tr></thead>
        <tbody>${rows}</tbody>
      </table>
    </section>
  </main>
</body>
</html>`
}

function renderIncidentRow(incident: Incident): string {
  return `<tr>
    <td>${escapeHtml(incident.title)}<small>${escapeHtml(incident.summary)}</small></td>
    <td><span class="badge severity-${incident.severity}">${incident.severity}</span></td>
    <td><span class="badge">${incident.status}</span></td>
    <td>${incident.updatedAt}</td>
  </tr>`
}

function escapeHtml(value: string): string {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
}

function styles(): string {
  return `
    :root { color-scheme: light; font-family: Inter, ui-sans-serif, system-ui, sans-serif; }
    body { margin: 0; background: #f5f7f8; color: #172026; }
    main { width: min(960px, calc(100% - 32px)); margin: 0 auto; padding: 48px 0; }
    header { margin-bottom: 28px; }
    .eyebrow { color: #47606b; font-size: 13px; text-transform: uppercase; letter-spacing: .08em; }
    h1 { margin: 0; font-size: 48px; line-height: 1; letter-spacing: 0; }
    h2 { margin: 34px 0 14px; font-size: 22px; letter-spacing: 0; }
    .summary { color: #2f4a55; font-size: 18px; }
    .metrics { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; }
    .metrics div { background: white; border: 1px solid #d7e0e4; border-radius: 8px; padding: 16px; }
    .metrics span { display: block; font-size: 24px; font-weight: 700; overflow-wrap: anywhere; }
    .metrics small, td small { display: block; color: #60727a; margin-top: 4px; }
    table { width: 100%; border-collapse: collapse; background: white; border: 1px solid #d7e0e4; }
    th, td { text-align: left; padding: 14px; border-bottom: 1px solid #e6ecef; vertical-align: top; }
    th { color: #47606b; font-size: 13px; font-weight: 700; }
    .badge { display: inline-block; border-radius: 999px; padding: 4px 9px; background: #e8eef1; font-size: 12px; font-weight: 700; }
    .severity-minor { background: #e9f5ee; color: #1f6f43; }
    .severity-major { background: #fff1d7; color: #7a4a00; }
    .severity-critical { background: #ffe3e3; color: #9b1c1c; }
    @media (max-width: 720px) {
      main { padding: 28px 0; }
      h1 { font-size: 38px; }
      .metrics { grid-template-columns: 1fr; }
      th:nth-child(4), td:nth-child(4) { display: none; }
    }
  `
}
