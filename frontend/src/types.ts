export type EventRecord = {
  id: number;
  timestamp_utc: string;
  channel: string;
  record_id: number;
  event_id: number;
  provider: string;
  computer: string;
  severity: string;
  category: string;
  summary: string;
  username: string | null;
  source_ip: string | null;
  process_name: string | null;
  command_line: string | null;
  fields: Record<string, string>;
  raw_xml: string;
};

export type EventListResponse = {
  items: EventRecord[];
  total: number;
  limit: number;
  offset: number;
};

export type DashboardSummary = {
  hostname: string;
  events_total: number;
  events_24h: number;
  open_alerts: number;
  active_channels: number;
  severities: { severity: string; count: number }[];
  activity: { hour: string; count: number }[];
};

export type AlertRecord = {
  id: number;
  rule_id: string;
  fingerprint: string;
  severity: string;
  title: string;
  description: string;
  first_seen_utc: string;
  last_seen_utc: string;
  event_count: number;
  status: string;
};

export type SourceStatus = {
  channel: string;
  status: string;
  last_checked_utc: string | null;
  events_last_poll: number;
  error: string | null;
};
