import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  Activity,
  AlertTriangle,
  ArrowDownRight,
  ArrowUpRight,
  Bell,
  Check,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  CircleHelp,
  Clock3,
  Database,
  FileSearch,
  Filter,
  Gauge,
  LayoutDashboard,
  LockKeyhole,
  RefreshCw,
  Search,
  Server,
  Shield,
  ShieldAlert,
  ShieldCheck,
  TerminalSquare,
  X,
} from 'lucide-react';
import { api } from './api';
import type { AlertRecord, DashboardSummary, EventRecord, EventListResponse, SourceStatus } from './types';

type View = 'overview' | 'events' | 'alerts' | 'sources';

const EMPTY_SUMMARY: DashboardSummary = {
  hostname: '—', events_total: 0, events_24h: 0, open_alerts: 0, active_channels: 0,
  severities: [], activity: [],
};

function App() {
  const [view, setView] = useState<View>('overview');
  const [summary, setSummary] = useState(EMPTY_SUMMARY);
  const [eventsResponse, setEventsResponse] = useState<EventListResponse>({ items: [], total: 0, limit: 100, offset: 0 });
  const [alerts, setAlerts] = useState<AlertRecord[]>([]);
  const [sources, setSources] = useState<SourceStatus[]>([]);
  const [query, setQuery] = useState('');
  const [channel, setChannel] = useState('');
  const [severity, setSeverity] = useState('');
  const [offset, setOffset] = useState(0);
  const [selectedEvent, setSelectedEvent] = useState<EventRecord | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);

  const load = useCallback(async (showSpinner = false) => {
    if (showSpinner) setRefreshing(true);
    try {
      const [nextSummary, nextSources, nextAlerts] = await Promise.all([
        api.summary(), api.sources(), api.alerts(),
      ]);
      setSummary(nextSummary);
      setSources(nextSources);
      setAlerts(nextAlerts);
      setError(null);
      setLastUpdated(new Date());
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'Could not reach the local SentinelLite API.');
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  }, []);

  const loadEvents = useCallback(async () => {
    try {
      const params = new URLSearchParams({ limit: '100', offset: String(offset) });
      if (query.trim()) params.set('q', query.trim());
      if (channel) params.set('channel', channel);
      if (severity) params.set('severity', severity);
      setEventsResponse(await api.events(params));
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'Could not load events.');
    }
  }, [query, channel, severity, offset]);

  useEffect(() => {
    void load();
    void loadEvents();
    const timer = window.setInterval(() => {
      void load();
      void loadEvents();
    }, 5000);
    return () => window.clearInterval(timer);
  }, [load, loadEvents]);

  const activeSources = sources.filter((source) => source.status === 'active').length;
  const openAlerts = alerts.filter((alert) => alert.status === 'open');

  const changeView = (nextView: View) => {
    setView(nextView);
    setSelectedEvent(null);
  };

  const closeAlert = async (id: number) => {
    try {
      await api.closeAlert(id);
      await load(true);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'Could not close alert.');
    }
  };

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand-lockup">
          <div className="brand-mark"><Shield size={21} strokeWidth={2.3} /></div>
          <div><div className="brand-name">Sentinel<span>Lite</span></div><div className="brand-caption">WINDOWS SECURITY MONITOR</div></div>
        </div>

        <div className="sidebar-label">WORKSPACE</div>
        <nav className="main-nav" aria-label="Main navigation">
          <NavButton active={view === 'overview'} icon={<LayoutDashboard size={17} />} label="Overview" onClick={() => changeView('overview')} />
          <NavButton active={view === 'events'} icon={<FileSearch size={17} />} label="Event explorer" count={summary.events_24h} onClick={() => changeView('events')} />
          <NavButton active={view === 'alerts'} icon={<ShieldAlert size={17} />} label="Detections" count={openAlerts.length} onClick={() => changeView('alerts')} />
          <NavButton active={view === 'sources'} icon={<Server size={17} />} label="Log sources" onClick={() => changeView('sources')} />
        </nav>

        <div className="sidebar-spacer" />
        <div className="local-card">
          <div className="local-card-top"><span className="pulse-dot" /> LOCAL COLLECTION</div>
          <div className="local-host">{summary.hostname}</div>
          <div className="local-sub">{activeSources} of {sources.length || 4} channels active</div>
          <div className="local-progress"><span style={{ width: `${sources.length ? (activeSources / sources.length) * 100 : 0}%` }} /></div>
          <div className="local-foot"><LockKeyhole size={12} /> Loopback only</div>
        </div>
        <div className="sidebar-footer"><span className="version-dot" /> SentinelLite MVP <span className="version">v0.1.0</span></div>
      </aside>

      <main className="main-area">
        <header className="topbar">
          <div className="breadcrumb"><span>Workspace</span><ChevronRight size={14} /><strong>{viewTitle(view)}</strong></div>
          <div className="topbar-actions">
            <div className="live-indicator"><span className="pulse-dot" /> COLLECTOR {sources.some((source) => source.status === 'active') ? 'RUNNING' : 'STARTING'}</div>
            <span className="topbar-divider" />
            <button className={`icon-button ${refreshing ? 'spinning' : ''}`} aria-label="Refresh data" title="Refresh data" onClick={() => { void load(true); void loadEvents(); }}><RefreshCw size={16} /></button>
            <div className="avatar">{summary.hostname.slice(0, 1).toUpperCase() || 'W'}</div>
          </div>
        </header>

        <div className="page-content">
          {error && <div className="error-banner"><AlertTriangle size={16} /><span>{error}</span><button onClick={() => setError(null)} aria-label="Dismiss"><X size={15} /></button></div>}
          {view === 'overview' && <Overview summary={summary} alerts={openAlerts} sources={sources} onOpenAlerts={() => changeView('alerts')} onOpenEvents={() => changeView('events')} onOpenSources={() => changeView('sources')} />}
          {view === 'events' && <EventsPage response={eventsResponse} query={query} channel={channel} severity={severity} offset={offset} selected={selectedEvent} onQuery={(value) => { setQuery(value); setOffset(0); }} onChannel={(value) => { setChannel(value); setOffset(0); }} onSeverity={(value) => { setSeverity(value); setOffset(0); }} onOffset={setOffset} onSelect={setSelectedEvent} />}
          {view === 'alerts' && <AlertsPage alerts={alerts} onClose={closeAlert} />}
          {view === 'sources' && <SourcesPage sources={sources} />}
          <footer className="page-footer"><span>SentinelLite is a local-first MVP. Events stay on this machine.</span><span>{lastUpdated ? `Updated ${lastUpdated.toLocaleTimeString()}` : loading ? 'Connecting…' : 'Waiting for first poll'}</span></footer>
        </div>
      </main>
    </div>
  );
}

function NavButton({ active, icon, label, count, onClick }: { active: boolean; icon: React.ReactNode; label: string; count?: number; onClick: () => void }) {
  return <button className={`nav-button ${active ? 'active' : ''}`} onClick={onClick}>{icon}<span>{label}</span>{typeof count === 'number' && count > 0 && <em>{formatCount(count)}</em>}</button>;
}

function Overview({ summary, alerts, sources, onOpenAlerts, onOpenEvents, onOpenSources }: { summary: DashboardSummary; alerts: AlertRecord[]; sources: SourceStatus[]; onOpenAlerts: () => void; onOpenEvents: () => void; onOpenSources: () => void }) {
  const maxActivity = Math.max(1, ...summary.activity.map((point) => point.count));
  const severityCounts = useMemo(() => {
    const map = new Map(summary.severities.map((item) => [item.severity, item.count]));
    return [
      { label: 'Critical', key: 'critical', color: 'critical', count: map.get('critical') ?? 0 },
      { label: 'Error', key: 'error', color: 'error', count: map.get('error') ?? 0 },
      { label: 'Warning', key: 'warning', color: 'warning', count: map.get('warning') ?? 0 },
      { label: 'Info', key: 'info', color: 'info', count: map.get('info') ?? 0 },
    ];
  }, [summary.severities]);

  return <>
    <div className="page-heading"><div><div className="eyebrow">SECURITY OPERATIONS</div><h1>Overview</h1><p>Your Windows event activity, collected locally and continuously.</p></div><button className="button button-secondary" onClick={onOpenEvents}><FileSearch size={15} /> Explore events</button></div>

    <section className="metrics-grid">
      <MetricCard title="Events collected" value={formatCount(summary.events_total)} note={`${formatCount(summary.events_24h)} in the last 24 hours`} icon={<Activity size={18} />} tone="blue" trend="live" />
      <MetricCard title="Open detections" value={formatCount(summary.open_alerts)} note="From starter detection rules" icon={<ShieldAlert size={18} />} tone="amber" trend={summary.open_alerts > 0 ? 'attention' : 'clear'} />
      <MetricCard title="Channels reporting" value={`${summary.active_channels} / ${Math.max(sources.length, 4)}`} note="Channels with events in the last 24h" icon={<Database size={18} />} tone="violet" trend="source" />
      <MetricCard title="Host monitored" value={summary.hostname || '—'} note="This local Windows machine" icon={<Server size={18} />} tone="green" trend="local" compact />
    </section>

    <section className="overview-grid">
      <div className="panel activity-panel">
        <div className="panel-heading"><div><h2>Event activity</h2><p>Events received over the last 24 hours</p></div><span className="panel-tag"><span className="pulse-dot" /> LIVE</span></div>
        <div className="activity-chart">
          {summary.activity.map((point, index) => <div className="chart-column" key={`${point.hour}-${index}`} title={`${point.hour}: ${point.count} events`}><div className="chart-track"><div className="chart-bar" style={{ height: `${Math.max(point.count > 0 ? 5 : 1, (point.count / maxActivity) * 100)}%` }} /></div><span>{index % 4 === 0 ? point.hour : ''}</span></div>)}
        </div>
        <div className="chart-legend"><span><i className="legend-dot" /> Incoming Windows events</span><span>{formatCount(summary.events_24h)} total</span></div>
      </div>

      <div className="panel severity-panel">
        <div className="panel-heading"><div><h2>Event severity</h2><p>Last 24 hours</p></div><span className="subtle-icon"><Gauge size={17} /></span></div>
        <div className="severity-list">
          {severityCounts.map((item) => <div className="severity-row" key={item.key}><div className="severity-name"><span className={`severity-dot ${item.color}`} />{item.label}</div><strong>{formatCount(item.count)}</strong><div className="severity-track"><span className={item.color} style={{ width: `${summary.events_24h ? Math.max(item.count ? 2 : 0, (item.count / summary.events_24h) * 100) : 0}%` }} /></div></div>)}
        </div>
        <div className="severity-foot"><span>Informational events are expected on healthy systems.</span></div>
      </div>
    </section>

    <section className="lower-grid">
      <div className="panel detections-panel">
        <div className="panel-heading"><div><h2>Recent detections</h2><p>Signals raised by the built-in starter rules</p></div><button className="text-button" onClick={onOpenAlerts}>View all <ChevronRight size={14} /></button></div>
        {alerts.length === 0 ? <EmptyState icon={<ShieldCheck size={22} />} title="No open detections" text="The starter rules have not raised any open alerts. Collection is still running." /> : <div className="compact-alert-list">{alerts.slice(0, 4).map((alert) => <div className="compact-alert" key={alert.id}><span className={`alert-severity ${alert.severity}`}><AlertTriangle size={14} /></span><div className="compact-alert-main"><strong>{alert.title}</strong><span>{alert.description}</span></div><time>{timeAgo(alert.last_seen_utc)}</time></div>)}</div>}
      </div>

      <div className="panel sources-panel">
        <div className="panel-heading"><div><h2>Log sources</h2><p>Local Windows channels</p></div><button className="text-button" onClick={onOpenSources}>Details <ChevronRight size={14} /></button></div>
        <div className="source-list-mini">{sources.slice(0, 4).map((source) => <div className="source-mini" key={source.channel}><span className={`source-status-dot ${source.status}`} /><div><strong>{source.channel.split('/').slice(-1)[0]}</strong><small>{source.error ? friendlyError(source.error) : source.status === 'active' ? `${source.events_last_poll} new events on last poll` : 'Waiting for collector'}</small></div><span className={`status-label ${source.status}`}>{source.status}</span></div>)}</div>
      </div>
    </section>
  </>;
}

function MetricCard({ title, value, note, icon, tone, trend, compact }: { title: string; value: string; note: string; icon: React.ReactNode; tone: string; trend: string; compact?: boolean }) {
  return <div className="metric-card"><div className="metric-top"><span>{title}</span><span className={`metric-icon ${tone}`}>{icon}</span></div><div className={`metric-value ${compact ? 'compact-value' : ''}`}>{value}</div><div className="metric-note"><span className={`metric-indicator ${trend}`}>{trend === 'attention' ? <ArrowUpRight size={13} /> : trend === 'clear' ? <Check size={13} /> : trend === 'live' ? <Activity size={13} /> : <ArrowDownRight size={13} />}</span>{note}</div></div>;
}

function EventsPage({ response, query, channel, severity, offset, selected, onQuery, onChannel, onSeverity, onOffset, onSelect }: { response: EventListResponse; query: string; channel: string; severity: string; offset: number; selected: EventRecord | null; onQuery: (value: string) => void; onChannel: (value: string) => void; onSeverity: (value: string) => void; onOffset: (value: number) => void; onSelect: (event: EventRecord | null) => void }) {
  const pageCount = Math.max(1, Math.ceil(response.total / response.limit));
  const currentPage = Math.floor(offset / response.limit) + 1;
  return <>
    <div className="page-heading"><div><div className="eyebrow">INVESTIGATION</div><h1>Event explorer</h1><p>Search and inspect normalized Windows events alongside their original XML.</p></div><div className="heading-meta"><span className="count-pill"><Activity size={14} /> {formatCount(response.total)} events</span></div></div>
    <div className="panel events-panel">
      <div className="event-toolbar">
        <div className="search-wrap"><Search size={16} /><input value={query} onChange={(event) => onQuery(event.target.value)} placeholder="Search event, user, IP, process…" aria-label="Search events" /><kbd>⌕</kbd></div>
        <div className="filter-wrap"><Filter size={15} /><select value={channel} onChange={(event) => onChannel(event.target.value)} aria-label="Filter by channel"><option value="">All channels</option>{['Security', 'System', 'Application', 'Microsoft-Windows-Sysmon/Operational'].map((name) => <option key={name} value={name}>{name}</option>)}</select><ChevronDown size={13} /></div>
        <div className="filter-wrap"><select value={severity} onChange={(event) => onSeverity(event.target.value)} aria-label="Filter by severity"><option value="">All severities</option>{['critical', 'error', 'warning', 'info', 'verbose', 'unknown'].map((name) => <option key={name} value={name}>{capitalize(name)}</option>)}</select><ChevronDown size={13} /></div>
      </div>
      <div className="table-scroll"><table className="events-table"><thead><tr><th>TIME</th><th>EVENT</th><th>CHANNEL</th><th>SEVERITY</th><th>USER / SOURCE</th><th>HOST</th><th></th></tr></thead><tbody>
        {response.items.map((event) => <tr key={event.id} className={selected?.id === event.id ? 'selected-row' : ''} onClick={() => onSelect(event)} tabIndex={0} onKeyDown={(keyEvent) => { if (keyEvent.key === 'Enter') onSelect(event); }}><td><span className="event-time">{formatTime(event.timestamp_utc)}</span><small>{formatDate(event.timestamp_utc)}</small></td><td><div className="event-title">{event.summary}</div><div className="event-sub">ID {event.event_id} <span>·</span> {event.provider}</div></td><td><span className="channel-pill">{event.channel.split('/').slice(-1)[0]}</span></td><td><SeverityBadge severity={event.severity} /></td><td><div className="event-user">{event.username || '—'}</div><div className="event-sub">{event.source_ip || event.process_name || ' '}</div></td><td className="host-cell">{event.computer}</td><td><ChevronRight size={15} className="row-chevron" /></td></tr>)}
      </tbody></table>{response.items.length === 0 && <EmptyState icon={<FileSearch size={22} />} title="No events found" text="Try a broader search or wait for the next collector poll." />}</div>
      <div className="table-footer"><span>Showing {response.items.length ? offset + 1 : 0}–{Math.min(offset + response.items.length, response.total)} of {formatCount(response.total)}</span><div className="pagination"><button aria-label="Previous page" disabled={currentPage <= 1} onClick={() => onOffset(Math.max(0, offset - response.limit))}><ChevronLeft size={15} /></button><span>Page {currentPage} of {pageCount}</span><button aria-label="Next page" disabled={currentPage >= pageCount} onClick={() => onOffset(offset + response.limit)}><ChevronRight size={15} /></button></div></div>
    </div>
    {selected && <div className="detail-overlay" onClick={() => onSelect(null)}><aside className="detail-drawer" onClick={(event) => event.stopPropagation()}><div className="drawer-head"><div><div className="eyebrow">EVENT DETAIL · #{selected.record_id}</div><h2>{selected.summary}</h2></div><button className="icon-button" onClick={() => onSelect(null)} aria-label="Close event detail"><X size={17} /></button></div><div className="drawer-tags"><SeverityBadge severity={selected.severity} /><span className="channel-pill">{selected.channel}</span></div><div className="detail-fields"><DetailField label="Timestamp" value={new Date(selected.timestamp_utc).toLocaleString()} /><DetailField label="Event ID" value={String(selected.event_id)} /><DetailField label="Provider" value={selected.provider} /><DetailField label="Computer" value={selected.computer} /><DetailField label="User" value={selected.username || '—'} /><DetailField label="Source IP" value={selected.source_ip || '—'} /><DetailField label="Process" value={selected.process_name || '—'} /><DetailField label="Category" value={selected.category} /></div><div className="drawer-section"><h3>Extracted fields</h3><pre className="json-block">{JSON.stringify(selected.fields, null, 2)}</pre></div><div className="drawer-section"><h3>Raw Windows event XML</h3><pre className="xml-block">{selected.raw_xml}</pre></div></aside></div>}
  </>;
}

function AlertsPage({ alerts, onClose }: { alerts: AlertRecord[]; onClose: (id: number) => void }) {
  const [filter, setFilter] = useState('open');
  const visible = alerts.filter((alert) => filter === 'all' || alert.status === filter);
  return <>
    <div className="page-heading"><div><div className="eyebrow">DETECTION ENGINE</div><h1>Detections</h1><p>Starter rules flag a small set of potentially interesting Windows activities.</p></div><span className="count-pill"><Bell size={14} /> {alerts.filter((alert) => alert.status === 'open').length} open</span></div>
    <div className="callout-note"><CircleHelp size={16} /><span><strong>Starter rules, not a complete SOC detection pack.</strong> Tune thresholds and validate alerts against your environment before relying on them for incident response.</span></div>
    <div className="panel alerts-panel"><div className="panel-heading panel-heading-padded"><div><h2>Alert queue</h2><p>Generated from normalized local events</p></div><div className="segmented"><button className={filter === 'open' ? 'active' : ''} onClick={() => setFilter('open')}>Open</button><button className={filter === 'closed' ? 'active' : ''} onClick={() => setFilter('closed')}>Closed</button><button className={filter === 'all' ? 'active' : ''} onClick={() => setFilter('all')}>All</button></div></div>
      {visible.length === 0 ? <EmptyState icon={<ShieldCheck size={23} />} title={filter === 'open' ? 'No open detections' : 'Nothing in this view'} text="Alerts will appear here when a starter detection rule matches collected Windows events." /> : <div className="alert-table">{visible.map((alert) => <div className="alert-row" key={alert.id}><div className={`alert-icon ${alert.severity}`}><AlertTriangle size={17} /></div><div className="alert-content"><div className="alert-row-title"><strong>{alert.title}</strong><SeverityBadge severity={alert.severity} />{alert.status === 'closed' && <span className="closed-pill">CLOSED</span>}</div><p>{alert.description}</p><div className="alert-meta"><span>{alert.rule_id}</span><span>·</span><span>{alert.event_count} matching event{alert.event_count === 1 ? '' : 's'}</span><span>·</span><span>Last seen {timeAgo(alert.last_seen_utc)}</span></div></div>{alert.status === 'open' && <button className="button button-secondary small-button" onClick={() => onClose(alert.id)}><Check size={14} /> Close</button>}</div>)}</div>}
    </div>
  </>;
}

function SourcesPage({ sources }: { sources: SourceStatus[] }) {
  return <>
    <div className="page-heading"><div><div className="eyebrow">COLLECTION</div><h1>Log sources</h1><p>Windows Event Log channels polled by the local collector every five seconds.</p></div><span className="count-pill"><Database size={14} /> {sources.filter((source) => source.status === 'active').length} active</span></div>
    <div className="callout-note"><LockKeyhole size={16} /><span>The collector is read-only and listens on this machine only. Access to the Security channel may require running the terminal as Administrator.</span></div>
    <div className="source-cards">{sources.map((source) => <div className="panel source-card" key={source.channel}><div className="source-card-head"><div className="source-icon"><TerminalSquare size={19} /></div><span className={`status-label ${source.status}`}>{source.status}</span></div><h2>{source.channel}</h2><p>{source.channel.includes('Sysmon') ? 'Detailed process, network and file telemetry from Sysmon, if installed and configured.' : source.channel === 'Security' ? 'Authentication, account management and security audit events.' : source.channel === 'System' ? 'Windows service, driver and operating system events.' : 'Application events emitted by installed applications and services.'}</p><div className="source-card-meta"><span><Clock3 size={13} /> {source.last_checked_utc ? `Checked ${timeAgo(source.last_checked_utc)}` : 'Not checked yet'}</span><span><Activity size={13} /> {source.events_last_poll} new this poll</span></div>{source.error && <div className="source-error"><AlertTriangle size={14} /> {friendlyError(source.error)}</div>}</div>)}</div>
  </>;
}

function SeverityBadge({ severity }: { severity: string }) {
  return <span className={`severity-badge ${severity}`}><span />{capitalize(severity)}</span>;
}

function EmptyState({ icon, title, text }: { icon: React.ReactNode; title: string; text: string }) {
  return <div className="empty-state"><div className="empty-icon">{icon}</div><strong>{title}</strong><p>{text}</p></div>;
}

function DetailField({ label, value }: { label: string; value: string }) {
  return <div className="detail-field"><span>{label}</span><strong>{value}</strong></div>;
}

function viewTitle(view: View) {
  return ({ overview: 'Overview', events: 'Event explorer', alerts: 'Detections', sources: 'Log sources' })[view];
}
function capitalize(value: string) { return value ? value[0].toUpperCase() + value.slice(1) : value; }
function formatCount(value: number) { return new Intl.NumberFormat().format(value); }
function formatTime(value: string) { return new Date(value).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' }); }
function formatDate(value: string) { return new Date(value).toLocaleDateString([], { day: '2-digit', month: 'short' }); }
function timeAgo(value: string) {
  const seconds = Math.max(0, Math.floor((Date.now() - new Date(value).getTime()) / 1000));
  if (seconds < 60) return `${seconds}s ago`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  return `${Math.floor(seconds / 86400)}d ago`;
}
function friendlyError(error: string) {
  if (error.toLowerCase().includes('access is denied') || error.toLowerCase().includes('error 5')) return 'Permission denied — try running as Administrator';
  if (error.toLowerCase().includes('the system cannot find the file') || error.toLowerCase().includes('error 15007')) return 'Channel not installed or unavailable';
  return error;
}

export default App;
