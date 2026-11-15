//! Business- and transport-independent discovery, query plans and reproducible analysis.
mod analysis;
pub use analysis::{analyze, matches, matches_typed};
pub(crate) use analysis::{date, row_date};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    Category,
    Number,
    Date,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub id: String,
    pub label: String,
    pub kind: FieldKind,
    #[serde(default)]
    pub options: Vec<Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resource {
    pub id: String,
    pub label: String,
    pub fields: Vec<Field>,
    #[serde(default)]
    pub events: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub source: String,
    pub version: String,
    pub resources: Vec<Resource>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    Eq,
    Ge,
    Le,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Filter {
    pub field: String,
    pub op: Operator,
    pub value: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Analysis {
    Count,
    Group {
        field: String,
    },
    Sum {
        field: String,
    },
    Mean {
        field: String,
    },
    Compare {
        date_field: String,
        start: String,
        end: String,
        previous_start: String,
        previous_end: String,
    },
    Forecast {
        date_field: String,
        target_date: String,
        window_days: u32,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Query {
    pub resource: String,
    #[serde(default)]
    pub filters: Vec<Filter>,
    pub analyses: Vec<Analysis>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub version: String,
    pub queries: Vec<Query>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Decision {
    Query {
        plan: Plan,
    },
    Clarify {
        question: String,
        #[serde(default)]
        draft: Option<Query>,
    },
    Subscribe {
        resource: String,
        event: String,
        #[serde(default)]
        filters: Vec<Filter>,
    },
    Cancel {
        subscription: String,
    },
    Reset {},
    Unsupported {
        capability: String,
        explanation: String,
        #[serde(default)]
        requires_approval: bool,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Interpretation {
    pub term: String,
    pub meaning: String,
    pub resource: String,
    pub version: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct SubscriptionHint {
    pub id: String,
    pub resource: String,
    pub event: String,
    pub filters: Vec<Filter>,
    pub status: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub previous: Option<Plan>,
    pub question: Option<String>,
    pub draft: Option<Query>,
    #[serde(default)]
    pub interpretations: Vec<Interpretation>,
    #[serde(default)]
    pub schemas: Vec<Resource>,
    #[serde(default)]
    pub subscriptions: Vec<SubscriptionHint>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Batch {
    pub rows: Vec<BTreeMap<String, Value>>,
    pub truncated: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub source: String,
    pub resource: String,
    pub version: String,
    pub filters: Vec<Filter>,
    pub observed_rows: usize,
    pub results: Vec<Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reply {
    pub status: String,
    pub text: String,
    #[serde(default)]
    pub evidence: Vec<Evidence>,
}
impl Reply {
    pub fn message(status: &str, text: impl Into<String>) -> Self {
        Self {
            status: status.into(),
            text: text.into(),
            evidence: Vec::new(),
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("configured provider or execution budget reached")]
    Limited,
    #[error("access denied or unavailable capability")]
    Forbidden,
    #[error("invalid or stale plan")]
    Invalid,
    #[error("source or persistence unavailable")]
    Unavailable,
    #[error("incomplete data or configured limit reached")]
    Incomplete,
}

impl Resource {
    pub fn field(&self, id: &str) -> Result<&Field, Error> {
        self.fields
            .iter()
            .find(|f| f.id == id)
            .ok_or(Error::Forbidden)
    }
    pub fn validate_filters(&self, filters: &[Filter]) -> Result<(), Error> {
        if filters.len() > 8 {
            return Err(Error::Invalid);
        }
        for filter in filters {
            let field = self.field(&filter.field)?;
            match field.kind {
                FieldKind::Category => {
                    if filter.op != Operator::Eq || !field.options.contains(&filter.value) {
                        return Err(Error::Invalid);
                    }
                }
                FieldKind::Number => {
                    if !filter.value.as_f64().is_some_and(f64::is_finite) {
                        return Err(Error::Invalid);
                    }
                }
                FieldKind::Date => {
                    date(filter.value.as_str().ok_or(Error::Invalid)?)?;
                }
            }
        }
        Ok(())
    }
    pub fn validate_values(&self, row: &BTreeMap<String, Value>) -> Result<(), Error> {
        for (id, value) in row {
            let field = self.field(id)?;
            if value.is_null() {
                continue;
            }
            match field.kind {
                FieldKind::Category if !field.options.contains(value) => {
                    return Err(Error::Invalid);
                }
                FieldKind::Number if !value.as_f64().is_some_and(f64::is_finite) => {
                    return Err(Error::Invalid);
                }
                FieldKind::Date => {
                    row_date(value)?;
                }
                _ => (),
            }
        }
        Ok(())
    }
    fn typed(&self, id: &str, kind: FieldKind) -> Result<(), Error> {
        if self.field(id)?.kind == kind {
            Ok(())
        } else {
            Err(Error::Invalid)
        }
    }
}
impl Catalog {
    pub fn resource(&self, id: &str) -> Result<&Resource, Error> {
        self.resources
            .iter()
            .find(|r| r.id == id)
            .ok_or(Error::Forbidden)
    }
    pub fn validate(&self, plan: &Plan, today: NaiveDate) -> Result<(), Error> {
        if plan.version != self.version || plan.queries.is_empty() || plan.queries.len() > 4 {
            return Err(Error::Invalid);
        }
        for query in &plan.queries {
            let resource = self.resource(&query.resource)?;
            resource.validate_filters(&query.filters)?;
            if query.analyses.is_empty() || query.analyses.len() > 8 {
                return Err(Error::Invalid);
            }
            for analysis in &query.analyses {
                match analysis {
                    Analysis::Count => (),
                    Analysis::Group { field } => resource.typed(field, FieldKind::Category)?,
                    Analysis::Sum { field } | Analysis::Mean { field } => {
                        resource.typed(field, FieldKind::Number)?
                    }
                    Analysis::Forecast {
                        date_field,
                        target_date,
                        window_days,
                    } => {
                        resource.typed(date_field, FieldKind::Date)?;
                        if !(1..=366).contains(&(date(target_date)? - today).num_days())
                            || !(2..=365).contains(window_days)
                        {
                            return Err(Error::Invalid);
                        }
                    }
                    Analysis::Compare {
                        date_field,
                        start,
                        end,
                        previous_start,
                        previous_end,
                    } => {
                        resource.typed(date_field, FieldKind::Date)?;
                        let (a, b, c, d) = (
                            date(start)?,
                            date(end)?,
                            date(previous_start)?,
                            date(previous_end)?,
                        );
                        if a > b
                            || c > d
                            || b > today
                            || d > today
                            || b - a != d - c
                            || !(b < c || d < a)
                        {
                            return Err(Error::Invalid);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
impl Query {
    pub fn fields(&self, resource: &Resource) -> Result<Vec<String>, Error> {
        let mut fields = BTreeSet::new();
        for analysis in &self.analyses {
            match analysis {
                Analysis::Count => (),
                Analysis::Group { field } | Analysis::Sum { field } | Analysis::Mean { field } => {
                    fields.insert(field.clone());
                }
                Analysis::Compare { date_field, .. } | Analysis::Forecast { date_field, .. } => {
                    fields.insert(date_field.clone());
                }
            }
        }
        for filter in &self.filters {
            fields.insert(filter.field.clone());
        }
        if fields.is_empty() {
            fields.insert(resource.fields.first().ok_or(Error::Forbidden)?.id.clone());
        }
        if fields.len() > 16 {
            return Err(Error::Invalid);
        }
        Ok(fields.into_iter().collect())
    }
}
