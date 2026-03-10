import { invoke } from "@tauri-apps/api/core";

export type AuthStatus = {
  has_admin: boolean;
  logged_in_user: string | null;
};

export async function dbInit(): Promise<{ db_path: string }> {
  return invoke("db_init");
}

export async function authStatus(): Promise<AuthStatus> {
  return invoke("auth_status");
}

export async function authBootstrapCreateAdmin(username: string, password: string): Promise<void> {
  return invoke("auth_bootstrap_create_admin", { username, password });
}

export async function authLogin(username: string, password: string): Promise<void> {
  return invoke("auth_login", { username, password });
}

export async function authLogout(): Promise<void> {
  return invoke("auth_logout");
}

export type CsvPreview = {
  headers: string[];
  rows: string[][];
};

export type ImportPropertyResult = {
  property_name: string;
  property_id: number;
  import_id: number | null;
  blocked_duplicate: boolean;
  blocked_message: string | null;
  rows_imported: number;
  rows_rejected: number;
};

export type ImportBookingsResult = {
  file_hash: string;
  properties: ImportPropertyResult[];
};

export async function previewBookingsCsv(filePath: string, maxRows = 50): Promise<CsvPreview> {
  return invoke("preview_bookings_csv", { filePath, maxRows });
}

export async function importBookingsCsv(filePath: string): Promise<ImportBookingsResult> {
  return invoke("import_bookings_csv", { filePath });
}

export type Property = {
  id: number;
  name: string;
};

export type OverviewMetrics = {
  bookings_total: number;
  bookings_canceled: number;
  cancellation_rate: number;
  room_nights: number;
  avg_los: number;
  adr_avg: number | null;
  est_revenue: number | null;
  min_arrival_date: string | null;
  max_arrival_date: string | null;
};

export type ArrivalsByMonthRow = {
  month: string;
  arrivals: number;
};

export async function listProperties(): Promise<Property[]> {
  return invoke("list_properties");
}

export async function getOverviewMetrics(args: {
  propertyIds: number[];
  startDate?: string;
  endDate?: string;
}): Promise<OverviewMetrics> {
  const payload: Record<string, unknown> = { propertyIds: args.propertyIds };
  if (args.startDate) payload.startDate = args.startDate;
  if (args.endDate) payload.endDate = args.endDate;
  return invoke("overview_metrics", payload);
}

export async function getArrivalsByMonth(args: {
  propertyIds: number[];
  startDate?: string;
  endDate?: string;
}): Promise<ArrivalsByMonthRow[]> {
  const payload: Record<string, unknown> = { propertyIds: args.propertyIds };
  if (args.startDate) payload.startDate = args.startDate;
  if (args.endDate) payload.endDate = args.endDate;
  return invoke("arrivals_by_month", payload);
}

export type KpiCard = {
  key: string;
  label: string;
  value: number | null;
  previous_value: number | null;
  delta: number | null;
  delta_pct: number | null;
};

export type KpiDashboard = {
  start_date: string | null;
  end_date: string | null;
  previous_start_date: string | null;
  previous_end_date: string | null;
  cards: KpiCard[];
};

export async function getKpiDashboard(args: {
  propertyIds: number[];
  startDate?: string;
  endDate?: string;
  comparePreviousPeriod?: boolean;
}): Promise<KpiDashboard> {
  const payload: Record<string, unknown> = {
    propertyIds: args.propertyIds,
  };
  if (args.startDate) payload.startDate = args.startDate;
  if (args.endDate) payload.endDate = args.endDate;
  if (typeof args.comparePreviousPeriod === "boolean") {
    payload.comparePreviousPeriod = args.comparePreviousPeriod;
  }
  return invoke("kpi_dashboard", payload);
}

export type CancellationByMonthRow = {
  month: string;
  bookings_total: number;
  bookings_canceled: number;
  cancellation_rate: number;
};

export type CategoricalBreakdownRow = {
  key: string;
  count: number;
  share: number;
};

export async function getCancellationsByMonth(args: {
  propertyIds: number[];
  startDate?: string;
  endDate?: string;
}): Promise<CancellationByMonthRow[]> {
  const payload: Record<string, unknown> = { propertyIds: args.propertyIds };
  if (args.startDate) payload.startDate = args.startDate;
  if (args.endDate) payload.endDate = args.endDate;
  return invoke("cancellations_by_month", payload);
}

export async function getMarketSegmentMix(args: {
  propertyIds: number[];
  startDate?: string;
  endDate?: string;
  limit?: number;
}): Promise<CategoricalBreakdownRow[]> {
  const payload: Record<string, unknown> = { propertyIds: args.propertyIds };
  if (args.startDate) payload.startDate = args.startDate;
  if (args.endDate) payload.endDate = args.endDate;
  if (typeof args.limit === "number") payload.limit = args.limit;
  return invoke("market_segment_mix", payload);
}

export async function getDistributionChannelMix(args: {
  propertyIds: number[];
  startDate?: string;
  endDate?: string;
  limit?: number;
}): Promise<CategoricalBreakdownRow[]> {
  const payload: Record<string, unknown> = { propertyIds: args.propertyIds };
  if (args.startDate) payload.startDate = args.startDate;
  if (args.endDate) payload.endDate = args.endDate;
  if (typeof args.limit === "number") payload.limit = args.limit;
  return invoke("distribution_channel_mix", payload);
}

export async function getCountryMix(args: {
  propertyIds: number[];
  startDate?: string;
  endDate?: string;
  limit?: number;
}): Promise<CategoricalBreakdownRow[]> {
  const payload: Record<string, unknown> = { propertyIds: args.propertyIds };
  if (args.startDate) payload.startDate = args.startDate;
  if (args.endDate) payload.endDate = args.endDate;
  if (typeof args.limit === "number") payload.limit = args.limit;
  return invoke("country_mix", payload);
}
