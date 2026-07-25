//! SQL lineage extraction using `sqlparser`.
//!
//! Supports a practical subset:
//! - `SELECT … FROM … [JOIN …]`
//! - `CREATE VIEW name AS SELECT …`
//! - `CREATE TABLE name AS SELECT …`
//! - `INSERT INTO target SELECT …`
//! - `WITH cte AS (…) SELECT …`
//!
//! Table-level edges are always produced. Column-level edges are produced when
//! projections are simple column references (`a.x`, `x`) or aliased columns.

use std::collections::{HashMap, HashSet};

use sqlparser::ast::{
    Cte, Expr, Ident, ObjectName, Query, Select, SelectItem, SetExpr, Statement, TableFactor,
    TableWithJoins,
};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use tracing::debug;

use drp_common::{Error, Result};

/// Extracted lineage from one SQL statement / script.
#[derive(Debug, Clone, Default)]
pub struct SqlLineageExtract {
    /// Downstream target table/view name (FQN or bare), when known.
    pub target: Option<String>,
    /// Upstream source tables (normalized names).
    pub sources: Vec<String>,
    /// Column mappings: target column → list of (source_table?, source_column).
    pub column_mappings: Vec<ColumnMapping>,
    /// Original SQL (trimmed).
    pub sql: String,
}

/// One column-level mapping inferred from a projection.
#[derive(Debug, Clone)]
pub struct ColumnMapping {
    /// Output column name (alias or source name).
    pub target_column: String,
    /// Upstream columns feeding this output.
    pub sources: Vec<(Option<String>, String)>,
    /// Expression text when available.
    pub expression: Option<String>,
}

/// Parse SQL text and extract table/column lineage facts.
pub fn extract_lineage_from_sql(sql: &str) -> Result<Vec<SqlLineageExtract>> {
    let dialect = GenericDialect {};
    let statements = Parser::parse_sql(&dialect, sql)
        .map_err(|e| Error::validation(format!("SQL parse error: {e}")))?;

    let mut out = Vec::new();
    for stmt in statements {
        if let Some(extract) = extract_from_statement(&stmt, sql)? {
            out.push(extract);
        }
    }
    if out.is_empty() {
        return Err(Error::validation(
            "no lineage-producing statement found (expected SELECT / CREATE VIEW|TABLE AS / INSERT SELECT)",
        ));
    }
    Ok(out)
}

fn extract_from_statement(stmt: &Statement, original: &str) -> Result<Option<SqlLineageExtract>> {
    match stmt {
        Statement::Query(q) => {
            let mut ex = SqlLineageExtract {
                sql: original.trim().to_string(),
                ..Default::default()
            };
            walk_query(q, &mut ex, None);
            Ok(Some(ex))
        }
        Statement::CreateView { name, query, .. } => {
            let target = object_name_to_string(name);
            let mut ex = SqlLineageExtract {
                target: Some(target.clone()),
                sql: original.trim().to_string(),
                ..Default::default()
            };
            walk_query(query, &mut ex, Some(target));
            Ok(Some(ex))
        }
        Statement::CreateTable(ct) => {
            if let Some(q) = &ct.query {
                let target = object_name_to_string(&ct.name);
                let mut ex = SqlLineageExtract {
                    target: Some(target.clone()),
                    sql: original.trim().to_string(),
                    ..Default::default()
                };
                walk_query(q, &mut ex, Some(target));
                Ok(Some(ex))
            } else {
                Ok(None)
            }
        }
        Statement::Insert(insert) => {
            let target = object_name_to_string(&insert.table_name);
            if let Some(src) = &insert.source {
                let mut ex = SqlLineageExtract {
                    target: Some(target.clone()),
                    sql: original.trim().to_string(),
                    ..Default::default()
                };
                walk_query(src, &mut ex, Some(target));
                Ok(Some(ex))
            } else {
                Ok(None)
            }
        }
        other => {
            debug!(?other, "statement type ignored for lineage");
            Ok(None)
        }
    }
}

fn walk_query(query: &Query, ex: &mut SqlLineageExtract, _target_hint: Option<String>) {
    // CTEs first — their sources are also upstream of the outer query.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            walk_cte(cte, ex);
        }
    }
    match query.body.as_ref() {
        SetExpr::Select(select) => walk_select(select, ex),
        SetExpr::Query(q) => walk_query(q, ex, None),
        SetExpr::SetOperation { left, right, .. } => {
            if let SetExpr::Select(s) = left.as_ref() {
                walk_select(s, ex);
            }
            if let SetExpr::Select(s) = right.as_ref() {
                walk_select(s, ex);
            }
        }
        _ => {}
    }
    dedupe_sources(ex);
}

fn walk_cte(cte: &Cte, ex: &mut SqlLineageExtract) {
    walk_query(&cte.query, ex, Some(cte.alias.name.value.clone()));
}

fn walk_select(select: &Select, ex: &mut SqlLineageExtract) {
    let mut alias_to_table: HashMap<String, String> = HashMap::new();
    for twj in &select.from {
        collect_table_with_joins(twj, ex, &mut alias_to_table);
    }
    for item in &select.projection {
        if let Some(m) = mapping_from_select_item(item, &alias_to_table) {
            ex.column_mappings.push(m);
        }
    }
}

fn collect_table_with_joins(
    twj: &TableWithJoins,
    ex: &mut SqlLineageExtract,
    aliases: &mut HashMap<String, String>,
) {
    collect_table_factor(&twj.relation, ex, aliases);
    for join in &twj.joins {
        collect_table_factor(&join.relation, ex, aliases);
    }
}

fn collect_table_factor(
    factor: &TableFactor,
    ex: &mut SqlLineageExtract,
    aliases: &mut HashMap<String, String>,
) {
    match factor {
        TableFactor::Table { name, alias, .. } => {
            let table = object_name_to_string(name);
            ex.sources.push(table.clone());
            if let Some(a) = alias {
                aliases.insert(a.name.value.to_ascii_lowercase(), table.clone());
            }
            // Also map bare table name as alias of itself.
            if let Some(last) = name.0.last() {
                aliases
                    .entry(last.value.to_ascii_lowercase())
                    .or_insert(table);
            }
        }
        TableFactor::Derived {
            subquery, alias, ..
        } => {
            walk_query(subquery, ex, alias.as_ref().map(|a| a.name.value.clone()));
        }
        TableFactor::NestedJoin {
            table_with_joins, ..
        } => collect_table_with_joins(table_with_joins, ex, aliases),
        _ => {}
    }
}

fn mapping_from_select_item(
    item: &SelectItem,
    aliases: &HashMap<String, String>,
) -> Option<ColumnMapping> {
    match item {
        SelectItem::UnnamedExpr(expr) => {
            let sources = columns_in_expr(expr, aliases);
            let target = match expr {
                Expr::Identifier(i) => i.value.clone(),
                Expr::CompoundIdentifier(parts) if !parts.is_empty() => {
                    parts.last().unwrap().value.clone()
                }
                _ => return None,
            };
            if sources.is_empty() {
                return None;
            }
            Some(ColumnMapping {
                target_column: target,
                sources,
                expression: Some(expr.to_string()),
            })
        }
        SelectItem::ExprWithAlias { expr, alias } => {
            let sources = columns_in_expr(expr, aliases);
            if sources.is_empty() {
                return None;
            }
            Some(ColumnMapping {
                target_column: alias.value.clone(),
                sources,
                expression: Some(expr.to_string()),
            })
        }
        SelectItem::QualifiedWildcard(_, _) | SelectItem::Wildcard(_) => None,
    }
}

fn columns_in_expr(
    expr: &Expr,
    aliases: &HashMap<String, String>,
) -> Vec<(Option<String>, String)> {
    let mut out = Vec::new();
    collect_columns(expr, aliases, &mut out);
    out
}

fn collect_columns(
    expr: &Expr,
    aliases: &HashMap<String, String>,
    out: &mut Vec<(Option<String>, String)>,
) {
    match expr {
        Expr::Identifier(Ident { value, .. }) => {
            out.push((None, value.clone()));
        }
        Expr::CompoundIdentifier(parts) if parts.len() >= 2 => {
            let col = parts.last().unwrap().value.clone();
            let table_alias = parts[parts.len() - 2].value.to_ascii_lowercase();
            let table = aliases.get(&table_alias).cloned();
            out.push((table.or(Some(parts[parts.len() - 2].value.clone())), col));
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_columns(left, aliases, out);
            collect_columns(right, aliases, out);
        }
        Expr::UnaryOp { expr, .. } => collect_columns(expr, aliases, out),
        Expr::Nested(e) => collect_columns(e, aliases, out),
        Expr::Function(f) => {
            // sqlparser 0.52 Function args structure
            if let sqlparser::ast::FunctionArguments::List(list) = &f.args {
                for arg in &list.args {
                    if let sqlparser::ast::FunctionArg::Unnamed(
                        sqlparser::ast::FunctionArgExpr::Expr(e),
                    ) = arg
                    {
                        collect_columns(e, aliases, out);
                    }
                }
            }
        }
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            if let Some(op) = operand {
                collect_columns(op, aliases, out);
            }
            for c in conditions {
                collect_columns(c, aliases, out);
            }
            for r in results {
                collect_columns(r, aliases, out);
            }
            if let Some(e) = else_result {
                collect_columns(e, aliases, out);
            }
        }
        Expr::Cast { expr, .. } => {
            collect_columns(expr, aliases, out);
        }
        _ => {}
    }
}

fn object_name_to_string(name: &ObjectName) -> String {
    name.0
        .iter()
        .map(|p| p.value.clone())
        .collect::<Vec<_>>()
        .join(".")
}

fn dedupe_sources(ex: &mut SqlLineageExtract) {
    let mut seen = HashSet::new();
    ex.sources.retain(|s| seen.insert(s.to_ascii_lowercase()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_create_view_with_join() {
        let sql = r#"
            CREATE VIEW analytics.orders_enriched AS
            SELECT o.id, o.amount, u.email AS customer_email
            FROM raw.orders o
            JOIN raw.users u ON o.user_id = u.id
        "#;
        let extracts = extract_lineage_from_sql(sql).unwrap();
        assert_eq!(extracts.len(), 1);
        let e = &extracts[0];
        assert_eq!(e.target.as_deref(), Some("analytics.orders_enriched"));
        assert!(e.sources.iter().any(|s| s.contains("orders")));
        assert!(e.sources.iter().any(|s| s.contains("users")));
        assert!(e
            .column_mappings
            .iter()
            .any(|m| m.target_column == "customer_email"));
    }

    #[test]
    fn parses_insert_select() {
        let sql = "INSERT INTO mart.fact_orders SELECT id, amount FROM staging.orders";
        let e = &extract_lineage_from_sql(sql).unwrap()[0];
        assert_eq!(e.target.as_deref(), Some("mart.fact_orders"));
        assert!(e.sources.iter().any(|s| s.ends_with("orders")));
    }
}
