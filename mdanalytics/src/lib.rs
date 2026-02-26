pub mod types;
use polars::prelude::*;
use serde_json::Value;
use std::io::Cursor;
use std::path::PathBuf;

pub fn read_data_frame_from_csv(path_str: String) -> DataFrame {
    let file_path = PathBuf::from(path_str);

    CsvReadOptions::default()
        .with_has_header(true)
        .try_into_reader_with_file_path(Some(file_path))
        .expect("Failed to create CSV reader")
        .finish()
        .expect("Failed to parse CSV into DataFrame")
}

pub fn json_to_dataframe(
    raw_json: &str,
    float_cols: &[&str],
) -> Result<DataFrame, polars::error::PolarsError> {
    let json: Value = serde_json::from_str(raw_json)
        .map_err(|e| polars::error::PolarsError::ComputeError(e.to_string().into()))?;

    // Extract the series (handling the nested InfluxQL JSON structure)
    let series_list = match json["results"][0]["series"].as_array() {
        Some(list) if !list.is_empty() => list,
        _ => return Ok(DataFrame::default()), // Return empty DataFrame if no series
    };
    let first_series = &series_list[0];

    let column_names: Vec<String> = first_series["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    let rows = first_series["values"]
        .as_array()
        .ok_or_else(|| polars::error::PolarsError::ComputeError("No values found".into()))?;

    let mut polars_columns = Vec::new();

    for (col_idx, name) in column_names.iter().enumerate() {
        let col_data: Vec<Option<String>> = rows
            .iter()
            .map(|row| {
                let val = &row[col_idx];
                if val.is_null() {
                    None
                } else if let Some(s) = val.as_str() {
                    Some(s.to_string())
                } else {
                    Some(val.to_string())
                }
            })
            .collect();

        polars_columns.push(Column::from(Series::new(name.into(), col_data)));
    }

    let mut df = DataFrame::new_infer_height(polars_columns)?;

    // --- TEMPORAL PARSING ---
    // Convert the string "time" column to a proper Datetime type
    if df.column("time").is_ok() {
        df = df
            .lazy()
            .with_column(col("time").str().to_datetime(
                Some(TimeUnit::Microseconds), // InfluxDB typically uses µs/ns precision
                None,
                StrptimeOptions::default(),
                lit("raise"),
            ))
            .collect()?;
    }

    // Cast numeric fields
    for &c in float_cols {
        if df.column(c).is_ok() {
            df = df
                .lazy()
                .with_column(col(c).cast(DataType::Float64))
                .collect()?;
        }
    }

    Ok(df)
}

pub fn dataframe_to_json_value(df: &DataFrame) -> Result<Value, PolarsError> {
    let mut buf = Cursor::new(Vec::new());
    let mut df_clone = df.clone(); // Clone the DataFrame to make it mutable
    JsonWriter::new(&mut buf)
        .with_json_format(JsonFormat::Json)
        .finish(&mut df_clone)?; // Pass a mutable reference to the clone
    let json_bytes = buf.into_inner();
    let json_value: Value = serde_json::from_slice(&json_bytes)
        .map_err(|e| PolarsError::ComputeError(e.to_string().into()))?;
    Ok(json_value)
}
