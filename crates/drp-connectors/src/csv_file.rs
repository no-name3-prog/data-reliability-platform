//! CSV file connector — discover files and sample rows from disk paths / directories.

use std::fs::File;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use indexmap::IndexMap;
use serde_json::{json, Value};
use tracing::info;

use drp_common::{AssetKind, DataType, Error, Result, SourceLocation};
use drp_core::{
    Asset, CatalogDatabase, CatalogSchema, CatalogTable, CatalogTree, ColumnMeta, ConnectorPlugin,
    Plugin, PluginCapability, PluginContext, PluginInfo,
};

type SampleRows = Vec<IndexMap<String, Value>>;
type SchemaSample = (Vec<ColumnMeta>, SampleRows);

/// CSV connector (`id = "csv"`).
///
/// `location.uri` is a file path or directory of `*.csv` files.
pub struct CsvConnector {
    info: PluginInfo,
}

impl CsvConnector {
    /// Create the connector.
    pub fn new() -> Self {
        Self {
            info: PluginInfo::new("csv", "CSV File Connector", env!("CARGO_PKG_VERSION"))
                .with_description("Discover CSV files and sample rows from local paths")
                .with_capability(PluginCapability::Connector),
        }
    }

    fn resolve_paths(location: &SourceLocation) -> Result<Vec<PathBuf>> {
        let path = PathBuf::from(&location.uri);
        if !path.exists() {
            return Err(Error::connector(format!(
                "csv path does not exist: {}",
                path.display()
            )));
        }
        if path.is_file() {
            return Ok(vec![path]);
        }
        let mut files = Vec::new();
        for entry in std::fs::read_dir(&path)
            .map_err(|e| Error::connector(format!("read dir {}: {e}", path.display())))?
        {
            let entry = entry.map_err(|e| Error::connector(e.to_string()))?;
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("csv") {
                files.push(p);
            }
        }
        files.sort();
        Ok(files)
    }

    fn infer_columns(path: &Path) -> Result<SchemaSample> {
        let file = File::open(path)
            .map_err(|e| Error::connector(format!("open {}: {e}", path.display())))?;
        let mut rdr = csv::ReaderBuilder::new().flexible(true).from_reader(file);
        let headers: Vec<String> = rdr
            .headers()
            .map_err(|e| Error::connector(format!("csv headers: {e}")))?
            .iter()
            .map(|s| s.to_string())
            .collect();

        let mut rows = Vec::new();
        for rec in rdr.records().take(200) {
            let rec = rec.map_err(|e| Error::connector(format!("csv record: {e}")))?;
            let mut map = IndexMap::new();
            for (i, h) in headers.iter().enumerate() {
                let raw = rec.get(i).unwrap_or("");
                map.insert(h.clone(), cell_to_json(raw));
            }
            rows.push(map);
        }

        let columns = headers
            .iter()
            .enumerate()
            .map(|(i, h)| {
                let dtype = infer_col_type(&rows, h);
                ColumnMeta::new(h, dtype).at(i as u32)
            })
            .collect();
        Ok((columns, rows))
    }
}

impl Default for CsvConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for CsvConnector {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}

#[async_trait]
impl ConnectorPlugin for CsvConnector {
    async fn test_connection(&self, location: &SourceLocation, _ctx: &PluginContext) -> Result<()> {
        let paths = Self::resolve_paths(location)?;
        if paths.is_empty() {
            return Err(Error::connector("csv: no .csv files found at location"));
        }
        Ok(())
    }

    async fn discover(&self, location: &SourceLocation, ctx: &PluginContext) -> Result<Vec<Asset>> {
        let tree = self.discover_catalog(location, ctx).await?;
        Ok(tree
            .all_tables()
            .into_iter()
            .map(|t| {
                Asset::new(t.fqn.clone(), t.name.clone(), t.kind, location.clone())
                    .with_columns(t.columns.clone())
                    .with_tag(
                        "path",
                        t.properties.get("path").cloned().unwrap_or_default(),
                    )
                    .with_tag("format", "csv")
            })
            .collect())
    }

    async fn discover_catalog(
        &self,
        location: &SourceLocation,
        _ctx: &PluginContext,
    ) -> Result<CatalogTree> {
        let paths = Self::resolve_paths(location)?;
        let mut tree = CatalogTree::new(self.info.id.clone(), location.clone());
        let mut db = CatalogDatabase::new("files");
        let mut schema = CatalogSchema::new("csv");

        for path in paths {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("table")
                .to_string();
            let (columns, sample) = Self::infer_columns(&path)?;
            let mut table = CatalogTable::file(path.display().to_string())
                .with_columns(columns)
                .with_property("path", path.display().to_string())
                .with_property("format", "csv");
            table.name = name;
            table.kind = AssetKind::File;
            table.fqn = format!("files.csv.{}", table.name);
            table.row_count_estimate = Some(sample.len() as u64);
            info!(path = %path.display(), columns = table.columns.len(), "csv table discovered");
            schema.tables.push(table);
        }
        db.schemas.push(schema);
        tree.databases.push(db);
        Ok(tree)
    }

    async fn sample_rows(
        &self,
        asset: &Asset,
        limit: usize,
        _ctx: &PluginContext,
    ) -> Result<Vec<IndexMap<String, Value>>> {
        let path = asset
            .tags
            .get("path")
            .cloned()
            .or_else(|| asset.location.properties.get("path").cloned())
            .unwrap_or_else(|| asset.location.uri.clone());
        let path = if Path::new(&path).is_file() {
            PathBuf::from(path)
        } else {
            // uri may be directory — use fqn/name
            let dir = PathBuf::from(&asset.location.uri);
            dir.join(format!("{}.csv", asset.name))
        };
        let (_cols, mut rows) = Self::infer_columns(&path)?;
        rows.truncate(limit);
        Ok(rows)
    }
}

fn cell_to_json(raw: &str) -> Value {
    if raw.is_empty() {
        return Value::Null;
    }
    if let Ok(b) = raw.parse::<bool>() {
        return json!(b);
    }
    if let Ok(i) = raw.parse::<i64>() {
        return json!(i);
    }
    if let Ok(f) = raw.parse::<f64>() {
        return json!(f);
    }
    json!(raw)
}

fn infer_col_type(rows: &[IndexMap<String, Value>], col: &str) -> DataType {
    for row in rows {
        match row.get(col) {
            Some(Value::Bool(_)) => return DataType::Boolean,
            Some(Value::Number(n)) if n.is_i64() || n.is_u64() => return DataType::Integer,
            Some(Value::Number(_)) => return DataType::Float,
            Some(Value::String(_)) => return DataType::String,
            _ => {}
        }
    }
    DataType::Unknown
}
