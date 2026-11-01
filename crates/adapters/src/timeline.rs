// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{AdapterError, OpenEvent};
use chrono::{Datelike, Duration, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

const TIMELINE_FOLDER: &str = "Amara-Unfold-Timeline";
const TIMELINE_KIND: &str = "amara-unfold-timeline-index";
const MAX_SPAN_DAYS: i64 = 366;

pub type TimelineProgressCallback<'a> = dyn FnMut(u64, u64, &str) + 'a;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TimelineBuildResult {
    pub folder: String,
    pub events_indexed: u64,
    pub duplicate_events: u64,
    pub rejected_events: u64,
    pub dated_events: u64,
    pub undated_events: u64,
    pub day_pages: u64,
    pub month_pages: u64,
    pub entity_anchors: u64,
    pub cross_source_days: u64,
    pub strict_portable: bool,
}

#[derive(Debug, Clone)]
struct IndexedEvent {
    event: OpenEvent,
    relative_path: PathBuf,
    has_unquoted_date_scalar: bool,
    has_replacement_character: bool,
    has_control_character: bool,
    has_normalization_marker: bool,
    has_raw_html: bool,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct QualityReport {
    events_indexed: u64,
    duplicate_events: u64,
    rejected_events: u64,
    empty_titles: u64,
    empty_bodies: u64,
    replacement_character_events: u64,
    control_character_events: u64,
    normalized_source_character_events: u64,
    raw_html_events: u64,
    unquoted_date_scalar_events: u64,
    filename_only_titles: u64,
    undated_events: u64,
    reference_events: u64,
    month_precision_events: u64,
    year_precision_events: u64,
    ready_events: u64,
    review_suggested_events: u64,
    strict_portable: bool,
    rejected_paths: Vec<String>,
}

#[derive(Debug, Default)]
struct EntityAggregate {
    kind: String,
    id: String,
    label: String,
    event_ids: BTreeSet<String>,
    sources: BTreeSet<String>,
    dates: BTreeSet<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionGraph {
    schema_version: u8,
    generated_at: String,
    evidence_rules: Vec<&'static str>,
    cross_source_days: Vec<DayConnection>,
    same_day_actor_place_candidates: Vec<ActorPlaceCandidate>,
    shared_entities: Vec<EntityConnection>,
    direct_actor_place_events: Vec<DirectAssociation>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DayConnection {
    date: String,
    sources: Vec<String>,
    event_ids: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ActorPlaceCandidate {
    date: String,
    actors: Vec<String>,
    places: Vec<String>,
    event_ids: Vec<String>,
    evidence: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EntityConnection {
    kind: String,
    id: String,
    label: String,
    sources: Vec<String>,
    event_ids: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DirectAssociation {
    event_id: String,
    date: String,
    actors: Vec<String>,
    places: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TimelineManifest<'a> {
    kind: &'a str,
    schema_version: u8,
    generated_at: String,
    source_contract: &'a str,
    events_indexed: u64,
    duplicate_events: u64,
    rejected_events: u64,
    day_pages: u64,
    month_pages: u64,
    entity_anchors: u64,
    cross_source_days: u64,
    strict_portable: bool,
    network_used: bool,
}

pub fn build_timeline(
    destination: &Path,
    progress: &mut TimelineProgressCallback<'_>,
) -> Result<TimelineBuildResult, AdapterError> {
    build_timeline_cancellable(destination, &AtomicBool::new(false), progress)
}

/// Cancellation preserves the last complete Timeline; committed adapter output stays saved.
pub fn build_timeline_cancellable(
    destination: &Path,
    cancel: &AtomicBool,
    progress: &mut TimelineProgressCallback<'_>,
) -> Result<TimelineBuildResult, AdapterError> {
    let check_cancel = || {
        if cancel.load(Ordering::Relaxed) {
            Err(AdapterError::Cancelled)
        } else {
            Ok(())
        }
    };
    check_cancel()?;
    if !destination.is_dir() {
        return Err(AdapterError::InvalidPath(destination.display().to_string()));
    }

    let event_paths = event_paths(destination);
    let total = event_paths.len() as u64;
    progress(0, total, "Finding open Markdown events");

    let mut events = BTreeMap::<String, IndexedEvent>::new();
    let mut quality = QualityReport::default();
    for (index, path) in event_paths.iter().enumerate() {
        check_cancel()?;
        match read_event(destination, path) {
            Ok(indexed) => {
                let id = indexed.event.id.clone();
                if let Some(existing) = events.get(&id) {
                    quality.duplicate_events += 1;
                    if existing.event.provenance.raw_hash != indexed.event.provenance.raw_hash {
                        quality.rejected_events += 1;
                        quality.rejected_paths.push(format!(
                            "{} (conflicting duplicate id {id})",
                            indexed.relative_path.display()
                        ));
                    }
                } else {
                    events.insert(id, indexed);
                }
            }
            Err(error) => {
                quality.rejected_events += 1;
                if quality.rejected_paths.len() < 100 {
                    quality
                        .rejected_paths
                        .push(format!("{} ({error})", path.display()));
                }
            }
        }
        let seen = index as u64 + 1;
        if seen == total || seen == 1 || seen.is_multiple_of(500) {
            progress(seen, total, "Reading titles, dates, people, and places");
        }
    }
    check_cancel()?;
    if events.is_empty() {
        return Err(AdapterError::Parse(
            "no schema-v1 event Markdown was found in the output folder".into(),
        ));
    }

    quality.events_indexed = events.len() as u64;
    let mut days = BTreeMap::<NaiveDate, BTreeSet<String>>::new();
    let mut event_days = BTreeMap::<String, Vec<NaiveDate>>::new();
    let mut month_only = BTreeMap::<String, BTreeSet<String>>::new();
    let mut year_only = BTreeMap::<String, BTreeSet<String>>::new();
    let mut undated = BTreeSet::<String>::new();
    let mut references = BTreeSet::<String>::new();
    let mut entities = BTreeMap::<(String, String), EntityAggregate>::new();
    let mut span_warnings = Vec::new();

    for indexed in events.values() {
        check_cancel()?;
        update_quality(&mut quality, indexed);
        map_event_dates(
            indexed,
            &mut days,
            &mut event_days,
            &mut month_only,
            &mut year_only,
            &mut undated,
            &mut references,
            &mut span_warnings,
        );
        map_entities(indexed, &mut entities);
    }
    quality.strict_portable = quality.rejected_events == 0
        && quality.empty_titles == 0
        && quality.empty_bodies == 0
        && quality.replacement_character_events == 0
        && quality.control_character_events == 0
        && quality.unquoted_date_scalar_events == 0;

    let temporary = destination.join(format!(".{TIMELINE_FOLDER}.tmp-{}", std::process::id()));
    if temporary.exists() {
        fs::remove_dir_all(&temporary).map_err(output_error)?;
    }
    fs::create_dir_all(&temporary).map_err(output_error)?;
    let _cleanup = TimelineTemporary(temporary.clone());

    progress(total, total, "Writing chronological day pages");
    check_cancel()?;
    write_day_pages(&temporary, &events, &days)?;
    let month_keys = write_month_pages(destination, &temporary, &events, &days, &month_only)?;
    write_year_pages(destination, &temporary, &events, &month_keys, &year_only)?;
    write_undated(destination, &temporary, &events, &undated)?;
    write_reference_notes(destination, &temporary, &events, &references)?;

    progress(total, total, "Writing entity and connection indexes");
    check_cancel()?;
    write_entity_indexes(destination, &temporary, &events, &entities)?;
    let graph = connection_graph(&events, &days, &entities);
    write_connections(destination, &temporary, &events, &graph)?;
    write_catalog(destination, &temporary, &events, &event_days)?;
    write_quality_report(&temporary, &quality, &span_warnings)?;
    write_root_index(
        &temporary,
        &events,
        &days,
        &month_keys,
        &undated,
        &references,
        &entities,
        &graph,
        &quality,
    )?;

    let result = TimelineBuildResult {
        folder: destination
            .join(TIMELINE_FOLDER)
            .to_string_lossy()
            .into_owned(),
        events_indexed: events.len() as u64,
        duplicate_events: quality.duplicate_events,
        rejected_events: quality.rejected_events,
        dated_events: (events.len() - undated.len() - references.len()) as u64,
        undated_events: undated.len() as u64,
        day_pages: days.len() as u64,
        month_pages: month_keys.len() as u64,
        entity_anchors: entities.len() as u64,
        cross_source_days: graph.cross_source_days.len() as u64,
        strict_portable: quality.strict_portable,
    };
    write_manifest(&temporary, &result)?;
    remove_platform_metadata(&temporary)?;
    progress(total, total, "Finalizing the complete timeline");
    check_cancel()?;
    promote_timeline(destination, &temporary)?;
    progress(
        total,
        total,
        "Human-readable timeline and connections are ready",
    );
    Ok(result)
}

struct TimelineTemporary(PathBuf);
impl Drop for TimelineTemporary {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn event_paths(destination: &Path) -> Vec<PathBuf> {
    let generated = destination.join(TIMELINE_FOLDER);
    let mut paths = walkdir::WalkDir::new(destination)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            !entry
                .file_name()
                .to_string_lossy()
                .starts_with(".unfold-staging-")
        })
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| !path.starts_with(&generated))
        .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
        .filter(|path| {
            path.components()
                .any(|component| component.as_os_str() == "events")
        })
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| !name.to_string_lossy().starts_with("._"))
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

fn read_event(destination: &Path, path: &Path) -> Result<IndexedEvent, String> {
    let markdown = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let yaml = markdown
        .strip_prefix("---\n")
        .and_then(|content| content.split_once("\n---\n"))
        .ok_or_else(|| "missing YAML frontmatter".to_owned())?;
    let mut event: OpenEvent = serde_yaml::from_str(yaml.0).map_err(|error| error.to_string())?;
    event.body = yaml.1.trim().to_owned();
    validate_event(&event)?;
    Ok(IndexedEvent {
        event,
        relative_path: path.strip_prefix(destination).unwrap_or(path).to_path_buf(),
        has_unquoted_date_scalar: yaml.0.lines().any(|line| {
            line.strip_prefix("date: ").is_some_and(|value| {
                let value = value.trim_start();
                !value.starts_with('"') && !value.starts_with('\'')
            })
        }),
        has_replacement_character: markdown.contains('\u{fffd}'),
        has_control_character: markdown
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t')),
        has_normalization_marker: markdown.contains("[source character unavailable]")
            || markdown.contains("[source control character removed]"),
        has_raw_html: contains_raw_html(yaml.1),
    })
}

fn validate_event(event: &OpenEvent) -> Result<(), String> {
    if event.schema_version != 1 {
        return Err("schema_version is not 1".into());
    }
    for (key, value) in [
        ("id", event.id.as_str()),
        ("date", event.date.as_str()),
        ("source", event.source.as_str()),
        ("module", event.module.as_str()),
        ("type", event.event_type.as_str()),
        ("title", event.title.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("missing {key}"));
        }
    }
    if event.provenance.raw_path.trim().is_empty()
        || !event.provenance.raw_hash.starts_with("sha256:")
    {
        return Err("invalid provenance".into());
    }
    Ok(())
}

fn update_quality(quality: &mut QualityReport, indexed: &IndexedEvent) {
    let event = &indexed.event;
    quality.empty_titles += event.title.trim().is_empty() as u64;
    quality.empty_bodies += event.body.trim().is_empty() as u64;
    quality.replacement_character_events += indexed.has_replacement_character as u64;
    quality.control_character_events += indexed.has_control_character as u64;
    quality.normalized_source_character_events += indexed.has_normalization_marker as u64;
    quality.raw_html_events += indexed.has_raw_html as u64;
    quality.unquoted_date_scalar_events += indexed.has_unquoted_date_scalar as u64;
    quality.filename_only_titles += is_filename_title(&event.title) as u64;
    if is_timeless_reference(event) {
        quality.reference_events += 1;
    } else {
        quality.undated_events += event.date.starts_with("0000-") as u64;
    }
    quality.month_precision_events += (event.date_precision == "month") as u64;
    quality.year_precision_events +=
        (event.date_precision == "year" && !event.date.starts_with("0000-")) as u64;
    if review_reasons(indexed).is_empty() {
        quality.ready_events += 1;
    } else {
        quality.review_suggested_events += 1;
    }
}

#[allow(clippy::too_many_arguments)]
fn map_event_dates(
    indexed: &IndexedEvent,
    days: &mut BTreeMap<NaiveDate, BTreeSet<String>>,
    event_days: &mut BTreeMap<String, Vec<NaiveDate>>,
    month_only: &mut BTreeMap<String, BTreeSet<String>>,
    year_only: &mut BTreeMap<String, BTreeSet<String>>,
    undated: &mut BTreeSet<String>,
    references: &mut BTreeSet<String>,
    span_warnings: &mut Vec<String>,
) {
    let event = &indexed.event;
    if event.date.starts_with("0000-") {
        if is_timeless_reference(event) {
            references.insert(event.id.clone());
        } else {
            undated.insert(event.id.clone());
        }
        return;
    }
    if event.date_precision == "month" || event.date.ends_with("-00") {
        month_only
            .entry(event.date.get(..7).unwrap_or(&event.date).to_owned())
            .or_default()
            .insert(event.id.clone());
        return;
    }
    if event.date_precision == "year" || event.date.ends_with("-00-00") {
        year_only
            .entry(event.date.get(..4).unwrap_or(&event.date).to_owned())
            .or_default()
            .insert(event.id.clone());
        return;
    }
    let Ok(start) = NaiveDate::parse_from_str(&event.date, "%Y-%m-%d") else {
        undated.insert(event.id.clone());
        return;
    };
    let mut end = event
        .end_date
        .as_deref()
        .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
        .filter(|value| *value >= start)
        .unwrap_or(start);
    if (end - start).num_days() > MAX_SPAN_DAYS {
        span_warnings.push(format!(
            "{} spans more than {MAX_SPAN_DAYS} days; timeline expansion stops at the limit",
            event.id
        ));
        end = start + Duration::days(MAX_SPAN_DAYS);
    }
    let mut mapped = Vec::new();
    let mut day = start;
    while day <= end {
        days.entry(day).or_default().insert(event.id.clone());
        mapped.push(day);
        day += Duration::days(1);
    }
    event_days.insert(event.id.clone(), mapped);
}

fn map_entities(
    indexed: &IndexedEvent,
    entities: &mut BTreeMap<(String, String), EntityAggregate>,
) {
    let event = &indexed.event;
    for actor in &event.actors {
        let kind = actor_kind(actor);
        add_entity(entities, kind, actor, event, None);
    }
    for place in &event.places {
        let label = (event.source == "location").then_some(event.title.as_str());
        add_entity(entities, "places", place, event, label);
    }
}

fn add_entity(
    entities: &mut BTreeMap<(String, String), EntityAggregate>,
    kind: &str,
    id: &str,
    event: &OpenEvent,
    preferred_label: Option<&str>,
) {
    let key = (kind.to_owned(), id.to_owned());
    let entity = entities.entry(key).or_insert_with(|| EntityAggregate {
        kind: kind.to_owned(),
        id: id.to_owned(),
        label: preferred_label
            .map(str::to_owned)
            .unwrap_or_else(|| human_entity_label(kind, id)),
        ..EntityAggregate::default()
    });
    if let Some(label) = preferred_label.filter(|label| !label.trim().is_empty()) {
        entity.label = label.to_owned();
    }
    entity.event_ids.insert(event.id.clone());
    entity.sources.insert(event.source.clone());
    entity.dates.insert(event.date.clone());
}

fn write_day_pages(
    temporary: &Path,
    events: &BTreeMap<String, IndexedEvent>,
    days: &BTreeMap<NaiveDate, BTreeSet<String>>,
) -> Result<(), AdapterError> {
    for (date, ids) in days {
        let directory = temporary
            .join("days")
            .join(format!("{:04}", date.year()))
            .join(format!("{:02}", date.month()));
        fs::create_dir_all(&directory).map_err(output_error)?;
        let relative_directory = PathBuf::from(TIMELINE_FOLDER)
            .join("days")
            .join(format!("{:04}", date.year()))
            .join(format!("{:02}", date.month()));
        let mut ordered = ids
            .iter()
            .filter_map(|id| events.get(id))
            .collect::<Vec<_>>();
        ordered.sort_by_key(|indexed| {
            (
                indexed.event.time.clone().unwrap_or_default(),
                indexed.event.source.clone(),
                indexed.event.title.clone(),
            )
        });
        let mut output = format!(
            "# {date}\n\n{} records mapped to this day. Linked event files remain the source of truth.\n\n",
            ordered.len()
        );
        output.push_str("| Time | Source | Event | Evidence |\n|---|---|---|---|\n");
        for indexed in &ordered {
            let event = &indexed.event;
            let link = relative_link(&relative_directory, &indexed.relative_path);
            let evidence = event_evidence(event, *date);
            output.push_str(&format!(
                "| {} | {} | [{}]({}) | {} |\n",
                markdown_cell(event.time.as_deref().unwrap_or("—")),
                markdown_cell(&event.source),
                markdown_cell(&event.title),
                link,
                markdown_cell(&evidence)
            ));
        }
        append_day_connections(&mut output, &ordered);
        fs::write(directory.join(format!("{date}.md")), output).map_err(output_error)?;
    }
    Ok(())
}

fn append_day_connections(output: &mut String, events: &[&IndexedEvent]) {
    let sources = events
        .iter()
        .map(|indexed| indexed.event.source.clone())
        .collect::<BTreeSet<_>>();
    let mut actor_events = BTreeMap::<String, usize>::new();
    let mut place_events = BTreeMap::<String, usize>::new();
    let mut direct = Vec::new();
    for indexed in events {
        for actor in &indexed.event.actors {
            *actor_events.entry(actor.clone()).or_default() += 1;
        }
        for place in &indexed.event.places {
            *place_events.entry(place.clone()).or_default() += 1;
        }
        if !indexed.event.actors.is_empty() && !indexed.event.places.is_empty() {
            direct.push(format!(
                "{} directly contains actor and place evidence",
                indexed.event.title
            ));
        }
    }
    let shared_actors = actor_events
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .collect::<Vec<_>>();
    let shared_places = place_events
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .collect::<Vec<_>>();
    let all_actors = events
        .iter()
        .flat_map(|indexed| indexed.event.actors.iter().cloned())
        .collect::<BTreeSet<_>>();
    let all_places = events
        .iter()
        .flat_map(|indexed| indexed.event.places.iter().cloned())
        .collect::<BTreeSet<_>>();
    if sources.len() < 2
        && shared_actors.is_empty()
        && shared_places.is_empty()
        && direct.is_empty()
    {
        return;
    }
    output.push_str("\n## Evidence-based connections\n\n");
    if sources.len() > 1 {
        output.push_str(&format!(
            "- Same-date intersection across **{}**. This is context, not proof that the records describe one activity.\n",
            sources.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    if !all_actors.is_empty() && !all_places.is_empty() {
        output.push_str(&format!(
            "- Review candidate: actors **{}** and places **{}** occur somewhere in this day's evidence. This does not prove that a person was at a place.\n",
            all_actors.into_iter().collect::<Vec<_>>().join(", "),
            all_places.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    for (actor, count) in shared_actors {
        output.push_str(&format!(
            "- Unresolved actor `{actor}` appears in {count} records on this date.\n"
        ));
    }
    for (place, count) in shared_places {
        output.push_str(&format!(
            "- Place candidate `{place}` appears in {count} records on this date.\n"
        ));
    }
    for association in direct {
        output.push_str(&format!("- {association}.\n"));
    }
}

fn write_month_pages(
    destination: &Path,
    temporary: &Path,
    events: &BTreeMap<String, IndexedEvent>,
    days: &BTreeMap<NaiveDate, BTreeSet<String>>,
    month_only: &BTreeMap<String, BTreeSet<String>>,
) -> Result<BTreeSet<String>, AdapterError> {
    let mut months = month_only.keys().cloned().collect::<BTreeSet<_>>();
    for date in days.keys() {
        months.insert(format!("{:04}-{:02}", date.year(), date.month()));
    }
    for month in &months {
        let year = month.get(..4).unwrap_or("0000");
        let directory = temporary.join("months").join(year);
        fs::create_dir_all(&directory).map_err(output_error)?;
        let relative_directory = PathBuf::from(TIMELINE_FOLDER).join("months").join(year);
        let month_days = days
            .iter()
            .filter(|(date, _)| format!("{:04}-{:02}", date.year(), date.month()) == *month)
            .collect::<Vec<_>>();
        let placement_count = month_days.iter().map(|(_, ids)| ids.len()).sum::<usize>();
        let mut output = format!(
            "# {month}\n\n{placement_count} dated record placements across {} days.\n\n",
            month_days.len()
        );
        if !month_days.is_empty() {
            output.push_str("## Days\n\n");
            for (date, ids) in month_days {
                output.push_str(&format!(
                    "- [{date}](../../days/{}/{:02}/{date}.md) — {} records\n",
                    date.year(),
                    date.month(),
                    ids.len()
                ));
            }
        }
        if let Some(ids) = month_only.get(month) {
            output.push_str("\n## Known only to the month\n\n");
            for id in ids {
                if let Some(indexed) = events.get(id) {
                    output.push_str(&format!(
                        "- [{}]({}) — {}\n",
                        markdown_inline(&indexed.event.title),
                        relative_link(&relative_directory, &indexed.relative_path),
                        markdown_inline(&indexed.event.source)
                    ));
                }
            }
        }
        fs::write(directory.join(format!("{month}.md")), output).map_err(output_error)?;
    }
    let _ = destination;
    Ok(months)
}

fn write_year_pages(
    _destination: &Path,
    temporary: &Path,
    events: &BTreeMap<String, IndexedEvent>,
    months: &BTreeSet<String>,
    year_only: &BTreeMap<String, BTreeSet<String>>,
) -> Result<(), AdapterError> {
    let mut years = year_only.keys().cloned().collect::<BTreeSet<_>>();
    years.extend(
        months
            .iter()
            .filter_map(|month| month.get(..4).map(str::to_owned)),
    );
    let directory = temporary.join("years");
    fs::create_dir_all(&directory).map_err(output_error)?;
    let relative_directory = PathBuf::from(TIMELINE_FOLDER).join("years");
    for year in years {
        let mut output = format!("# {year}\n\n## Months\n\n");
        for month in months.iter().filter(|month| month.starts_with(&year)) {
            output.push_str(&format!("- [{month}](../months/{year}/{month}.md)\n"));
        }
        if let Some(ids) = year_only.get(&year) {
            output.push_str("\n## Known only to the year\n\n");
            for id in ids {
                if let Some(indexed) = events.get(id) {
                    output.push_str(&format!(
                        "- [{}]({}) — {}\n",
                        markdown_inline(&indexed.event.title),
                        relative_link(&relative_directory, &indexed.relative_path),
                        markdown_inline(&indexed.event.source)
                    ));
                }
            }
        }
        fs::write(directory.join(format!("{year}.md")), output).map_err(output_error)?;
    }
    Ok(())
}

fn write_undated(
    _destination: &Path,
    temporary: &Path,
    events: &BTreeMap<String, IndexedEvent>,
    undated: &BTreeSet<String>,
) -> Result<(), AdapterError> {
    let relative_directory = PathBuf::from(TIMELINE_FOLDER);
    let mut grouped = BTreeMap::<String, Vec<&IndexedEvent>>::new();
    for id in undated {
        if let Some(indexed) = events.get(id) {
            grouped
                .entry(indexed.event.module.clone())
                .or_default()
                .push(indexed);
        }
    }
    let mut output = format!(
        "# Undated records\n\n{} records have no defensible date in their source. They remain searchable without being assigned an invented day.\n",
        undated.len()
    );
    for (module, mut records) in grouped {
        records.sort_by_key(|indexed| indexed.event.title.clone());
        output.push_str(&format!("\n## {}\n\n", markdown_inline(&module)));
        for indexed in records {
            output.push_str(&format!(
                "- [{}]({}) — {}\n",
                markdown_inline(&indexed.event.title),
                relative_link(&relative_directory, &indexed.relative_path),
                markdown_inline(&indexed.event.event_type)
            ));
        }
    }
    fs::write(temporary.join("undated.md"), output).map_err(output_error)
}

fn write_reference_notes(
    _destination: &Path,
    temporary: &Path,
    events: &BTreeMap<String, IndexedEvent>,
    references: &BTreeSet<String>,
) -> Result<(), AdapterError> {
    let relative_directory = PathBuf::from(TIMELINE_FOLDER);
    let mut grouped = BTreeMap::<String, Vec<&IndexedEvent>>::new();
    for id in references {
        if let Some(indexed) = events.get(id) {
            grouped
                .entry(indexed.event.module.clone())
                .or_default()
                .push(indexed);
        }
    }
    let mut output = format!(
        "# Timeless reference notes\n\n{} topical records have no chronological date in their source. They remain searchable and traceable without being assigned an invented date.\n",
        references.len()
    );
    for (module, mut records) in grouped {
        records.sort_by_key(|indexed| indexed.event.title.clone());
        output.push_str(&format!("\n## {}\n\n", markdown_inline(&module)));
        for indexed in records {
            output.push_str(&format!(
                "- [{}]({}) — {}\n",
                markdown_inline(&indexed.event.title),
                relative_link(&relative_directory, &indexed.relative_path),
                markdown_inline(&indexed.event.event_type)
            ));
        }
    }
    fs::write(temporary.join("reference-notes.md"), output).map_err(output_error)
}

fn write_entity_indexes(
    _destination: &Path,
    temporary: &Path,
    events: &BTreeMap<String, IndexedEvent>,
    entities: &BTreeMap<(String, String), EntityAggregate>,
) -> Result<(), AdapterError> {
    let directory = temporary.join("entities");
    fs::create_dir_all(&directory).map_err(output_error)?;
    let relative_directory = PathBuf::from(TIMELINE_FOLDER).join("entities");
    let kinds = ["people", "merchants", "artists", "channels", "places"];
    for kind in kinds {
        let mut records = entities
            .values()
            .filter(|entity| entity.kind == kind)
            .collect::<Vec<_>>();
        records
            .sort_by_key(|entity| (std::cmp::Reverse(entity.event_ids.len()), entity.id.clone()));
        let mut output = format!(
            "# {}\n\nThese identifiers are derived from source evidence. They are connection candidates, not confirmed real-world identities.\n",
            entity_kind_label(kind)
        );
        for entity in records {
            output.push_str(&format!(
                "\n## {}\n\n`{}` · {} records · sources: {}\n\n",
                markdown_inline(&entity.label),
                entity.id,
                entity.event_ids.len(),
                entity
                    .sources
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            for id in entity.event_ids.iter().take(20) {
                if let Some(indexed) = events.get(id) {
                    output.push_str(&format!(
                        "- {} — [{}]({})\n",
                        indexed.event.date,
                        markdown_inline(&indexed.event.title),
                        relative_link(&relative_directory, &indexed.relative_path)
                    ));
                }
            }
            if entity.event_ids.len() > 20 {
                output.push_str(&format!(
                    "- … {} more records are listed in `catalog.csv`\n",
                    entity.event_ids.len() - 20
                ));
            }
        }
        fs::write(directory.join(format!("{kind}.md")), output).map_err(output_error)?;
    }
    Ok(())
}

fn connection_graph(
    events: &BTreeMap<String, IndexedEvent>,
    days: &BTreeMap<NaiveDate, BTreeSet<String>>,
    entities: &BTreeMap<(String, String), EntityAggregate>,
) -> ConnectionGraph {
    let mut cross_source_days = Vec::new();
    let mut same_day_actor_place_candidates = Vec::new();
    for (date, ids) in days {
        let sources = ids
            .iter()
            .filter_map(|id| events.get(id).map(|indexed| indexed.event.source.clone()))
            .collect::<BTreeSet<_>>();
        if sources.len() > 1 {
            cross_source_days.push(DayConnection {
                date: date.to_string(),
                sources: sources.into_iter().collect(),
                event_ids: ids.iter().cloned().collect(),
            });
        }
        let actors = ids
            .iter()
            .filter_map(|id| events.get(id))
            .flat_map(|indexed| indexed.event.actors.iter().cloned())
            .collect::<BTreeSet<_>>();
        let places = ids
            .iter()
            .filter_map(|id| events.get(id))
            .flat_map(|indexed| indexed.event.places.iter().cloned())
            .collect::<BTreeSet<_>>();
        if !actors.is_empty() && !places.is_empty() {
            same_day_actor_place_candidates.push(ActorPlaceCandidate {
                date: date.to_string(),
                actors: actors.into_iter().collect(),
                places: places.into_iter().collect(),
                event_ids: ids.iter().cloned().collect(),
                evidence: "same date only; human or stronger source evidence must confirm presence",
            });
        }
    }
    let shared_entities = entities
        .values()
        .filter(|entity| entity.event_ids.len() > 1)
        .map(|entity| EntityConnection {
            kind: entity.kind.clone(),
            id: entity.id.clone(),
            label: entity.label.clone(),
            sources: entity.sources.iter().cloned().collect(),
            event_ids: entity.event_ids.iter().cloned().collect(),
        })
        .collect();
    let direct_actor_place_events = events
        .values()
        .filter(|indexed| !indexed.event.actors.is_empty() && !indexed.event.places.is_empty())
        .map(|indexed| DirectAssociation {
            event_id: indexed.event.id.clone(),
            date: indexed.event.date.clone(),
            actors: indexed.event.actors.clone(),
            places: indexed.event.places.clone(),
        })
        .collect();
    ConnectionGraph {
        schema_version: 1,
        generated_at: Local::now().to_rfc3339(),
        evidence_rules: vec![
            "same-date: records share a source date; this does not prove one activity",
            "shared-actor: records contain the same unresolved actor identifier",
            "shared-place: records contain the same deterministic place identifier",
            "direct-actor-place: one source event explicitly contains both identifiers",
            "active-span: a multi-day event is visible on each covered day",
        ],
        cross_source_days,
        same_day_actor_place_candidates,
        shared_entities,
        direct_actor_place_events,
    }
}

fn write_connections(
    _destination: &Path,
    temporary: &Path,
    _events: &BTreeMap<String, IndexedEvent>,
    graph: &ConnectionGraph,
) -> Result<(), AdapterError> {
    let mut output = "# Evidence-based connections\n\nConnections are deterministic indexes over event frontmatter. Same-day intersections are candidates for review, not claims about causation or companionship.\n\n".to_owned();
    output.push_str(&format!(
        "- {} dates contain records from more than one source.\n- {} dates contain an actor/place review candidate.\n- {} unresolved entities occur in more than one event.\n- {} events directly contain both actor and place evidence.\n\n",
        graph.cross_source_days.len(),
        graph.same_day_actor_place_candidates.len(),
        graph.shared_entities.len(),
        graph.direct_actor_place_events.len()
    ));
    output.push_str("## Cross-source dates\n\n");
    for day in &graph.cross_source_days {
        let year = day.date.get(..4).unwrap_or("0000");
        let month = day.date.get(5..7).unwrap_or("00");
        output.push_str(&format!(
            "- [{}](days/{year}/{month}/{}.md) — {}\n",
            day.date,
            day.date,
            day.sources.join(", ")
        ));
    }
    fs::write(temporary.join("connections.md"), output).map_err(output_error)
}

fn write_catalog(
    destination: &Path,
    temporary: &Path,
    events: &BTreeMap<String, IndexedEvent>,
    event_days: &BTreeMap<String, Vec<NaiveDate>>,
) -> Result<(), AdapterError> {
    let path = temporary.join("catalog.csv");
    let mut writer =
        csv::Writer::from_path(&path).map_err(|error| AdapterError::Output(error.to_string()))?;
    writer
        .write_record([
            "id",
            "date",
            "time",
            "end_date",
            "source",
            "module",
            "type",
            "title",
            "actors",
            "places",
            "tags",
            "timeline_days",
            "event_path",
            "raw_path",
            "raw_hash",
            "review_status",
            "review_reasons",
        ])
        .map_err(|error| AdapterError::Output(error.to_string()))?;
    for indexed in events.values() {
        let event = &indexed.event;
        let mapped_days = event_days
            .get(&event.id)
            .map(|days| {
                days.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(";")
            })
            .unwrap_or_default();
        let review_reasons = review_reasons(indexed);
        writer
            .write_record([
                event.id.as_str(),
                event.date.as_str(),
                event.time.as_deref().unwrap_or(""),
                event.end_date.as_deref().unwrap_or(""),
                event.source.as_str(),
                event.module.as_str(),
                event.event_type.as_str(),
                event.title.as_str(),
                &event.actors.join(";"),
                &event.places.join(";"),
                &event.tags.join(";"),
                &mapped_days,
                &path_text(&indexed.relative_path),
                event.provenance.raw_path.as_str(),
                event.provenance.raw_hash.as_str(),
                if review_reasons.is_empty() {
                    "ready"
                } else {
                    "review"
                },
                &review_reasons.join(";"),
            ])
            .map_err(|error| AdapterError::Output(error.to_string()))?;
    }
    writer
        .flush()
        .map_err(|error| AdapterError::Output(error.to_string()))?;
    let _ = destination;
    Ok(())
}

fn write_quality_report(
    temporary: &Path,
    quality: &QualityReport,
    span_warnings: &[String],
) -> Result<(), AdapterError> {
    let status = if quality.strict_portable {
        "PASS"
    } else {
        "NEEDS ATTENTION"
    };
    let mut markdown = format!(
        "# Data quality report\n\n**Strict portability: {status}**\n\n`ready` means structurally clean enough for an importer. `review` preserves the record but exposes a reason for human or resolver attention; it does not mean the source event is invalid.\n\n| Check | Count |\n|---|---:|\n| Events indexed | {} |\n| Ready events | {} |\n| Review suggested | {} |\n| Duplicate events ignored | {} |\n| Rejected events | {} |\n| Empty titles | {} |\n| Empty bodies | {} |\n| Unicode replacement characters | {} |\n| Invalid control characters | {} |\n| Visible source-normalization markers | {} |\n| Raw HTML bodies | {} |\n| Unquoted date scalars | {} |\n| Filename-only titles | {} |\n| Undated records | {} |\n| Timeless reference notes | {} |\n| Month-precision records | {} |\n| Year-precision records | {} |\n",
        quality.events_indexed,
        quality.ready_events,
        quality.review_suggested_events,
        quality.duplicate_events,
        quality.rejected_events,
        quality.empty_titles,
        quality.empty_bodies,
        quality.replacement_character_events,
        quality.control_character_events,
        quality.normalized_source_character_events,
        quality.raw_html_events,
        quality.unquoted_date_scalar_events,
        quality.filename_only_titles,
        quality.undated_events,
        quality.reference_events,
        quality.month_precision_events,
        quality.year_precision_events,
    );
    if !quality.rejected_paths.is_empty() {
        markdown.push_str("\n## Rejected records\n\n");
        for path in &quality.rejected_paths {
            markdown.push_str(&format!("- `{}`\n", markdown_inline(path)));
        }
    }
    if !span_warnings.is_empty() {
        markdown.push_str("\n## Span warnings\n\n");
        for warning in span_warnings {
            markdown.push_str(&format!("- {}\n", markdown_inline(warning)));
        }
    }
    fs::write(temporary.join("quality-report.md"), markdown).map_err(output_error)
}

#[allow(clippy::too_many_arguments)]
fn write_root_index(
    temporary: &Path,
    events: &BTreeMap<String, IndexedEvent>,
    days: &BTreeMap<NaiveDate, BTreeSet<String>>,
    months: &BTreeSet<String>,
    undated: &BTreeSet<String>,
    references: &BTreeSet<String>,
    entities: &BTreeMap<(String, String), EntityAggregate>,
    graph: &ConnectionGraph,
    quality: &QualityReport,
) -> Result<(), AdapterError> {
    let mut module_counts = BTreeMap::<String, usize>::new();
    for indexed in events.values() {
        *module_counts
            .entry(indexed.event.module.clone())
            .or_default() += 1;
    }
    let years = months
        .iter()
        .filter_map(|month| month.get(..4).map(str::to_owned))
        .collect::<BTreeSet<_>>();
    let quality_label = if quality.strict_portable {
        "passes strict portability checks"
    } else {
        "has issues requiring review"
    };
    let day_link = if days.is_empty() {
        String::new()
    } else {
        format!("- [{} day pages](days/)\n", days.len())
    };
    let mut output = format!(
        "# Amara Unfold Timeline\n\nThis is the human-readable index for **{} unique events**. Event Markdown remains the source of truth; this timeline can be deleted and rebuilt.\n\n{}- [{} month pages](months/)\n- [{} undated records](undated.md)\n- [{} timeless reference notes](reference-notes.md)\n- [{} cross-source dates](connections.md)\n- [{} unresolved entity anchors](entities/)\n- [Compact event catalog](catalog.csv)\n- [Data quality report](quality-report.md): {}\n\n",
        events.len(),
        day_link,
        months.len(),
        undated.len(),
        references.len(),
        graph.cross_source_days.len(),
        entities.len(),
        quality_label
    );
    output.push_str("## Years\n\n");
    for year in years {
        output.push_str(&format!("- [{year}](years/{year}.md)\n"));
    }
    output.push_str("\n## Sources\n\n| Module | Events |\n|---|---:|\n");
    for (module, count) in module_counts {
        output.push_str(&format!("| {} | {count} |\n", markdown_cell(&module)));
    }
    output.push_str("\n## Evidence rules\n\n- Dates and spans come only from source records.\n- Month-only, undated, and timeless reference records are never assigned invented days.\n- Shared actor/place identifiers create deterministic candidate links.\n- Same-day cross-source records are shown together but are not asserted to describe one activity.\n- Unknown people remain unresolved identifiers until a person resolver or human confirms them.\n");
    fs::write(temporary.join("index.md"), output).map_err(output_error)?;
    fs::write(
        temporary.join("README.md"),
        "# About this folder\n\n`index.md` is the human starting point. The timeline, connections, entity indexes, and quality report are Markdown-first. `catalog.csv` is the one compact navigation index used by the desktop app. The service event packages outside this folder remain the durable records. No account, database, network service, or Amara product is required.\n",
    )
    .map_err(output_error)
}

fn write_manifest(temporary: &Path, result: &TimelineBuildResult) -> Result<(), AdapterError> {
    let manifest = TimelineManifest {
        kind: TIMELINE_KIND,
        schema_version: 1,
        generated_at: Local::now().to_rfc3339(),
        source_contract: "Amara Unfold Open Event Schema v1 Markdown",
        events_indexed: result.events_indexed,
        duplicate_events: result.duplicate_events,
        rejected_events: result.rejected_events,
        day_pages: result.day_pages,
        month_pages: result.month_pages,
        entity_anchors: result.entity_anchors,
        cross_source_days: result.cross_source_days,
        strict_portable: result.strict_portable,
        network_used: false,
    };
    let yaml = serde_yaml::to_string(&manifest)
        .map_err(|error| AdapterError::Output(error.to_string()))?;
    fs::write(temporary.join("manifest.yaml"), yaml).map_err(output_error)
}

fn promote_timeline(destination: &Path, temporary: &Path) -> Result<(), AdapterError> {
    let final_path = destination.join(TIMELINE_FOLDER);
    let previous = temporary.with_extension("previous");
    if previous.exists() {
        return Err(AdapterError::Output(
            "a previous timeline recovery folder already exists".into(),
        ));
    }
    let had_previous = final_path.exists();
    if had_previous {
        let manifest = fs::read_to_string(final_path.join("manifest.yaml")).map_err(|_| {
            AdapterError::Output(format!(
                "{} already exists but is not a generated Amara Unfold timeline",
                final_path.display()
            ))
        })?;
        if !manifest.contains(&format!("kind: {TIMELINE_KIND}")) {
            return Err(AdapterError::Output(format!(
                "{} already exists but is not a generated Amara Unfold timeline",
                final_path.display()
            )));
        }
        fs::rename(&final_path, &previous).map_err(output_error)?;
    }
    if let Err(error) = fs::rename(temporary, &final_path) {
        if had_previous {
            fs::rename(&previous, &final_path).map_err(|restore| AdapterError::Output(format!("timeline promotion failed: {error}; restore failed: {restore}; prior timeline retained at {}", previous.display())))?;
        }
        return Err(output_error(error));
    }
    if had_previous {
        fs::remove_dir_all(previous).map_err(output_error)?;
    }
    Ok(())
}

fn event_evidence(event: &OpenEvent, mapped_date: NaiveDate) -> String {
    let mut evidence = Vec::new();
    if event.date != mapped_date.to_string() {
        evidence.push(format!("active span from {}", event.date));
    }
    if !event.actors.is_empty() {
        evidence.push(format!("actors: {}", event.actors.join(", ")));
    }
    if !event.places.is_empty() {
        evidence.push(format!("places: {}", event.places.join(", ")));
    }
    if evidence.is_empty() {
        "date only".into()
    } else {
        evidence.join("; ")
    }
}

fn actor_kind(actor: &str) -> &'static str {
    if actor.starts_with("merchant-") {
        "merchants"
    } else if actor.starts_with("artist-") {
        "artists"
    } else if actor.starts_with("channel-") {
        "channels"
    } else {
        "people"
    }
}

fn entity_kind_label(kind: &str) -> &str {
    match kind {
        "people" => "Unresolved people and correspondents",
        "merchants" => "Merchants",
        "artists" => "Artists",
        "channels" => "Channels",
        "places" => "Places",
        _ => "Entities",
    }
}

fn human_entity_label(kind: &str, id: &str) -> String {
    let stripped = match kind {
        "merchants" => id.strip_prefix("merchant-").unwrap_or(id),
        "artists" => id.strip_prefix("artist-").unwrap_or(id),
        "channels" => id.strip_prefix("channel-").unwrap_or(id),
        _ => id,
    };
    stripped.replace('-', " ")
}

fn relative_link(from_directory: &Path, target: &Path) -> String {
    let depth = from_directory
        .components()
        .filter(|component| matches!(component, Component::Normal(_)))
        .count();
    format!("{}{}", "../".repeat(depth), path_text(target))
}

fn path_text(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn markdown_cell(value: &str) -> String {
    value.replace('|', "\\|").replace(['\r', '\n'], " ")
}

fn markdown_inline(value: &str) -> String {
    value.replace(['\r', '\n'], " ")
}

fn contains_raw_html(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    ["<!doctype", "<html", "<body", "<div", "<table", "<span"]
        .iter()
        .any(|needle| lower.contains(needle))
}

fn is_filename_title(title: &str) -> bool {
    let lower = title.to_ascii_lowercase();
    [".jpg", ".jpeg", ".png", ".heic", ".webp", ".mov", ".mp4"]
        .iter()
        .any(|extension| lower.ends_with(extension))
}

fn review_reasons(indexed: &IndexedEvent) -> Vec<&'static str> {
    let event = &indexed.event;
    let mut reasons = Vec::new();
    if event.date.starts_with("0000-") {
        if !is_timeless_reference(event) {
            reasons.push("undated");
        }
    } else if event.date_precision == "month" {
        reasons.push("month_precision");
    } else if event.date_precision == "year" {
        reasons.push("year_precision");
    }
    if is_filename_title(&event.title) {
        reasons.push("filename_only_title");
    }
    if indexed.has_normalization_marker || indexed.has_replacement_character {
        reasons.push("source_encoding_normalized");
    }
    if indexed.has_control_character {
        reasons.push("control_character");
    }
    if indexed.has_raw_html {
        reasons.push("raw_html_body");
    }
    if indexed.has_unquoted_date_scalar {
        reasons.push("unquoted_date_scalar");
    }
    if event
        .actors
        .iter()
        .any(|actor| actor_kind(actor) == "people")
    {
        reasons.push("identity_resolution_pending");
    }
    if event.places.iter().any(|place| place.starts_with("pg-")) {
        reasons.push("place_resolution_pending");
    }
    reasons
}

fn is_timeless_reference(event: &OpenEvent) -> bool {
    event
        .tags
        .iter()
        .any(|tag| tag.eq_ignore_ascii_case("reference"))
}

fn remove_platform_metadata(root: &Path) -> Result<(), AdapterError> {
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let name = entry.file_name().to_string_lossy();
        if name.starts_with("._") || name == ".DS_Store" {
            fs::remove_file(entry.path()).map_err(output_error)?;
        }
    }
    Ok(())
}

fn output_error(error: std::io::Error) -> AdapterError {
    AdapterError::Output(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{model::EventInput, writer::PackageWriter, ExtractionProgress};

    #[test]
    fn cancelled_final_index_preserves_complete_tree_and_allows_retry() {
        fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
            walkdir::WalkDir::new(root)
                .into_iter()
                .map(Result::unwrap)
                .filter(|entry| entry.file_type().is_file())
                .map(|entry| {
                    (
                        entry.path().strip_prefix(root).unwrap().to_path_buf(),
                        fs::read(entry.path()).unwrap(),
                    )
                })
                .collect()
        }
        for phase in [
            "Finding open Markdown events",
            "Reading titles, dates, people, and places",
            "Writing chronological day pages",
            "Writing entity and connection indexes",
            "Finalizing the complete timeline",
        ] {
            let root = tempfile::tempdir().unwrap();
            write_package(
                root.path(),
                &[event(
                    "2024-06-02",
                    None,
                    "diary",
                    "Previous complete event",
                    vec![],
                    vec![],
                )],
            );
            build_timeline(root.path(), &mut |_, _, _| {}).unwrap();
            let before = snapshot(root.path());
            let cancel = AtomicBool::new(false);
            let result = build_timeline_cancellable(root.path(), &cancel, &mut |_, _, message| {
                if message == phase {
                    cancel.store(true, Ordering::Relaxed);
                }
            });
            assert!(matches!(result, Err(AdapterError::Cancelled)), "{phase}");
            assert_eq!(snapshot(root.path()), before, "{phase}");
            assert!(!root.path().read_dir().unwrap().any(|entry| entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".tmp-")));
            cancel.store(false, Ordering::Relaxed);
            assert_eq!(
                build_timeline_cancellable(root.path(), &cancel, &mut |_, _, _| {})
                    .unwrap()
                    .events_indexed,
                1
            );
        }
    }

    fn write_package(destination: &Path, events: &[OpenEvent]) {
        let source = destination.join("source");
        fs::create_dir_all(&source).unwrap();
        let mut package = PackageWriter::new(destination, &source, "airbnb-json-takeout").unwrap();
        for event in events {
            package.write_event(event).unwrap();
        }
        package
            .finish(&ExtractionProgress {
                adapter_id: "airbnb-json-takeout".into(),
                phase: "complete".into(),
                current_item: None,
                records_seen: events.len() as u64,
                source_records_included: events.len() as u64,
                events_written: events.len() as u64,
                events_skipped: 0,
                errors: 0,
                skip_reasons: Default::default(),
                total_hint: Some(events.len() as u64),
                message: "complete".into(),
            })
            .unwrap();
        package.commit().unwrap();
    }

    fn event(
        date: &str,
        end_date: Option<&str>,
        source: &str,
        title: &str,
        actors: Vec<String>,
        places: Vec<String>,
    ) -> OpenEvent {
        OpenEvent::new(EventInput {
            date,
            time: None,
            end_date,
            source,
            module: source,
            event_type: "test",
            title,
            raw_path: "/raw/source",
            raw_bytes: title.as_bytes(),
            body: format!("Readable body for {title}"),
            actors,
            places,
            tags: Vec::new(),
            confidence: "high",
        })
    }

    #[test]
    fn timeline_expands_spans_and_keeps_inference_labels_honest() {
        let root = tempfile::tempdir().unwrap();
        let events = vec![
            event(
                "2024-06-01",
                Some("2024-06-03"),
                "location",
                "Airbnb stay",
                vec!["ag-000000001".into()],
                vec!["pg-000000001".into()],
            ),
            event(
                "2024-06-02",
                None,
                "photo",
                "Rome photo",
                vec!["ag-000000001".into()],
                vec!["pg-000000001".into()],
            ),
            event("2024-06", None, "diary", "June note", vec![], vec![]),
            event("", None, "web", "Undated item", vec![], vec![]),
        ];
        write_package(root.path(), &events);

        let result = build_timeline(root.path(), &mut |_, _, _| {}).unwrap();
        assert_eq!(result.events_indexed, 4);
        assert_eq!(result.day_pages, 3);
        assert_eq!(result.undated_events, 1);
        assert_eq!(result.cross_source_days, 1);
        assert!(result.strict_portable);

        let day = fs::read_to_string(
            root.path()
                .join(TIMELINE_FOLDER)
                .join("days/2024/06/2024-06-02.md"),
        )
        .unwrap();
        assert!(day.contains("Airbnb stay"));
        assert!(day.contains("Rome photo"));
        assert!(day.contains("Same-date intersection"));
        assert!(day.contains("context, not proof"));
        assert!(root
            .path()
            .join(TIMELINE_FOLDER)
            .join("months/2024/2024-06.md")
            .is_file());
        let index = fs::read_to_string(root.path().join(TIMELINE_FOLDER).join("index.md")).unwrap();
        assert!(index.contains("[3 day pages](days/)"));
        assert!(root
            .path()
            .join(TIMELINE_FOLDER)
            .join("catalog.csv")
            .is_file());
    }

    #[test]
    fn timeline_omits_day_link_when_no_day_pages_exist() {
        let root = tempfile::tempdir().unwrap();
        let events = vec![
            event("2024-06", None, "diary", "June note", vec![], vec![]),
            event("", None, "diary", "Undated note", vec![], vec![]),
        ];
        write_package(root.path(), &events);

        let result = build_timeline(root.path(), &mut |_, _, _| {}).unwrap();
        assert_eq!(result.day_pages, 0);
        let timeline = root.path().join(TIMELINE_FOLDER);
        assert!(!timeline.join("days").exists());
        let index = fs::read_to_string(timeline.join("index.md")).unwrap();
        assert!(!index.contains("(days/)"));
        assert!(index.contains("[1 month pages](months/)"));
    }

    #[test]
    fn timeless_reference_notes_are_not_reported_as_undated_events() {
        let root = tempfile::tempdir().unwrap();
        let mut reference = event("", None, "diary", "Hermann Hesse quotation", vec![], vec![]);
        reference.tags.push("reference".into());
        write_package(root.path(), &[reference]);

        let result = build_timeline(root.path(), &mut |_, _, _| {}).unwrap();
        assert_eq!(result.events_indexed, 1);
        assert_eq!(result.dated_events, 0);
        assert_eq!(result.undated_events, 0);

        let timeline = root.path().join(TIMELINE_FOLDER);
        let references = fs::read_to_string(timeline.join("reference-notes.md")).unwrap();
        assert!(references.contains("Hermann Hesse quotation"));
        let undated = fs::read_to_string(timeline.join("undated.md")).unwrap();
        assert!(undated.contains("0 records"));
        let quality = fs::read_to_string(timeline.join("quality-report.md")).unwrap();
        assert!(quality.contains("| Undated records | 0 |"));
        assert!(quality.contains("| Timeless reference notes | 1 |"));
        assert!(quality.contains("| Ready events | 1 |"));
        assert!(quality.contains("| Review suggested | 0 |"));
    }
}

#[cfg(test)]
#[test]
fn timeline_excludes_uncommitted_packages() {
    let root = tempfile::tempdir().unwrap();
    for file in [
        "committed/events/2024/09/event.md",
        ".unfold-staging-crash/package/events/2024/09/event.md",
        ".unfold-staging-crash/previous/events/2024/09/event.md",
    ] {
        let path = root.path().join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "synthetic").unwrap();
    }
    assert_eq!(
        event_paths(root.path()),
        vec![root.path().join("committed/events/2024/09/event.md")]
    );
}
