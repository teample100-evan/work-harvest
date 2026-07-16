use jsonschema::{Retrieve, Uri, Validator};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::sync::OnceLock;

const COMMON_SCHEMA_URI: &str = "https://work-harvest.local/schemas/common.schema.json";
const COMMON_SCHEMA: &str = include_str!("../../../schemas/common.schema.json");
const WORK_ITEM_SCHEMA: &str = include_str!("../../../schemas/work-item.schema.json");
const WORK_CONTEXT_SCHEMA: &str = include_str!("../../../schemas/work-context.schema.json");
const CHECKPOINT_SCHEMA: &str = include_str!("../../../schemas/checkpoint.schema.json");

#[derive(Clone, Copy)]
pub(crate) enum DocumentKind {
    WorkItem,
    WorkContext,
    Checkpoint,
}

pub(crate) struct SchemaViolation {
    pub instance_path: String,
    pub message: String,
}

struct SchemaValidators {
    work_item: Validator,
    work_context: Validator,
    checkpoint: Validator,
}

#[derive(Clone)]
struct EmbeddedSchemas {
    schemas: HashMap<String, Value>,
}

impl Retrieve for EmbeddedSchemas {
    fn retrieve(&self, uri: &Uri<String>) -> Result<Value, Box<dyn Error + Send + Sync + 'static>> {
        self.schemas
            .get(uri.as_str())
            .cloned()
            .ok_or_else(|| format!("Embedded schema not found: {uri}").into())
    }
}

static VALIDATORS: OnceLock<Result<SchemaValidators, String>> = OnceLock::new();

fn parse_schema(label: &str, source: &str) -> Result<Value, String> {
    serde_json::from_str(source).map_err(|error| format!("Could not parse {label} schema: {error}"))
}

fn build_validators() -> Result<SchemaValidators, String> {
    let common = parse_schema("common", COMMON_SCHEMA)?;
    let work_item = parse_schema("work item", WORK_ITEM_SCHEMA)?;
    let work_context = parse_schema("work context", WORK_CONTEXT_SCHEMA)?;
    let checkpoint = parse_schema("checkpoint", CHECKPOINT_SCHEMA)?;
    let retriever = EmbeddedSchemas {
        schemas: HashMap::from([(COMMON_SCHEMA_URI.to_string(), common)]),
    };

    let build = |schema: &Value| {
        jsonschema::draft202012::options()
            .with_retriever(retriever.clone())
            .should_validate_formats(true)
            .build(schema)
            .map_err(|error| format!("Could not compile embedded schema: {error}"))
    };

    Ok(SchemaValidators {
        work_item: build(&work_item)?,
        work_context: build(&work_context)?,
        checkpoint: build(&checkpoint)?,
    })
}

fn validators() -> Result<&'static SchemaValidators, String> {
    VALIDATORS
        .get_or_init(build_validators)
        .as_ref()
        .map_err(Clone::clone)
}

pub(crate) fn validate(kind: DocumentKind, value: &Value) -> Result<Vec<SchemaViolation>, String> {
    let validators = validators()?;
    let validator = match kind {
        DocumentKind::WorkItem => &validators.work_item,
        DocumentKind::WorkContext => &validators.work_context,
        DocumentKind::Checkpoint => &validators.checkpoint,
    };

    Ok(validator
        .iter_errors(value)
        .map(|error| SchemaViolation {
            instance_path: error.instance_path().to_string(),
            message: error.to_string(),
        })
        .collect())
}
