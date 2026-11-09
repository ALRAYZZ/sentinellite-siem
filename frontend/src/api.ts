import type {
  AlertRecord,
  DashboardSummary,
  EventListResponse,
  SourceStatus,
} from './types';

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    ...init,
    headers: { 'Content-Type': 'application/json', ...init?.headers },
  });
  if (!response.ok) {
    const detail = await response.text();
    throw new Error(detail || `Request failed (${response.status})`);
  }
  if (response.status === 204) return undefined as T;
  return response.json() as Promise<T>;
}

export const api = {
  summary: () => request<DashboardSummary>('/api/summary'),
  events: (params: URLSearchParams) => request<EventListResponse>(`/api/events?${params}`),
  alerts: () => request<AlertRecord[]>('/api/alerts'),
  sources: () => request<SourceStatus[]>('/api/sources'),
  closeAlert: (id: number) => request<void>(`/api/alerts/${id}/close`, { method: 'POST' }),
};
