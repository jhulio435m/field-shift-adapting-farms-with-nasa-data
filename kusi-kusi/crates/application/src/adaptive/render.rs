//! Evidence rendering for adaptive queries.
use kusi_domain::adaptive::{Catalog, Evidence, Operator};

pub fn render(catalog: &Catalog, evidence: &[Evidence]) -> String {
    let mut lines = Vec::new();
    for item in evidence {
        let resource = catalog.resource(&item.resource).ok();
        let label = resource.map(|r| r.label.as_str()).unwrap_or(&item.resource);
        lines.push(format!("{label} — fuente {}", item.source));
        if !item.filters.is_empty() {
            let filters = item
                .filters
                .iter()
                .map(|f| {
                    let label = resource
                        .and_then(|r| r.field(&f.field).ok())
                        .map(|f| f.label.as_str())
                        .unwrap_or(&f.field);
                    let op = match f.op {
                        Operator::Eq => "es",
                        Operator::Ge => "desde / al menos",
                        Operator::Le => "hasta / como máximo",
                    };
                    let value = f
                        .value
                        .as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| f.value.to_string());
                    format!("{label} {op} {value}")
                })
                .collect::<Vec<_>>()
                .join("; ");
            lines.push(format!("Filtros: {filters}."));
        }
        for result in &item.results {
            let field_label = |key: &str| {
                resource
                    .and_then(|r| result[key].as_str().and_then(|id| r.field(id).ok()))
                    .map(|f| f.label.clone())
                    .unwrap_or_default()
            };
            match result["op"].as_str() {
                Some("count") => lines.push(format!("Total: {}.", result["value"])),
                Some("group") => {
                    lines.push(format!("Por {}:", field_label("field")));
                    if let Some(groups) = result["groups"].as_object() {
                        for (key, value) in groups {
                            lines.push(format!("• {key}: {value}"));
                        }
                    }
                }
                Some("sum") => lines.push(format!(
                    "Suma de {}: {} ({} valores no nulos).",
                    field_label("field"),
                    result["value"],
                    result["non_null_rows"]
                )),
                Some("mean") => lines.push(format!(
                    "Media de {}: {} ({} valores no nulos).",
                    field_label("field"),
                    result["value"],
                    result["non_null_rows"]
                )),
                Some("compare") => lines.push(format!(
                    "Comparación: {} en {}–{}, frente a {} en {}–{}. Diferencia: {}.",
                    result["current"],
                    result["start"].as_str().unwrap_or("?"),
                    result["end"].as_str().unwrap_or("?"),
                    result["previous"],
                    result["previous_start"].as_str().unwrap_or("?"),
                    result["previous_end"].as_str().unwrap_or("?"),
                    result["difference"]
                )),
                Some("forecast") => lines.push(format!(
                    "Proyección al {}: aproximadamente {}. Actualmente: {}.\nHistórico: {} registros en {} días completos, desde {}; ritmo {} por día. Horizonte: {} días.\nEscenarios al 50 % y 150 % del ritmo: {}–{}. Se excluye hoy, se incluyen días sin registros. Escenarios lineales, no intervalos de confianza ni cifras garantizadas; no modelan campañas, cupos ni estacionalidad.",
                    result["target_date"].as_str().unwrap_or("?"),
                    display_number(&result["central"]),
                    result["current"],
                    result["observed"],
                    result["full_days"],
                    result["start"].as_str().unwrap_or("?"),
                    display_number(&result["daily_rate"]),
                    result["remaining_days"],
                    display_number(&result["lower_scenario"]),
                    display_number(&result["upper_scenario"])
                )),
                _ => (),
            }
        }
        lines.push(format!(
            "Datos completos para esta lectura: {} filas. Alcance y filtros indicados arriba.",
            item.observed_rows
        ));
    }
    lines.join("\n")
}

fn display_number(value: &serde_json::Value) -> String {
    value
        .as_f64()
        .map(|n| {
            format!("{n:.2}")
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_owned()
        })
        .unwrap_or_else(|| "?".into())
}
