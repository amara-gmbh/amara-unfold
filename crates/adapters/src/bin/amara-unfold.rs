// SPDX-License-Identifier: AGPL-3.0-or-later

use amara_unfold_adapters::{build_timeline, extract, ExtractionRequest, EXECUTABLE_ADAPTERS};
use std::{path::PathBuf, sync::atomic::AtomicBool};

fn main() {
    if let Err(error) = run() {
        eprintln!("amara-unfold: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments.first().map(String::as_str) == Some("adapters") {
        for adapter in EXECUTABLE_ADAPTERS {
            println!("{adapter}");
        }
        return Ok(());
    }
    if arguments.first().map(String::as_str) == Some("validate") {
        let folder = arguments.get(1).ok_or("missing package folder")?;
        let count = validate_package(&PathBuf::from(folder))?;
        println!("valid open Markdown events: {count}");
        return Ok(());
    }
    if arguments.first().map(String::as_str) == Some("timeline") {
        let folder = arguments.get(1).ok_or("missing export folder")?;
        let result = build_timeline(&PathBuf::from(folder), &mut |seen, total, message| {
            eprintln!("[timeline] {seen}/{total} · {message}");
        })
        .map_err(|error| error.to_string())?;
        println!("{}", result.folder);
        return Ok(());
    }
    if arguments.first().map(String::as_str) != Some("extract") {
        return Err("usage: amara-unfold extract <adapter-id> --source <path> --output <folder>\n       amara-unfold timeline <export-folder>\n       amara-unfold validate <package-folder>\n       amara-unfold adapters".into());
    }
    let adapter_id = arguments.get(1).ok_or("missing adapter id")?.clone();
    if !EXECUTABLE_ADAPTERS.contains(&adapter_id.as_str()) {
        return Err(format!("unknown adapter: {adapter_id}"));
    }
    let source = argument_value(&arguments, "--source")?;
    let destination = argument_value(&arguments, "--output")?;
    let mut last_reported = 0;
    let result = extract(
        ExtractionRequest {
            adapter_id,
            source: PathBuf::from(source),
            destination: PathBuf::from(destination),
        },
        &AtomicBool::new(false),
        &mut |progress| {
            if progress.records_seen == 0
                || progress.records_seen >= last_reported + 100
                || progress.phase != "extracting"
            {
                last_reported = progress.records_seen;
                eprintln!(
                    "[{}] {} records · {} written · {} not written (see reasons) · {} errors",
                    progress.adapter_id,
                    progress.records_seen,
                    progress.events_written,
                    progress.events_skipped,
                    progress.errors
                );
            }
        },
    )
    .map_err(|error| error.to_string())?;
    println!("{}", result.folder);
    Ok(())
}

fn validate_package(folder: &std::path::Path) -> Result<u64, String> {
    let events = folder.join("events");
    if !folder.join("manifest.yaml").is_file() || !folder.join("README.md").is_file() {
        return Err("package needs manifest.yaml and README.md".into());
    }
    let mut count = 0_u64;
    for entry in walkdir::WalkDir::new(events)
        .into_iter()
        .filter_map(Result::ok)
    {
        let file_name = entry.file_name().to_string_lossy();
        if !entry.file_type().is_file()
            || file_name.starts_with("._")
            || file_name == ".DS_Store"
            || entry.path().extension().is_none_or(|value| value != "md")
        {
            continue;
        }
        let markdown = std::fs::read_to_string(entry.path())
            .map_err(|error| format!("{}: {error}", entry.path().display()))?;
        let yaml = markdown
            .strip_prefix("---\n")
            .and_then(|content| content.split_once("\n---\n").map(|parts| parts.0))
            .ok_or_else(|| format!("{}: missing YAML frontmatter", entry.path().display()))?;
        let value: serde_yaml::Value = serde_yaml::from_str(yaml)
            .map_err(|error| format!("{}: {error}", entry.path().display()))?;
        if value["schema_version"].as_u64() != Some(1) {
            return Err(format!(
                "{}: schema_version is not 1",
                entry.path().display()
            ));
        }
        for key in ["id", "date", "source", "module", "type", "title"] {
            if value[key].as_str().is_none_or(str::is_empty) {
                return Err(format!("{}: missing {key}", entry.path().display()));
            }
        }
        let provenance = &value["provenance"];
        if provenance["raw_path"].as_str().is_none_or(str::is_empty)
            || provenance["raw_hash"]
                .as_str()
                .is_none_or(|hash| !hash.starts_with("sha256:"))
        {
            return Err(format!(
                "{}: invalid provenance raw_path={:?} raw_hash={:?}",
                entry.path().display(),
                provenance["raw_path"],
                provenance["raw_hash"]
            ));
        }
        count += 1;
    }
    if count == 0 {
        return Err("package contains no event Markdown".into());
    }
    Ok(count)
}

fn argument_value(arguments: &[String], name: &str) -> Result<String, String> {
    let index = arguments
        .iter()
        .position(|argument| argument == name)
        .ok_or_else(|| format!("missing {name}"))?;
    arguments
        .get(index + 1)
        .cloned()
        .ok_or_else(|| format!("missing value after {name}"))
}
