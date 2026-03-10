import { useEffect, useMemo, useState } from "preact/hooks";
import {
  getArrivalsByMonth,
  getCancellationsByMonth,
  getCountryMix,
  getDistributionChannelMix,
  getKpiDashboard,
  getMarketSegmentMix,
  getOverviewMetrics,
  listProperties,
  type ArrivalsByMonthRow,
  type CancellationByMonthRow,
  type CategoricalBreakdownRow,
  type KpiDashboard,
  type OverviewMetrics,
  type Property,
} from "../lib/api";

type Props = {};

function formatPercent(x: number): string {
  return `${(x * 100).toFixed(1)}%`;
}

function formatMoney(x: number | null): string {
  if (x == null) return "—";
  return x.toLocaleString(undefined, { maximumFractionDigits: 2 });
}

export function OverviewDashboard(_props: Props) {
  const [properties, setProperties] = useState<Property[]>([]);
  const [selectedIds, setSelectedIds] = useState<number[]>([]);
  const [startDate, setStartDate] = useState<string>("");
  const [endDate, setEndDate] = useState<string>("");

  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [metrics, setMetrics] = useState<OverviewMetrics | null>(null);
  const [arrivals, setArrivals] = useState<ArrivalsByMonthRow[]>([]);
  const [kpis, setKpis] = useState<KpiDashboard | null>(null);
  const [cancellations, setCancellations] = useState<CancellationByMonthRow[]>([]);
  const [segmentMix, setSegmentMix] = useState<CategoricalBreakdownRow[]>([]);
  const [channelMix, setChannelMix] = useState<CategoricalBreakdownRow[]>([]);
  const [countryMix, setCountryMix] = useState<CategoricalBreakdownRow[]>([]);

  const selectedName = useMemo(() => {
    const selected = new Set(selectedIds);
    const names = properties.filter((p) => selected.has(p.id)).map((p) => p.name);
    if (names.length === 0) return "No properties";
    if (names.length === properties.length) return "All properties";
    return names.join(", ");
  }, [properties, selectedIds]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const props = await listProperties();
        if (cancelled) return;
        setProperties(props);
        setSelectedIds(props.map((p) => p.id));
      } catch (e) {
        if (!cancelled) setError(String(e));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  async function refresh() {
    setError(null);
    setLoading(true);
    try {
      const m = await getOverviewMetrics({
        propertyIds: selectedIds,
        startDate: startDate || undefined,
        endDate: endDate || undefined,
      });
      setMetrics(m);

      // If user hasn't chosen a range yet, default to dataset bounds.
      if (!startDate && m.min_arrival_date) setStartDate(m.min_arrival_date);
      if (!endDate && m.max_arrival_date) setEndDate(m.max_arrival_date);

      const a = await getArrivalsByMonth({
        propertyIds: selectedIds,
        startDate: (startDate || m.min_arrival_date || undefined) ?? undefined,
        endDate: (endDate || m.max_arrival_date || undefined) ?? undefined,
      });
      setArrivals(a);

      const k = await getKpiDashboard({
        propertyIds: selectedIds,
        startDate: (startDate || m.min_arrival_date || undefined) ?? undefined,
        endDate: (endDate || m.max_arrival_date || undefined) ?? undefined,
        comparePreviousPeriod: true,
      });
      setKpis(k);

      const [c, seg, ch, co] = await Promise.all([
        getCancellationsByMonth({
          propertyIds: selectedIds,
          startDate: (startDate || m.min_arrival_date || undefined) ?? undefined,
          endDate: (endDate || m.max_arrival_date || undefined) ?? undefined,
        }),
        getMarketSegmentMix({
          propertyIds: selectedIds,
          startDate: (startDate || m.min_arrival_date || undefined) ?? undefined,
          endDate: (endDate || m.max_arrival_date || undefined) ?? undefined,
          limit: 8,
        }),
        getDistributionChannelMix({
          propertyIds: selectedIds,
          startDate: (startDate || m.min_arrival_date || undefined) ?? undefined,
          endDate: (endDate || m.max_arrival_date || undefined) ?? undefined,
          limit: 8,
        }),
        getCountryMix({
          propertyIds: selectedIds,
          startDate: (startDate || m.min_arrival_date || undefined) ?? undefined,
          endDate: (endDate || m.max_arrival_date || undefined) ?? undefined,
          limit: 8,
        }),
      ]);
      setCancellations(c);
      setSegmentMix(seg);
      setChannelMix(ch);
      setCountryMix(co);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    if (selectedIds.length === 0) return;
    // Initial load once properties selected.
    void refresh();
  }, [selectedIds.join(",")]);

  return (
    <section class="panel">
      <div class="headerRow">
        <span class="pill">Report</span>
        <h2 class="sectionTitle">Overview</h2>
        <span class="subtle">• {selectedName}</span>
      </div>

      <div class="panel">
        <h3>Filters</h3>
        {properties.map((p) => {
          const checked = selectedIds.includes(p.id);
          return (
            <div key={p.id}>
              <label>
                <input
                  type="checkbox"
                  checked={checked}
                  onChange={(e) => {
                    const next = new Set(selectedIds);
                    if (e.currentTarget.checked) next.add(p.id);
                    else next.delete(p.id);
                    setSelectedIds(Array.from(next));
                  }}
                />
                {p.name}
              </label>
            </div>
          );
        })}
        <div class="row">
        <label>
          Start
          <input type="date" value={startDate} onInput={(e) => setStartDate(e.currentTarget.value)} />
        </label>
        <label>
          End
          <input type="date" value={endDate} onInput={(e) => setEndDate(e.currentTarget.value)} />
        </label>
        <button type="button" onClick={refresh} disabled={loading || selectedIds.length === 0}>
          {loading ? "Loading..." : "Refresh"}
        </button>
      </div>
      </div>

      {error ? <p>{error}</p> : null}

      {kpis ? (
        <div>
          <h3>KPIs</h3>
          <p class="subtle">
            Current: {kpis.start_date ?? "—"} → {kpis.end_date ?? "—"}
            {kpis.previous_start_date && kpis.previous_end_date
              ? ` (prev: ${kpis.previous_start_date} → ${kpis.previous_end_date})`
              : ""}
          </p>
          <div class="kpiGrid">
            {kpis.cards.map((c) => {
              const deltaClass =
                c.delta == null
                  ? "deltaFlat"
                  : c.delta > 0
                    ? "deltaUp"
                    : c.delta < 0
                      ? "deltaDown"
                      : "deltaFlat";

              return (
                <div key={c.key} class="kpiCard">
                  <div class="kpiLabel">{c.label}</div>
                  <div class="kpiValue">
                    {c.value == null ? "—" : c.value.toLocaleString(undefined, { maximumFractionDigits: 2 })}
                  </div>
                  <div class={`kpiDelta ${deltaClass}`}>
                    {c.value != null && c.delta != null
                      ? `Δ ${c.delta >= 0 ? "+" : ""}${c.delta.toLocaleString(undefined, {
                          maximumFractionDigits: 2,
                        })}${c.delta_pct != null ? ` (${(c.delta_pct * 100).toFixed(1)}%)` : ""}`
                      : "—"}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      ) : null}

      <div class="panel">
        <h3>Mini visuals</h3>

        <div class="grid2">
          <div>
            <h3 class="sectionTitle">Cancellations by month</h3>
            <div class="tableWrap">
              {cancellations.length === 0 ? (
                <p>—</p>
              ) : (
                <table>
                  <thead>
                    <tr>
                      <th>Month</th>
                      <th>Total</th>
                      <th>Canceled</th>
                      <th>Rate</th>
                    </tr>
                  </thead>
                  <tbody>
                    {cancellations.map((r) => (
                      <tr key={r.month}>
                        <td>{r.month}</td>
                        <td>{r.bookings_total.toLocaleString()}</td>
                        <td>{r.bookings_canceled.toLocaleString()}</td>
                        <td>{(r.cancellation_rate * 100).toFixed(1)}%</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>
          </div>

          <div>
            <h3 class="sectionTitle">Market segment mix (top)</h3>
            <div class="tableWrap">
              {segmentMix.length === 0 ? (
                <p>—</p>
              ) : (
                <table>
                  <thead>
                    <tr>
                      <th>Segment</th>
                      <th>Count</th>
                      <th>Share</th>
                    </tr>
                  </thead>
                  <tbody>
                    {segmentMix.map((r) => (
                      <tr key={r.key}>
                        <td>{r.key}</td>
                        <td>{r.count.toLocaleString()}</td>
                        <td>{(r.share * 100).toFixed(1)}%</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>
          </div>
        </div>

        <div class="grid2">
          <div>
            <h3 class="sectionTitle">Distribution channel mix (top)</h3>
            <div class="tableWrap">
              {channelMix.length === 0 ? (
                <p>—</p>
              ) : (
                <table>
                  <thead>
                    <tr>
                      <th>Channel</th>
                      <th>Count</th>
                      <th>Share</th>
                    </tr>
                  </thead>
                  <tbody>
                    {channelMix.map((r) => (
                      <tr key={r.key}>
                        <td>{r.key}</td>
                        <td>{r.count.toLocaleString()}</td>
                        <td>{(r.share * 100).toFixed(1)}%</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>
          </div>

          <div>
            <h3 class="sectionTitle">Country mix (top)</h3>
            <div class="tableWrap">
              {countryMix.length === 0 ? (
                <p>—</p>
              ) : (
                <table>
                  <thead>
                    <tr>
                      <th>Country</th>
                      <th>Count</th>
                      <th>Share</th>
                    </tr>
                  </thead>
                  <tbody>
                    {countryMix.map((r) => (
                      <tr key={r.key}>
                        <td>{r.key}</td>
                        <td>{r.count.toLocaleString()}</td>
                        <td>{(r.share * 100).toFixed(1)}%</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>
          </div>
        </div>
      </div>

      {metrics ? (
        <div>
          <p>
            Range: {metrics.min_arrival_date ?? "—"} → {metrics.max_arrival_date ?? "—"}
          </p>
          <ul>
            <li>Bookings: {metrics.bookings_total.toLocaleString()}</li>
            <li>
              Canceled: {metrics.bookings_canceled.toLocaleString()} ({formatPercent(metrics.cancellation_rate)})
            </li>
            <li>Room-nights (non-canceled): {metrics.room_nights.toLocaleString()}</li>
            <li>Avg LOS (non-canceled): {metrics.avg_los.toFixed(2)}</li>
            <li>ADR avg (non-canceled): {formatMoney(metrics.adr_avg)}</li>
            <li>Est revenue (non-canceled): {formatMoney(metrics.est_revenue)}</li>
          </ul>
        </div>
      ) : null}

      <div>
        <h3>Arrivals by month (non-canceled)</h3>
        {arrivals.length === 0 ? (
          <p>—</p>
        ) : (
          <div class="tableWrap">
            <table>
              <thead>
                <tr>
                  <th>Month</th>
                  <th>Arrivals</th>
                </tr>
              </thead>
              <tbody>
                {arrivals.map((r) => (
                  <tr key={r.month}>
                    <td>{r.month}</td>
                    <td>{r.arrivals.toLocaleString()}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </section>
  );
}
