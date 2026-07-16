use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Serialize, de::DeserializeOwned};
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Read};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use work_harvest_core::{
    CheckpointInput, CheckpointWriteError, DataRootCounts, IssueSeverity, PerformanceNoteInput,
    PerformanceNoteWriteError, QueryError, StoredCheckpointRecord, StoredWorkItemRecord,
    WorkContextDocument, WorkItemCreateInput, WorkItemDocument, WorkItemPaths, WorkItemWriteError,
    WriteError, capture_checkpoint, create_performance_note_from_current, create_work_item,
    find_last_checkpoint, inspect_data_root, list_work_item_records, read_work_item_for_edit,
    show_work_item,
};

const USAGE: &str = "Work Harvest CLI

Usage:
  wh work-item create --input <file|-> [--root <path>] [--json]
  wh work-item list [--project <id>] [--status <status>] [--root <path>] [--json]
  wh work-item show <id> [--root <path>] [--json]
  wh checkpoint capture --input <file|-> [--root <path>] [--json]
  wh checkpoint last --work-item <id> [--root <path>] [--json]
  wh report performance-note --work-item <id> [--output <path>] [--root <path>] [--json]
  wh validate [--root <path>] [--include-examples] [--json]

Environment:
  WORK_HARVEST_HOME  Default data root when --root is omitted
";

#[derive(Debug, Clone, Serialize)]
struct ErrorDetail {
    file: String,
    message: String,
}

#[derive(Debug)]
struct CliError {
    message: String,
    exit_code: i32,
    details: Vec<ErrorDetail>,
}

impl CliError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            exit_code: 1,
            details: Vec::new(),
        }
    }

    fn usage(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            exit_code: 2,
            details: Vec::new(),
        }
    }

    fn with_details(message: impl Into<String>, details: Vec<ErrorDetail>) -> Self {
        Self {
            message: message.into(),
            exit_code: 1,
            details,
        }
    }
}

#[derive(Debug, Default)]
struct CommandOptions {
    root: Option<String>,
    json: bool,
    help: bool,
    input: Option<String>,
    include_examples: bool,
    project: Option<String>,
    status: Option<String>,
    work_item: Option<String>,
    output: Option<String>,
    positionals: Vec<String>,
}

fn is_boolean_option(name: &str) -> bool {
    matches!(name, "json" | "help" | "include-examples")
}

fn option_allowed(name: &str, allowed: &[&str]) -> bool {
    matches!(name, "root" | "json" | "help") || allowed.contains(&name)
}

fn set_option(options: &mut CommandOptions, name: &str, value: Option<String>) {
    match name {
        "root" => options.root = value,
        "json" => options.json = true,
        "help" => options.help = true,
        "input" => options.input = value,
        "include-examples" => options.include_examples = true,
        "project" => options.project = value,
        "status" => options.status = value,
        "work-item" => options.work_item = value,
        "output" => options.output = value,
        _ => unreachable!("validated option"),
    }
}

fn parse_options(
    args: &[String],
    allowed: &[&str],
    allow_positionals: bool,
) -> Result<CommandOptions, CliError> {
    let mut options = CommandOptions::default();
    let mut index = 0;
    while index < args.len() {
        let argument = &args[index];
        if argument == "--" {
            let remaining = &args[index + 1..];
            if !allow_positionals && !remaining.is_empty() {
                return Err(CliError::usage(format!(
                    "Unexpected argument: {}",
                    remaining[0]
                )));
            }
            options.positionals.extend(remaining.iter().cloned());
            break;
        }
        if argument == "-h" {
            options.help = true;
            index += 1;
            continue;
        }
        if argument == "-i" {
            if !allowed.contains(&"input") {
                return Err(CliError::usage("Unknown option: -i"));
            }
            index += 1;
            let value = args
                .get(index)
                .cloned()
                .ok_or_else(|| CliError::usage("Option -i requires a value"))?;
            options.input = Some(value);
            index += 1;
            continue;
        }
        if let Some(long) = argument.strip_prefix("--") {
            let (name, inline_value) =
                long.split_once('=').map_or((long, None), |(name, value)| {
                    (name, Some(value.to_string()))
                });
            if !option_allowed(name, allowed) {
                return Err(CliError::usage(format!("Unknown option: --{name}")));
            }
            if is_boolean_option(name) {
                if inline_value.is_some() {
                    return Err(CliError::usage(format!(
                        "Option --{name} does not take a value"
                    )));
                }
                set_option(&mut options, name, None);
                index += 1;
                continue;
            }
            let value = if let Some(value) = inline_value {
                value
            } else {
                index += 1;
                args.get(index)
                    .cloned()
                    .ok_or_else(|| CliError::usage(format!("Option --{name} requires a value")))?
            };
            set_option(&mut options, name, Some(value));
            index += 1;
            continue;
        }
        if argument.starts_with('-') {
            return Err(CliError::usage(format!("Unknown option: {argument}")));
        }
        if !allow_positionals {
            return Err(CliError::usage(format!("Unexpected argument: {argument}")));
        }
        options.positionals.push(argument.clone());
        index += 1;
    }
    Ok(options)
}

fn resolve_data_root(value: Option<&str>) -> Result<PathBuf, CliError> {
    let selected = value
        .map(PathBuf::from)
        .or_else(|| env::var_os("WORK_HARVEST_HOME").map(PathBuf::from));
    let path = match selected {
        Some(path) if path.is_absolute() => path,
        Some(path) => env::current_dir()
            .map_err(|error| {
                CliError::new(format!("Could not resolve current directory: {error}"))
            })?
            .join(path),
        None => env::current_dir().map_err(|error| {
            CliError::new(format!("Could not resolve current directory: {error}"))
        })?,
    };
    Ok(path)
}

fn read_structured_input<T: DeserializeOwned>(input: Option<&str>) -> Result<T, CliError> {
    if input.is_none() && io::stdin().is_terminal() {
        return Err(CliError::usage(
            "--input <file|-> is required when stdin is a TTY",
        ));
    }
    let mut text = String::new();
    if input.is_none() || input == Some("-") {
        io::stdin()
            .read_to_string(&mut text)
            .map_err(|error| CliError::usage(format!("Could not read input: {error}")))?;
    } else if let Some(path) = input {
        text = fs::read_to_string(path)
            .map_err(|error| CliError::usage(format!("Could not read input {path}: {error}")))?;
    }
    if text.trim().is_empty() {
        return Err(CliError::usage("Input is empty"));
    }
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text)
        .map_err(|error| CliError::usage(format!("Could not parse JSON/YAML input: {error}")))?;
    if !matches!(value, serde_yaml_ng::Value::Mapping(_)) {
        return Err(CliError::usage(
            "Could not parse JSON/YAML input: Expected an object",
        ));
    }
    serde_yaml_ng::from_value(value)
        .map_err(|error| CliError::usage(format!("Could not parse JSON/YAML input: {error}")))
}

fn now_rfc3339() -> String {
    let now: DateTime<Utc> = SystemTime::now().into();
    now.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn print_json<T: Serialize>(value: &T) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value)
        .map_err(|error| CliError::new(format!("Could not serialize CLI output: {error}")))?;
    use std::io::Write;
    writeln!(lock).map_err(|error| CliError::new(format!("Could not write CLI output: {error}")))
}

fn print_result<T: Serialize>(value: &T, as_json: bool, message: String) -> Result<(), CliError> {
    if as_json {
        print_json(value)
    } else {
        println!("{message}");
        Ok(())
    }
}

#[derive(Serialize)]
struct WorkItemCreateOutput {
    work_item: WorkItemDocument,
    context: WorkContextDocument,
    paths: WorkItemPaths,
}

#[derive(Serialize)]
struct CheckpointCaptureOutput {
    checkpoint: work_harvest_core::CheckpointDocument,
    work_item: WorkItemDocument,
    context: WorkContextDocument,
    paths: work_harvest_core::CheckpointPaths,
}

#[derive(Serialize)]
struct PerformanceNoteOutput {
    work_item: WorkItemDocument,
    checkpoint_count: usize,
    redacted_checkpoint_count: usize,
    excluded_checkpoint_count: usize,
    paths: work_harvest_core::PerformanceNotePaths,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct WorkItemListEntry {
    id: String,
    project_id: String,
    title: String,
    status: String,
    initiative_id: Option<String>,
    updated_at: String,
    current_state: String,
    next_steps: Vec<String>,
    last_checkpoint_id: Option<String>,
}

#[derive(Serialize)]
struct WorkItemListOutput {
    work_items: Vec<WorkItemListEntry>,
}

impl From<StoredWorkItemRecord> for WorkItemListEntry {
    fn from(record: StoredWorkItemRecord) -> Self {
        Self {
            id: record.work_item.id,
            project_id: record.work_item.project_id,
            title: record.work_item.title,
            status: record.work_item.status,
            initiative_id: record.work_item.classification.initiative_id,
            updated_at: record.work_item.updated_at,
            current_state: record.context.current_state,
            next_steps: record.context.next_steps,
            last_checkpoint_id: record.context.last_checkpoint_id,
        }
    }
}

#[derive(Serialize)]
struct LastCheckpointOutput {
    checkpoint: Option<work_harvest_core::CheckpointDocument>,
    paths: Option<work_harvest_core::StoredCheckpointPaths>,
}

#[derive(Serialize)]
struct ValidationDataset {
    label: String,
    counts: DataRootCounts,
}

#[derive(Serialize)]
struct ValidationOutput {
    valid: bool,
    datasets: Vec<ValidationDataset>,
}

fn map_work_item_error(error: WorkItemWriteError, id: Option<&str>) -> CliError {
    match error {
        WorkItemWriteError::Validation { details, .. } => CliError::with_details(
            "Work item validation failed",
            vec![ErrorDetail {
                file: "work-item.json".to_string(),
                message: details,
            }],
        ),
        WorkItemWriteError::Write(WriteError::CreateConflict(_)) => CliError::new(format!(
            "Work item already exists: {}",
            id.unwrap_or("unknown")
        )),
        other => CliError::new(other.to_string()),
    }
}

fn map_checkpoint_error(error: CheckpointWriteError) -> CliError {
    match error {
        CheckpointWriteError::Validation { details, .. } => CliError::with_details(
            "Checkpoint validation failed",
            vec![ErrorDetail {
                file: "checkpoint.json".to_string(),
                message: details,
            }],
        ),
        other => CliError::new(other.to_string()),
    }
}

fn map_report_error(error: PerformanceNoteWriteError) -> CliError {
    match error {
        PerformanceNoteWriteError::Write(WriteError::CreateConflict(path)) => {
            CliError::new(format!("Performance note already exists: {path}"))
        }
        other => CliError::new(other.to_string()),
    }
}

fn map_query_error(error: QueryError) -> CliError {
    match error {
        QueryError::WorkItemNotFound(id) => CliError::new(format!("Unknown work item: {id}")),
        other => CliError::new(other.to_string()),
    }
}

fn handle_create_work_item(args: &[String]) -> Result<(), CliError> {
    let options = parse_options(args, &["input"], false)?;
    if options.help {
        print!("{USAGE}");
        return Ok(());
    }
    let root = resolve_data_root(options.root.as_deref())?;
    let input: WorkItemCreateInput = read_structured_input(options.input.as_deref())?;
    let work_item_id = input.id.clone();
    let result = create_work_item(&root, input, &now_rfc3339())
        .map_err(|error| map_work_item_error(error, Some(&work_item_id)))?;
    let output = WorkItemCreateOutput {
        work_item: result.work_item,
        context: result.context,
        paths: result.paths,
    };
    print_result(
        &output,
        options.json,
        format!(
            "Created work item {}\n  metadata: {}\n  context data: {}\n  context: {}",
            output.work_item.id,
            output.paths.work_item,
            output.paths.context_data,
            output.paths.context
        ),
    )
}

fn handle_list_work_items(args: &[String]) -> Result<(), CliError> {
    let options = parse_options(args, &["project", "status"], false)?;
    if options.help {
        print!("{USAGE}");
        return Ok(());
    }
    let root = resolve_data_root(options.root.as_deref())?;
    let records = if root.exists() {
        list_work_item_records(&root).map_err(map_query_error)?
    } else {
        Vec::new()
    };
    let work_items = records
        .into_iter()
        .map(WorkItemListEntry::from)
        .filter(|item| {
            options
                .project
                .as_ref()
                .is_none_or(|project| item.project_id == *project)
                && options
                    .status
                    .as_ref()
                    .is_none_or(|status| item.status == *status)
        })
        .collect::<Vec<_>>();
    let message = if work_items.is_empty() {
        "No work items found".to_string()
    } else {
        work_items
            .iter()
            .map(|item| {
                format!(
                    "{}\t{}\t{}\t{}\n  {}",
                    item.id, item.status, item.project_id, item.title, item.current_state
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    print_result(&WorkItemListOutput { work_items }, options.json, message)
}

fn handle_show_work_item(args: &[String]) -> Result<(), CliError> {
    let options = parse_options(args, &[], true)?;
    if options.help {
        print!("{USAGE}");
        return Ok(());
    }
    if options.positionals.len() != 1 {
        return Err(CliError::usage(
            "work-item show requires exactly one work item id",
        ));
    }
    let root = resolve_data_root(options.root.as_deref())?;
    let result = show_work_item(&root, &options.positionals[0]).map_err(map_query_error)?;
    let last = result
        .last_checkpoint
        .as_ref()
        .map(|entry| &entry.checkpoint);
    let message = format!(
        "{} {}\n  status: {}\n  current: {}\n  last checkpoint: {}",
        result.work_item.id,
        result.work_item.title,
        result.work_item.status,
        result.context.current_state,
        last.map_or_else(
            || "none".to_string(),
            |checkpoint| format!("{} ({})", checkpoint.id, checkpoint.captured_at)
        )
    );
    print_result(&result, options.json, message)
}

fn handle_capture_checkpoint(args: &[String]) -> Result<(), CliError> {
    let options = parse_options(args, &["input"], false)?;
    if options.help {
        print!("{USAGE}");
        return Ok(());
    }
    let root = resolve_data_root(options.root.as_deref())?;
    let input: CheckpointInput = read_structured_input(options.input.as_deref())?;
    let expected = read_work_item_for_edit(&root, &input.work_item_id)
        .map_err(|error| map_work_item_error(error, Some(&input.work_item_id)))?
        .revisions;
    let result =
        capture_checkpoint(&root, input, expected, &now_rfc3339()).map_err(map_checkpoint_error)?;
    let output = CheckpointCaptureOutput {
        checkpoint: result.checkpoint,
        work_item: result.work_item,
        context: result.context,
        paths: result.paths,
    };
    print_result(
        &output,
        options.json,
        format!(
            "Captured checkpoint {}\n  record: {}\n  Markdown: {}\n  work item: {}",
            output.checkpoint.id,
            output.paths.checkpoint,
            output.paths.checkpoint_markdown,
            output.work_item.status
        ),
    )
}

fn handle_last_checkpoint(args: &[String]) -> Result<(), CliError> {
    let options = parse_options(args, &["work-item"], false)?;
    if options.help {
        print!("{USAGE}");
        return Ok(());
    }
    let work_item_id = options
        .work_item
        .as_deref()
        .ok_or_else(|| CliError::usage("checkpoint last requires --work-item <id>"))?;
    let root = resolve_data_root(options.root.as_deref())?;
    let result = find_last_checkpoint(&root, work_item_id).map_err(map_query_error)?;
    let message = result.as_ref().map_or_else(
        || format!("No checkpoint found for {work_item_id}"),
        |entry| {
            format!(
                "{}\t{}\t{}",
                entry.checkpoint.id, entry.checkpoint.captured_at, entry.checkpoint.title
            )
        },
    );
    let output = match result {
        Some(StoredCheckpointRecord { checkpoint, paths }) => LastCheckpointOutput {
            checkpoint: Some(checkpoint),
            paths: Some(paths),
        },
        None => LastCheckpointOutput {
            checkpoint: None,
            paths: None,
        },
    };
    print_result(&output, options.json, message)
}

fn handle_performance_note(args: &[String]) -> Result<(), CliError> {
    let options = parse_options(args, &["work-item", "output"], false)?;
    if options.help {
        print!("{USAGE}");
        return Ok(());
    }
    let work_item_id = options
        .work_item
        .as_deref()
        .ok_or_else(|| CliError::usage("report performance-note requires --work-item <id>"))?;
    if let Some(output) = options.output.as_deref()
        && Path::new(output).extension() != Some(std::ffi::OsStr::new("md"))
    {
        return Err(CliError::usage("Report output must be a .md file"));
    }
    let root = resolve_data_root(options.root.as_deref())?;
    let result = create_performance_note_from_current(
        &root,
        PerformanceNoteInput {
            work_item_id: work_item_id.to_string(),
            output: options.output,
            markdown: None,
        },
        &now_rfc3339(),
    )
    .map_err(map_report_error)?;
    let output = PerformanceNoteOutput {
        work_item: result.work_item,
        checkpoint_count: result.checkpoint_count,
        redacted_checkpoint_count: result.redacted_checkpoint_count,
        excluded_checkpoint_count: result.excluded_checkpoint_count,
        paths: result.paths,
    };
    print_result(
        &output,
        options.json,
        format!(
            "Created performance note for {}\n  checkpoints: {} included · {} redacted · {} excluded\n  report: {}",
            output.work_item.id,
            output.checkpoint_count,
            output.redacted_checkpoint_count,
            output.excluded_checkpoint_count,
            output.paths.report
        ),
    )
}

fn validation_dataset(
    root: &Path,
    label: &str,
) -> Result<(ValidationDataset, Vec<ErrorDetail>), CliError> {
    if !root.exists() {
        return Ok((
            ValidationDataset {
                label: label.to_string(),
                counts: DataRootCounts {
                    work_items: 0,
                    contexts: 0,
                    checkpoints: 0,
                },
            },
            Vec::new(),
        ));
    }
    let snapshot = inspect_data_root(root).map_err(|error| CliError::new(error.to_string()))?;
    let details = snapshot
        .issues
        .into_iter()
        .filter(|issue| issue.severity == IssueSeverity::Error)
        .map(|issue| ErrorDetail {
            file: issue.path,
            message: issue.message,
        })
        .collect();
    Ok((
        ValidationDataset {
            label: label.to_string(),
            counts: snapshot.counts,
        },
        details,
    ))
}

fn handle_validate(args: &[String]) -> Result<(), CliError> {
    let options = parse_options(args, &["include-examples"], false)?;
    if options.help {
        print!("{USAGE}");
        return Ok(());
    }
    let root = resolve_data_root(options.root.as_deref())?;
    let (root_dataset, mut details) = validation_dataset(&root, "root")?;
    let mut datasets = vec![root_dataset];
    if options.include_examples {
        let examples = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("examples");
        let (examples_dataset, example_details) = validation_dataset(&examples, "examples")?;
        datasets.push(examples_dataset);
        details.extend(example_details);
    }
    if !details.is_empty() {
        return Err(CliError::with_details("Validation failed", details));
    }
    let counts = datasets.iter().fold(
        DataRootCounts {
            work_items: 0,
            contexts: 0,
            checkpoints: 0,
        },
        |mut total, dataset| {
            total.work_items += dataset.counts.work_items;
            total.contexts += dataset.counts.contexts;
            total.checkpoints += dataset.counts.checkpoints;
            total
        },
    );
    print_result(
        &ValidationOutput {
            valid: true,
            datasets,
        },
        options.json,
        format!(
            "Valid: {} work item(s), {} context(s), {} checkpoint(s)",
            counts.work_items, counts.contexts, counts.checkpoints
        ),
    )
}

fn execute(args: &[String]) -> Result<(), CliError> {
    if args.is_empty() || matches!(args[0].as_str(), "--help" | "-h") {
        print!("{USAGE}");
        return Ok(());
    }
    match (args[0].as_str(), args.get(1).map(String::as_str)) {
        ("validate", _) => handle_validate(&args[1..]),
        ("work-item", Some("create")) => handle_create_work_item(&args[2..]),
        ("work-item", Some("list")) => handle_list_work_items(&args[2..]),
        ("work-item", Some("show")) => handle_show_work_item(&args[2..]),
        ("checkpoint", Some("capture")) => handle_capture_checkpoint(&args[2..]),
        ("checkpoint", Some("last")) => handle_last_checkpoint(&args[2..]),
        ("report", Some("performance-note")) => handle_performance_note(&args[2..]),
        _ => Err(CliError::usage(format!(
            "Unknown command: {}",
            args.join(" ")
        ))),
    }
}

fn print_error(error: &CliError, as_json: bool) {
    if as_json {
        #[derive(Serialize)]
        struct JsonError<'a> {
            error: &'a str,
            details: &'a [ErrorDetail],
        }
        use std::io::Write;
        let stderr = io::stderr();
        let mut lock = stderr.lock();
        let result = serde_json::to_writer_pretty(
            &mut lock,
            &JsonError {
                error: &error.message,
                details: &error.details,
            },
        )
        .and_then(|()| writeln!(lock).map_err(serde_json::Error::io));
        if let Err(serialization_error) = result {
            eprintln!("Error: Could not serialize CLI error: {serialization_error}");
        }
    } else {
        eprintln!("Error: {}", error.message);
        for detail in &error.details {
            eprintln!("  - {}: {}", detail.file, detail.message);
        }
    }
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let as_json = args.iter().any(|argument| argument == "--json");
    if let Err(error) = execute(&args) {
        print_error(&error, as_json);
        if error.exit_code == 2 {
            eprint!("{USAGE}");
        }
        std::process::exit(error.exit_code);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_inline_values_and_rejects_unknown_options() {
        let parsed = parse_options(
            &["--root=/tmp/work".to_string(), "--json".to_string()],
            &[],
            false,
        )
        .unwrap();
        assert_eq!(parsed.root.as_deref(), Some("/tmp/work"));
        assert!(parsed.json);
        assert!(parse_options(&["--wat".to_string()], &[], false).is_err());
    }
}
