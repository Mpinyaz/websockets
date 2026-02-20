use mdanalytics::read_data_frame_from_csv;
use polars::prelude::*;

fn main() -> PolarsResult<()> {
    let df = read_data_frame_from_csv("./input.csv".into());
    println!("Schema: \n{:?} ", df.schema());
    let area = df
        .lazy()
        .with_column((col("SepalWidthCm") * col("SepalLengthCm")).alias("SepalArea"))
        .collect()
        .unwrap();

    if let Some(arr) = area.get(26) {
        for val in arr {
            println!("{:?}", val)
        }
    }

    println!("{:?}", area);

    let describe = area
        .clone()
        .lazy()
        .select([
            col("SepalWidthCm").mean().alias("Average Sepal Width"),
            col("SepalWidthCm").var(1).alias("Variation"),
            col("SepalArea").max().alias("Maximum Sepal Area"),
        ])
        .collect();
    println!("{:?}", describe);

    let filter: DataFrame = area
        .clone()
        .lazy()
        .filter(col("SepalLengthCm").gt(lit(5.2)))
        .collect()?;

    println!("{}", filter);
    println!("{:?}", filter.get_column_names());

    let slice = area.column("SepalLengthCm")?.slice(30, 10);
    println!("{:?}", slice);

    println!("Length {:?}", slice.len());
    Ok(())
}
