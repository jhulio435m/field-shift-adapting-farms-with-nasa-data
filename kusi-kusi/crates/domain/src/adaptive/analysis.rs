//! Deterministic filtering, aggregation, temporal comparison and rate forecasting.
use super::*;
use chrono::{Duration, NaiveDate};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub(crate) fn date(value: &str) -> Result<NaiveDate, Error> {
    if value.len() != 10 {
        return Err(Error::Invalid);
    }
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| Error::Invalid)
}
pub(crate) fn row_date(value: &Value) -> Result<NaiveDate, Error> {
    let text = value.as_str().ok_or(Error::Invalid)?;
    date(text.get(..10).ok_or(Error::Invalid)?)
}

pub fn matches(filters: &[Filter], row: &BTreeMap<String, Value>) -> Result<bool, Error> {
    for filter in filters {
        let value = row.get(&filter.field).ok_or(Error::Invalid)?;
        let order = match (value, &filter.value) {
            (Value::Number(a), Value::Number(b)) => a
                .as_f64()
                .and_then(|a| b.as_f64().and_then(|b| a.partial_cmp(&b))),
            (Value::String(a), Value::String(b)) => Some(a.cmp(b)),
            _ if value == &filter.value => Some(std::cmp::Ordering::Equal),
            _ => None,
        };
        let accepts = match filter.op {
            Operator::Eq => value == &filter.value,
            Operator::Ge => order.is_some_and(|o| !o.is_lt()),
            Operator::Le => order.is_some_and(|o| !o.is_gt()),
        };
        if !accepts {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn analyze(
    catalog: &Catalog,
    query: &Query,
    batch: &Batch,
    today: NaiveDate,
) -> Result<Evidence, Error> {
    catalog.validate(
        &Plan {
            version: catalog.version.clone(),
            queries: vec![query.clone()],
        },
        today,
    )?;
    if batch.truncated || batch.rows.len() > 10000 {
        return Err(Error::Incomplete);
    }
    let resource = catalog.resource(&query.resource)?;
    let required = query.fields(resource)?;
    for row in &batch.rows {
        if required.iter().any(|field| !row.contains_key(field)) {
            return Err(Error::Invalid);
        }
        resource.validate_values(row)?;
    }
    let rows = batch
        .rows
        .iter()
        .filter_map(|row| match matches_typed(resource, &query.filters, row) {
            Ok(true) => Some(Ok(row)),
            Ok(false) => None,
            Err(e) => Some(Err(e)),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut results = Vec::new();
    for analysis in &query.analyses {
        let value = match analysis {
            Analysis::Count => json!({"op":"count","value":rows.len()}),
            Analysis::Group { field } => {
                let mut counts = BTreeMap::<String, usize>::new();
                for row in &rows {
                    let value = row.get(field).ok_or(Error::Invalid)?;
                    *counts
                        .entry(
                            value
                                .as_str()
                                .map(str::to_owned)
                                .unwrap_or_else(|| value.to_string()),
                        )
                        .or_default() += 1;
                }
                json!({"op":"group","field":field,"groups":counts})
            }
            Analysis::Sum { field } | Analysis::Mean { field } => {
                let values = rows
                    .iter()
                    .map(|row| row.get(field).ok_or(Error::Invalid))
                    .collect::<Result<Vec<_>, _>>()?;
                let values = values
                    .into_iter()
                    .filter(|v| !v.is_null())
                    .map(|v| v.as_f64().ok_or(Error::Invalid))
                    .collect::<Result<Vec<_>, _>>()?;
                let sum: f64 = values.iter().sum();
                if !sum.is_finite() {
                    return Err(Error::Invalid);
                }
                match analysis {
                    Analysis::Sum { .. } => {
                        json!({"op":"sum","field":field,"value":sum,"non_null_rows":values.len()})
                    }
                    _ => {
                        json!({"op":"mean","field":field,"value":if values.is_empty() {None} else {Some(sum/values.len() as f64)},"non_null_rows":values.len()})
                    }
                }
            }
            Analysis::Compare {
                date_field,
                start,
                end,
                previous_start,
                previous_end,
            } => {
                let (a, b, c, d) = (
                    date(start)?,
                    date(end)?,
                    date(previous_start)?,
                    date(previous_end)?,
                );
                let dates = rows
                    .iter()
                    .map(|row| row_date(row.get(date_field).ok_or(Error::Invalid)?))
                    .collect::<Result<Vec<_>, _>>()?;
                let current = dates.iter().filter(|v| a <= **v && **v <= b).count();
                let previous = dates.iter().filter(|v| c <= **v && **v <= d).count();
                json!({"op":"compare","date_field":date_field,"start":start,"end":end,"previous_start":previous_start,"previous_end":previous_end,"current":current,"previous":previous,"difference":current as i64-previous as i64})
            }
            Analysis::Forecast {
                date_field,
                target_date,
                window_days,
            } => {
                let dates = rows
                    .iter()
                    .map(|row| row_date(row.get(date_field).ok_or(Error::Invalid)?))
                    .collect::<Result<Vec<_>, _>>()?;
                if dates.iter().any(|d| *d > today) {
                    return Err(Error::Invalid);
                }
                let first = dates.iter().min().copied().ok_or(Error::Incomplete)?;
                let start = std::cmp::max(first, today - Duration::days(i64::from(*window_days)));
                let days = (today - start).num_days();
                if days < 1 {
                    return Err(Error::Incomplete);
                }
                let observed = dates.iter().filter(|d| **d >= start && **d < today).count();
                let rate = observed as f64 / days as f64;
                let horizon = (date(target_date)? - today).num_days();
                json!({"op":"forecast","date_field":date_field,"target_date":target_date,"current":rows.len(),"observed":observed,"start":start.to_string(),"full_days":days,"daily_rate":rate,"remaining_days":horizon,"central":rows.len() as f64+(rate*horizon as f64).round(),"lower_scenario":rows.len() as f64+(rate*horizon as f64*0.5).round(),"upper_scenario":rows.len() as f64+(rate*horizon as f64*1.5).round(),"method":"constant_daily_rate_complete_days","assumptions":["zero days included; today excluded","50%/150% sensitivity, not confidence intervals","no campaigns, seasonality or capacity modeled"]})
            }
        };
        results.push(value);
    }
    Ok(Evidence {
        source: catalog.source.clone(),
        resource: query.resource.clone(),
        version: catalog.version.clone(),
        filters: query.filters.clone(),
        observed_rows: rows.len(),
        results,
    })
}

pub fn matches_typed(
    resource: &Resource,
    filters: &[Filter],
    row: &BTreeMap<String, Value>,
) -> Result<bool, Error> {
    let mut normalized = row.clone();
    for filter in filters {
        if resource.field(&filter.field)?.kind == FieldKind::Date {
            let value = row.get(&filter.field).ok_or(Error::Invalid)?;
            if value.is_null() {
                return Ok(false);
            }
            normalized.insert(
                filter.field.clone(),
                Value::String(row_date(value)?.to_string()),
            );
        }
    }
    matches(filters, &normalized)
}
