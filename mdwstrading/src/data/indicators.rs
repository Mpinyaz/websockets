use crate::data::Ohlcv;
use chrono::NaiveDateTime;
use polars::{df, error::PolarsResult, frame::DataFrame, prelude::Column};
use rust_decimal::prelude::ToPrimitive;
use ta::indicators::{AverageTrueRange, ExponentialMovingAverage, RelativeStrengthIndex};
use ta::{DataItem, Next};

pub fn ohlcv_to_df(data: Vec<Ohlcv>) -> PolarsResult<DataFrame> {
    let mut ts = Vec::with_capacity(data.len());
    let mut open = Vec::with_capacity(data.len());
    let mut high = Vec::with_capacity(data.len());
    let mut low = Vec::with_capacity(data.len());
    let mut close = Vec::with_capacity(data.len());
    let mut vol = Vec::with_capacity(data.len());

    for item in data {
        ts.push(item.timestamp);
        open.push(item.open);
        high.push(item.high);
        low.push(item.low);
        close.push(item.close);
        vol.push(item.volume);
    }
    let naive_ts: Vec<NaiveDateTime> = ts.into_iter().map(|dt| dt.naive_utc()).collect();
    let open_f64: Vec<f64> = open
        .into_iter()
        .map(|d| d.to_f64().unwrap_or(f64::NAN))
        .collect();
    let close_f64: Vec<f64> = close
        .into_iter()
        .map(|d| d.to_f64().unwrap_or(f64::NAN))
        .collect();
    let high_f64: Vec<f64> = high
        .into_iter()
        .map(|d| d.to_f64().unwrap_or(f64::NAN))
        .collect();
    let low_f64: Vec<f64> = low
        .into_iter()
        .map(|d| d.to_f64().unwrap_or(f64::NAN))
        .collect();
    let vol_f64: Vec<f64> = vol
        .into_iter()
        .map(|d| d.to_f64().unwrap_or(f64::NAN))
        .collect();
    df!(
        "timestamp" => naive_ts,
        "open" => open_f64,
        "high" => high_f64,
        "low" => low_f64,
        "close" => close_f64,
        "volume" => vol_f64,
    )
}
pub fn compute_indicators(data: Vec<Ohlcv>) -> PolarsResult<DataFrame> {
    // 1. Get the base DataFrame
    let df = ohlcv_to_df(data.clone())?;

    // 2. Initialize Indicators
    let mut rsi = RelativeStrengthIndex::new(14).unwrap();
    let mut ema_fast = ExponentialMovingAverage::new(12).unwrap();
    let mut atr = AverageTrueRange::new(14).unwrap();

    // 3. Process data using ta::DataItem
    let mut rsi_vals = Vec::with_capacity(data.len());
    let mut ema_vals = Vec::with_capacity(data.len());
    let mut atr_vals = Vec::with_capacity(data.len());

    for item in data {
        // Build DataItem for indicators that need OHLCV (like ATR)
        let out = DataItem::builder()
            .open(item.open.to_f64().unwrap_or(f64::NAN))
            .high(item.high.to_f64().unwrap_or(f64::NAN))
            .low(item.low.to_f64().unwrap_or(f64::NAN))
            .close(item.close.to_f64().unwrap_or(f64::NAN))
            .volume(item.volume.to_f64().unwrap_or(f64::NAN))
            .build()
            .unwrap();

        rsi_vals.push(rsi.next(&out));
        ema_vals.push(ema_fast.next(&out));
        atr_vals.push(atr.next(&out));
    }

    // 4. Attach columns back to DataFrame
    df.hstack(&[
        Column::new("rsi_14".into(), rsi_vals),
        Column::new("ema_12".into(), ema_vals),
        Column::new("atr_14".into(), atr_vals),
    ])
}
