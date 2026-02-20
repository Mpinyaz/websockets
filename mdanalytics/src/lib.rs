use polars::prelude::*;
use std::path::PathBuf;
pub fn read_data_frame_from_csv(path_str: String) -> DataFrame {
    let file_path = PathBuf::from(path_str);

    CsvReadOptions::default()
        .with_has_header(true)
        .try_into_reader_with_file_path(Some(file_path))
        .expect("Failed to create CSV reader")
        .finish()
        .expect("Failed to parse CSV into DataFrame") // Handle parsing errors
}
