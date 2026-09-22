//! tc-proof comparator entrypoint.

#![expect(
    clippy::print_stdout,
    reason = "tc-proof emits schema-versioned JSON results on stdout"
)]
#![expect(clippy::print_stderr, reason = "tc-proof emits diagnostics on stderr")]

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use refactor_proof::{canonical_json, compare, verifier};

const HELP: &str = "tc-proof compare --context PATH
tc-proof prepare --task-dir PATH --run-dir PATH --worktree PATH --scope-base COMMIT \
  --oracle-tag refs/tags/visual-baseline --oracle-commit COMMIT --tool PATH --comparator PATH \
  --native-build-receipt PATH --taskfmt PATH --taskfmt-source PATH \
  --taskfmt-revision COMMIT --taskfmt-version VERSION --taskfmt-sha256 SHA256 \
  --observer-provider PATH [--dependency-receipt PATH] [--run-id ID] \
  [--observer-nonce NONCE] [--observer-socket PATH]
tc-proof validate --run-dir PATH
tc-proof launch --run-dir PATH --check-id CHK-NNN [--timeout-ms N] -- PROGRAM [ARGS...]
";

fn usage() -> ! {
    eprint!("{HELP}\nSee docs/refactoring-plan/proof-contract.md.\n");
    std::process::exit(2);
}

fn parse_compare_args(args: &[String]) -> PathBuf {
    let mut context = None;
    let mut index = 0;
    while index < args.len() {
        let Some(option) = args.get(index).map(String::as_str) else {
            break;
        };
        match option {
            "--context" => {
                if context.is_some() {
                    eprintln!("tc-proof: duplicate option: --context");
                    usage();
                }
                index = index.saturating_add(1);
                context = Some(
                    args.get(index)
                        .filter(|value| !value.starts_with('-'))
                        .map_or_else(|| usage(), PathBuf::from),
                );
            }
            "--help" | "-h" => {
                print!("{HELP}");
                std::process::exit(0);
            }
            other if !other.starts_with('-') => {
                eprintln!("tc-proof: unexpected positional argument: {other}");
                usage();
            }
            other => {
                eprintln!("tc-proof: unknown option: {other}");
                usage();
            }
        }
        index = index.saturating_add(1);
    }
    context.unwrap_or_else(|| usage())
}

fn value_after(args: &[String], index: &mut usize, option: &str) -> Result<String, String> {
    *index = (*index).saturating_add(1);
    args.get(*index)
        .filter(|value| !value.starts_with('-'))
        .cloned()
        .ok_or_else(|| format!("{option} requires a value"))
}

fn set_once<T>(slot: &mut Option<T>, value: T, option: &str) -> Result<(), String> {
    if slot.is_some() {
        return Err(format!("duplicate option: {option}"));
    }
    *slot = Some(value);
    Ok(())
}

#[expect(
    clippy::too_many_lines,
    reason = "the CLI parser keeps every preparation binding explicit"
)]
fn parse_prepare_args(args: &[String]) -> Result<verifier::PrepareOptions, String> {
    let mut task_dir = None;
    let mut run_dir = None;
    let mut worktree = None;
    let mut scope_base = None;
    let mut oracle_tag = None;
    let mut oracle_commit = None;
    let mut tool = None;
    let mut comparator = None;
    let mut taskfmt = None;
    let mut native_build_receipt = None;
    let mut taskfmt_source = None;
    let mut taskfmt_revision = None;
    let mut taskfmt_version = None;
    let mut taskfmt_sha256 = None;
    let mut observer_provider = None;
    let mut dependency_receipts = Vec::new();
    let mut run_id = None;
    let mut observer_nonce = None;
    let mut observer_socket = None;
    let mut index = 0;
    while index < args.len() {
        let Some(option) = args.get(index).map(String::as_str) else {
            break;
        };
        match option {
            "--task-dir" => {
                set_once(
                    &mut task_dir,
                    PathBuf::from(value_after(args, &mut index, "--task-dir")?),
                    "--task-dir",
                )?;
            }
            "--run-dir" => {
                set_once(
                    &mut run_dir,
                    PathBuf::from(value_after(args, &mut index, "--run-dir")?),
                    "--run-dir",
                )?;
            }
            "--worktree" => {
                set_once(
                    &mut worktree,
                    PathBuf::from(value_after(args, &mut index, "--worktree")?),
                    "--worktree",
                )?;
            }
            "--scope-base" => set_once(
                &mut scope_base,
                value_after(args, &mut index, "--scope-base")?,
                "--scope-base",
            )?,
            "--oracle-tag" => set_once(
                &mut oracle_tag,
                value_after(args, &mut index, "--oracle-tag")?,
                "--oracle-tag",
            )?,
            "--oracle-commit" => {
                set_once(
                    &mut oracle_commit,
                    value_after(args, &mut index, "--oracle-commit")?,
                    "--oracle-commit",
                )?;
            }
            "--tool" => set_once(
                &mut tool,
                PathBuf::from(value_after(args, &mut index, "--tool")?),
                "--tool",
            )?,
            "--comparator" => {
                set_once(
                    &mut comparator,
                    PathBuf::from(value_after(args, &mut index, "--comparator")?),
                    "--comparator",
                )?;
            }
            "--taskfmt" => {
                set_once(
                    &mut taskfmt,
                    PathBuf::from(value_after(args, &mut index, "--taskfmt")?),
                    "--taskfmt",
                )?;
            }
            "--native-build-receipt" => {
                set_once(
                    &mut native_build_receipt,
                    PathBuf::from(value_after(args, &mut index, "--native-build-receipt")?),
                    "--native-build-receipt",
                )?;
            }
            "--taskfmt-source" => {
                set_once(
                    &mut taskfmt_source,
                    PathBuf::from(value_after(args, &mut index, "--taskfmt-source")?),
                    "--taskfmt-source",
                )?;
            }
            "--taskfmt-revision" => {
                set_once(
                    &mut taskfmt_revision,
                    value_after(args, &mut index, "--taskfmt-revision")?,
                    "--taskfmt-revision",
                )?;
            }
            "--taskfmt-version" => {
                set_once(
                    &mut taskfmt_version,
                    value_after(args, &mut index, "--taskfmt-version")?,
                    "--taskfmt-version",
                )?;
            }
            "--taskfmt-sha256" => {
                set_once(
                    &mut taskfmt_sha256,
                    value_after(args, &mut index, "--taskfmt-sha256")?,
                    "--taskfmt-sha256",
                )?;
            }
            "--observer-provider" => {
                set_once(
                    &mut observer_provider,
                    PathBuf::from(value_after(args, &mut index, "--observer-provider")?),
                    "--observer-provider",
                )?;
            }
            "--dependency-receipt" => dependency_receipts.push(PathBuf::from(value_after(
                args,
                &mut index,
                "--dependency-receipt",
            )?)),
            "--run-id" => set_once(
                &mut run_id,
                value_after(args, &mut index, "--run-id")?,
                "--run-id",
            )?,
            "--observer-nonce" => {
                set_once(
                    &mut observer_nonce,
                    value_after(args, &mut index, "--observer-nonce")?,
                    "--observer-nonce",
                )?;
            }
            "--observer-socket" => {
                set_once(
                    &mut observer_socket,
                    PathBuf::from(value_after(args, &mut index, "--observer-socket")?),
                    "--observer-socket",
                )?;
            }
            "--help" | "-h" => return Err(HELP.to_string()),
            option if !option.starts_with('-') => {
                return Err(format!("unexpected positional argument: {option}"));
            }
            option => return Err(format!("unknown prepare option: {option}")),
        }
        index = index.saturating_add(1);
    }
    Ok(verifier::PrepareOptions {
        task_dir: task_dir.ok_or_else(|| "missing --task-dir".to_string())?,
        run_dir: run_dir.ok_or_else(|| "missing --run-dir".to_string())?,
        worktree: worktree.ok_or_else(|| "missing --worktree".to_string())?,
        scope_base: scope_base.ok_or_else(|| "missing --scope-base".to_string())?,
        oracle_tag: oracle_tag.ok_or_else(|| "missing --oracle-tag".to_string())?,
        oracle_commit: oracle_commit.ok_or_else(|| "missing --oracle-commit".to_string())?,
        tool: tool.ok_or_else(|| "missing --tool".to_string())?,
        comparator: comparator.ok_or_else(|| "missing --comparator".to_string())?,
        taskfmt,
        native_build_receipt: native_build_receipt
            .ok_or_else(|| "missing --native-build-receipt".to_string())?,
        taskfmt_source: taskfmt_source.ok_or_else(|| "missing --taskfmt-source".to_string())?,
        taskfmt_revision: taskfmt_revision
            .ok_or_else(|| "missing --taskfmt-revision".to_string())?,
        taskfmt_version: taskfmt_version.ok_or_else(|| "missing --taskfmt-version".to_string())?,
        taskfmt_sha256: taskfmt_sha256.ok_or_else(|| "missing --taskfmt-sha256".to_string())?,
        observer_provider: observer_provider
            .ok_or_else(|| "missing --observer-provider".to_string())?,
        dependency_receipts,
        run_id,
        observer_nonce,
        observer_socket,
    })
}

fn parse_launch_args(args: &[String]) -> Result<verifier::LaunchOptions, String> {
    let separator = args
        .iter()
        .position(|arg| arg == "--")
        .ok_or_else(|| "launch requires -- before the child program".to_string())?;
    let mut run_dir = None;
    let mut check_id = None;
    let mut observer_socket = None;
    let mut timeout_ms = None;
    let prefix = args
        .get(..separator)
        .ok_or_else(|| "invalid launch option range".to_string())?;
    let mut index = 0;
    while index < separator {
        let Some(option) = prefix.get(index).map(String::as_str) else {
            break;
        };
        match option {
            "--run-dir" => {
                set_once(
                    &mut run_dir,
                    PathBuf::from(value_after(prefix, &mut index, "--run-dir")?),
                    "--run-dir",
                )?;
            }
            "--check-id" => set_once(
                &mut check_id,
                value_after(prefix, &mut index, "--check-id")?,
                "--check-id",
            )?,
            "--observer-socket" => {
                set_once(
                    &mut observer_socket,
                    PathBuf::from(value_after(prefix, &mut index, "--observer-socket")?),
                    "--observer-socket",
                )?;
            }
            "--timeout-ms" => {
                let parsed = value_after(prefix, &mut index, "--timeout-ms")?
                    .parse()
                    .map_err(|_| "invalid --timeout-ms".to_string())?;
                set_once(&mut timeout_ms, parsed, "--timeout-ms")?;
            }
            "--help" | "-h" => return Err(HELP.to_string()),
            option if !option.starts_with('-') => {
                return Err(format!("unexpected positional argument: {option}"));
            }
            option => return Err(format!("unknown launch option: {option}")),
        }
        index = index.saturating_add(1);
    }
    let program = args
        .get(separator.saturating_add(1))
        .ok_or_else(|| "launch child program is missing".to_string())?
        .clone();
    Ok(verifier::LaunchOptions {
        run_dir: run_dir.ok_or_else(|| "missing --run-dir".to_string())?,
        check_id: check_id.ok_or_else(|| "missing --check-id".to_string())?,
        observer_socket,
        timeout: Duration::from_millis(timeout_ms.unwrap_or(120_000)),
        program: PathBuf::from(program),
        args: args
            .get(separator.saturating_add(2)..)
            .ok_or_else(|| "invalid launch child argument range".to_string())?
            .to_vec(),
    })
}

fn parse_validate_args(args: &[String]) -> Result<PathBuf, String> {
    let mut run_dir = None;
    let mut index = 0;
    while index < args.len() {
        let Some(option) = args.get(index).map(String::as_str) else {
            break;
        };
        match option {
            "--run-dir" => {
                set_once(
                    &mut run_dir,
                    PathBuf::from(value_after(args, &mut index, "--run-dir")?),
                    "--run-dir",
                )?;
            }
            "--help" | "-h" => return Err(HELP.to_string()),
            option if !option.starts_with('-') => {
                return Err(format!("unexpected positional argument: {option}"));
            }
            option => return Err(format!("unknown validate option: {option}")),
        }
        index = index.saturating_add(1);
    }
    run_dir.ok_or_else(|| "missing --run-dir".to_string())
}

fn print_json<T: serde::Serialize>(value: &T) -> Result<(), String> {
    let value = serde_json::to_value(value).map_err(|error| error.to_string())?;
    println!("{}", canonical_json(&value));
    Ok(())
}

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty()
        || args
            .first()
            .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        print!("{HELP}");
        return ExitCode::SUCCESS;
    }
    match args.remove(0).as_str() {
        "compare" => {
            let code = compare::run_compare(&parse_compare_args(&args)).unwrap_or(2);
            ExitCode::from(code as u8)
        }
        "prepare" => match parse_prepare_args(&args)
            .and_then(|options| verifier::prepare(&options).map_err(|error| error.to_string()))
            .and_then(|run| print_json(&run))
        {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("tc-proof prepare: {error}");
                ExitCode::from(1)
            }
        },
        "validate" => match parse_validate_args(&args)
            .and_then(|run_dir| verifier::validate_run(&run_dir).map_err(|error| error.to_string()))
            .and_then(|run| print_json(&run))
        {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("tc-proof validate: {error}");
                ExitCode::from(1)
            }
        },
        "launch" => match parse_launch_args(&args)
            .and_then(|options| verifier::launch(&options).map_err(|error| error.to_string()))
            .and_then(|record| print_json(&record))
        {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("tc-proof launch: {error}");
                ExitCode::from(1)
            }
        },
        other => {
            eprintln!("tc-proof: unknown command: {other}");
            usage();
        }
    }
}
