import { useState } from "preact/hooks";
import { open } from "@tauri-apps/plugin-dialog";
import { importBookingsCsv, previewBookingsCsv, type CsvPreview, type ImportBookingsResult } from "../lib/api";

type Props = {
  onPreviewLoaded?: (preview: CsvPreview) => void;
};

export function ImportBookingsCsv({ onPreviewLoaded }: Props) {
  const [filePath, setFilePath] = useState<string | null>(null);
  const [preview, setPreview] = useState<CsvPreview | null>(null);
  const [importResult, setImportResult] = useState<ImportBookingsResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [importing, setImporting] = useState(false);

  async function pickFile() {
    setError(null);
    setImportResult(null);
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "CSV", extensions: ["csv"] }],
    });

    if (!selected) return;
    if (Array.isArray(selected)) {
      setError("Please select a single CSV file");
      return;
    }

    setFilePath(selected);
    setLoading(true);
    try {
      const p = await previewBookingsCsv(selected, 25);
      setPreview(p);
      onPreviewLoaded?.(p);
    } catch (e) {
      setError(String(e));
      setPreview(null);
    } finally {
      setLoading(false);
    }
  }

  async function runImport() {
    if (!filePath) return;
    setError(null);
    setImporting(true);
    try {
      const result = await importBookingsCsv(filePath);
      setImportResult(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setImporting(false);
    }
  }

  return (
    <section class="panel">
      <div class="row">
        <span class="pill">Data</span>
        <strong>Bookings CSV</strong>
        <button type="button" onClick={pickFile} disabled={loading || importing}>
          {loading ? "Loading..." : "Pick bookings CSV"}
        </button>
        <button
          type="button"
          onClick={runImport}
          disabled={!filePath || !preview || loading || importing}
        >
          {importing ? "Importing..." : "Import"}
        </button>
      </div>

      {filePath ? (
        <p class="subtle">
          File: <span class="mono">{filePath}</span>
        </p>
      ) : null}
      {error ? <p>{error}</p> : null}

      {preview ? (
        <div>
          <p class="subtle">
            Preview: {preview.rows.length} rows, {preview.headers.length} columns
          </p>
          <p class="mono subtle">{preview.headers.join(", ")}</p>
        </div>
      ) : null}

      {importResult ? (
        <div>
          <p class="subtle">
            Imported file hash: <span class="mono">{importResult.file_hash}</span>
          </p>
          <div>
            <p>Results:</p>
            <ul>
              {importResult.properties.map((p) => (
                <li key={p.property_id}>
                  <strong>{p.property_name}</strong>: {p.blocked_duplicate ? "BLOCKED" : "OK"} — imported {p.rows_imported} / rejected {p.rows_rejected}
                  {p.blocked_message ? <div>{p.blocked_message}</div> : null}
                </li>
              ))}
            </ul>
          </div>
        </div>
      ) : null}
    </section>
  );
}
