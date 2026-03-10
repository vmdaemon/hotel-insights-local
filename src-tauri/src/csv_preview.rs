use std::path::Path;

#[derive(serde::Serialize)]
pub struct CsvPreview {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

pub fn preview_csv(file_path: &Path, max_rows: usize) -> Result<CsvPreview, String> {
    let file = std::fs::File::open(file_path)
        .map_err(|e| format!("failed to open csv file: {e}"))?;

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(file);

    let headers = reader
        .headers()
        .map_err(|e| format!("failed to read csv headers: {e}"))?
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    let mut rows = Vec::new();
    for result in reader.records().take(max_rows) {
        let record = result.map_err(|e| format!("failed to read csv row: {e}"))?;
        rows.push(record.iter().map(|s| s.to_string()).collect());
    }

    Ok(CsvPreview { headers, rows })
}
