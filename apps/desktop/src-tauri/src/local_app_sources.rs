// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::Serialize;
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAppSource {
    id: String,
    label: String,
    tier: String,
    provider: String,
    adapter_id: Option<String>,
    path: Option<String>,
    status: String,
    record_count: u64,
    detail: String,
    next_step: String,
    privacy_note: String,
    evidence: Vec<String>,
}

struct LocalAppRoute<'a> {
    id: &'a str,
    label: &'a str,
    tier: &'a str,
    adapter_id: &'a str,
    detail: &'a str,
    next_step: &'a str,
    privacy_note: &'a str,
}

struct PendingLocalAppRoute<'a> {
    id: &'a str,
    label: &'a str,
    tier: &'a str,
    provider: &'a str,
    adapter_id: Option<&'a str>,
    path: PathBuf,
    detail: &'a str,
    next_step: &'a str,
    privacy_note: &'a str,
    inspect_user_path: bool,
}

pub fn list(include_user_locations: bool) -> Vec<LocalAppSource> {
    let Some(home) = home_dir() else {
        return Vec::new();
    };
    vec![
        apple_photos(),
        apple_calendar(&home, include_user_locations),
        apple_contacts(&home, include_user_locations),
        apple_mail(&home, include_user_locations),
        apple_music(&home),
        immich(&home, include_user_locations),
        apple_messages(&home, include_user_locations),
        apple_notes(&home, include_user_locations),
        apple_journal(&home, include_user_locations),
    ]
}

fn apple_photos() -> LocalAppSource {
    export_required(LocalAppRoute {
        id: "apple-photos",
        label: "Apple Photos",
        tier: "Photos export",
        adapter_id: "photo-media-folder",
        detail: "Photos.app keeps its live library behind macOS privacy controls. The public-ready path is to export originals or choose a normal photos/videos folder, then let Unfold read media metadata locally.",
        next_step: "Export originals from Apple Photos, or choose a normal folder containing photos/videos. Unfold reads dates, camera metadata, and GPS where available in the files.",
        privacy_note: "Photos can contain sensitive location data. Access is explicit, read-only, local-only, and limited to the folder you choose.",
    })
}

fn apple_calendar(home: &Path, include_user_locations: bool) -> LocalAppSource {
    let path = include_user_locations
        .then(|| exported_folder_with_extension(home, "Amara Unfold Calendar Exports", &["ics"]))
        .flatten();
    consent_required_optional_path(
        LocalAppRoute {
            id: "apple-calendar",
            label: "Apple Calendar",
            tier: "Calendar export",
            adapter_id: "apple-calendar-local",
            detail: "Unfold found a Calendar export folder containing .ics files. It does not read the events until you choose this source.",
            next_step: "Export calendars from Apple Calendar as .ics files, then choose the folder containing those .ics files.",
            privacy_note: "Calendar data is sensitive schedule data. This route reads only the exported .ics files you choose, stays local, and keeps no ongoing Calendar.app permission.",
        },
        path,
        "Apple Calendar import uses exported .ics files. Unfold does not inspect Calendar.app's private live database on app start.",
    )
}

fn apple_contacts(home: &Path, include_user_locations: bool) -> LocalAppSource {
    let path = include_user_locations
        .then(|| {
            exported_folder_with_extension(home, "Amara Unfold Contacts Exports", &["vcf", "vcard"])
        })
        .flatten();
    consent_required_optional_path(
        LocalAppRoute {
            id: "apple-contacts",
            label: "Apple Contacts",
            tier: "Contacts export",
            adapter_id: "apple-contacts-local",
            detail: "Unfold found a Contacts export folder containing vCards. It does not read the contacts until you choose this source.",
            next_step: "Export contacts from Apple Contacts as vCards (.vcf), then choose the folder containing those vCards. Note bodies are not emitted by default.",
            privacy_note: "Contacts are relational personal data. This route reads only exported vCards you choose, stays local, and keeps no ongoing Contacts.app permission.",
        },
        path,
        "Apple Contacts import uses exported vCards (.vcf). Unfold does not inspect Contacts.app's private live database on app start.",
    )
}

fn apple_mail(home: &Path, include_user_locations: bool) -> LocalAppSource {
    let mail_app = Path::new("/System/Applications/Mail.app");
    let installed = mail_app.exists();
    let export = include_user_locations
        .then(|| apple_mail_export_path(home))
        .flatten();
    LocalAppSource {
        id: "apple-mail".into(),
        label: "Apple Mail".into(),
        tier: "Mail export".into(),
        provider: "Apple Mail".into(),
        adapter_id: Some("apple-mail-local".into()),
        path: export
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        status: if export.is_some() {
            "ready"
        } else {
            "needs_preparation"
        }
        .into(),
        record_count: 0,
        detail: if let Some(path) = export.as_ref() {
            format!("Unfold found an exported Apple Mail .mbox package at {}. Discovery checked only the package shape; message bodies are not read until you choose this source.", path.display())
        } else if installed {
            "Apple Mail is present. The end-user path is to export a mailbox from Apple Mail first, then choose the exported .mbox folder here. Unfold does not need Full Disk Access for exported mailboxes."
                .into()
        } else {
            "Apple Mail was not found at the standard app path. If you already have an exported Apple Mail .mbox folder, choose it here."
                .into()
        },
        next_step: if export.is_some() {
            "Use the found .mbox export, or choose a different Mail export if this is not the mailbox you want.".into()
        } else {
            "Export a mailbox in Apple Mail with Mailbox › Export Mailbox…, then choose the saved .mbox folder.".into()
        },
        privacy_note: "Mail bodies and correspondents are private. Exported .mbox import is explicit, read-only, local-only, and does not grant Unfold ongoing Mail.app access.".into(),
        evidence: if let Some(path) = export.as_ref() {
            vec![
                format!("found exported .mbox package {}", path.display()),
                "package shape only; message bodies not inspected".into(),
            ]
        } else if installed {
            vec!["Apple Mail export route available".into()]
        } else {
            Vec::new()
        },
    }
}

fn apple_mail_export_path(home: &Path) -> Option<PathBuf> {
    let prepared = home.join("Desktop").join("Amara Unfold Mail Exports");
    prepared
        .is_dir()
        .then(|| find_mail_export(&prepared, 0))
        .flatten()
}

fn find_mail_export(directory: &Path, depth: usize) -> Option<PathBuf> {
    if is_likely_mail_export(directory) {
        return Some(directory.to_path_buf());
    }
    if depth >= 6 {
        return None;
    }
    let mut entries = fs::read_dir(directory)
        .ok()?
        .flatten()
        .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            !matches!(
                name.as_ref(),
                ".git" | ".Trash" | "node_modules" | "target" | "dist" | "Library"
            ) && (!name.starts_with('.') || name == ".mbox")
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    entries.sort();
    entries
        .into_iter()
        .find_map(|path| find_mail_export(&path, depth + 1))
}

fn is_likely_mail_export(directory: &Path) -> bool {
    directory
        .extension()
        .and_then(|value| value.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("mbox"))
        .unwrap_or(false)
        && (directory.join("mbox").is_file() || directory.join("Messages").is_dir())
}

fn apple_music(home: &Path) -> LocalAppSource {
    let library = home.join("Music/Music/Music Library.musiclibrary");
    consent_required(
        LocalAppRoute {
            id: "apple-music",
            label: "Apple Music",
            tier: "Music",
            adapter_id: "apple-music-local-library",
            detail: "Unfold knows the standard Music.app library location, but selecting this route does not contact Music.app or inspect tracks. The one-time read starts only after you choose Create Markdown.",
            next_step: "Use the standard Music.app library location to preserve tracks, playlists, playlist membership, and track Date Added in Markdown. Timeline uses Date Added only.",
            privacy_note: "Music library data remains local; access is explicit, read-only, and used only for the export. Unfold stores no ongoing Music.app access; revoke macOS Music/Automation permission in System Settings after export.",
        },
        library,
    )
}

fn immich(_home: &Path, _include_user_locations: bool) -> LocalAppSource {
    LocalAppSource {
        id: "immich".into(),
        label: "Immich".into(),
        tier: "Planned".into(),
        provider: "Immich".into(),
        adapter_id: None,
        path: None,
        status: "needs_preparation".into(),
        record_count: 0,
        detail: "Immich is not available in this build. It is listed as a planned application route and cannot be selected.".into(),
        next_step: "Use another available source. Immich import remains unavailable until its end-user route has passed current installed-app UAT.".into(),
        privacy_note: "Immich can contain a complete private photo archive. This build does not inspect or read Immich locations.".into(),
        evidence: vec!["not inspected; route unavailable".into()],
    }
}

fn apple_messages(home: &Path, include_user_locations: bool) -> LocalAppSource {
    let path = home.join("Library/Messages/chat.db");
    found_pending(PendingLocalAppRoute {
        id: "apple-messages",
        label: "Apple Messages / iMessage",
        tier: "Explicit opt-in",
        provider: "Apple Messages",
        adapter_id: None,
        path,
        detail: "Messages is not available in this build. It is listed only as a privacy-sensitive source category, not as a working route.",
        next_step: "Use a ready source or an exported folder. This build does not import Messages.",
        privacy_note: "Very sensitive conversation data; this build never reads Messages.",
        inspect_user_path: include_user_locations,
    })
}

fn apple_notes(home: &Path, include_user_locations: bool) -> LocalAppSource {
    let path = home.join("Library/Group Containers/group.com.apple.notes");
    found_pending(PendingLocalAppRoute {
        id: "apple-notes",
        label: "Apple Notes",
        tier: "Explicit opt-in",
        provider: "Apple Notes",
        adapter_id: None,
        path,
        detail: "Notes is not available in this build. It is listed only as a privacy-sensitive source category, not as a working route.",
        next_step: "Use a ready source or an exported folder. This build does not import Notes.",
        privacy_note: "Notes may contain secrets and journal material; this build never reads Notes.",
        inspect_user_path: include_user_locations,
    })
}

fn apple_journal(home: &Path, include_user_locations: bool) -> LocalAppSource {
    let path = home.join("Library/Group Containers/group.com.apple.Journal");
    found_pending(PendingLocalAppRoute {
        id: "apple-journal",
        label: "Apple Journal",
        tier: "Explicit opt-in",
        provider: "Apple Journal",
        adapter_id: None,
        path,
        detail: "Journal is not available in this build. It is listed only as a privacy-sensitive source category, not as a working route.",
        next_step: "Use a ready source or an exported folder. This build does not import Journal.",
        privacy_note: "Journal entries are intimate memory data; this build never reads Journal.",
        inspect_user_path: include_user_locations,
    })
}

fn consent_required(route: LocalAppRoute<'_>, path: PathBuf) -> LocalAppSource {
    LocalAppSource {
        id: route.id.into(),
        label: route.label.into(),
        tier: route.tier.into(),
        provider: route.label.into(),
        adapter_id: Some(route.adapter_id.into()),
        evidence: vec!["not inspected until selected".into()],
        path: Some(path.to_string_lossy().into_owned()),
        status: "needs_permission".into(),
        record_count: 0,
        detail: route.detail.into(),
        next_step: route.next_step.into(),
        privacy_note: route.privacy_note.into(),
    }
}

fn consent_required_optional_path(
    route: LocalAppRoute<'_>,
    path: Option<PathBuf>,
    missing_detail: &str,
) -> LocalAppSource {
    LocalAppSource {
        id: route.id.into(),
        label: route.label.into(),
        tier: route.tier.into(),
        provider: route.label.into(),
        adapter_id: Some(route.adapter_id.into()),
        evidence: vec!["not inspected until selected".into()],
        path: path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        status: if path.is_some() {
            "needs_permission"
        } else {
            "needs_preparation"
        }
        .into(),
        record_count: 0,
        detail: if path.is_some() {
            route.detail.into()
        } else {
            missing_detail.into()
        },
        next_step: route.next_step.into(),
        privacy_note: route.privacy_note.into(),
    }
}

fn export_required(route: LocalAppRoute<'_>) -> LocalAppSource {
    LocalAppSource {
        id: route.id.into(),
        label: route.label.into(),
        tier: route.tier.into(),
        provider: route.label.into(),
        adapter_id: Some(route.adapter_id.into()),
        evidence: vec!["export or folder chosen by user".into()],
        path: None,
        status: "needs_preparation".into(),
        record_count: 0,
        detail: route.detail.into(),
        next_step: route.next_step.into(),
        privacy_note: route.privacy_note.into(),
    }
}

fn exported_folder_with_extension(
    home: &Path,
    folder_name: &str,
    extensions: &[&str],
) -> Option<PathBuf> {
    let folder = home.join("Desktop").join(folder_name);
    if folder.is_dir() && folder_has_extension(&folder, extensions) {
        Some(folder)
    } else {
        None
    }
}

fn folder_has_extension(folder: &Path, extensions: &[&str]) -> bool {
    let mut stack = vec![folder.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file()
                && path
                    .extension()
                    .and_then(|value| value.to_str())
                    .map(|extension| {
                        extensions
                            .iter()
                            .any(|expected| extension.eq_ignore_ascii_case(expected))
                    })
                    .unwrap_or(false)
            {
                return true;
            }
        }
    }
    false
}

fn found_pending(route: PendingLocalAppRoute<'_>) -> LocalAppSource {
    let exists = route.inspect_user_path && route.path.exists();
    LocalAppSource {
        id: route.id.into(),
        label: route.label.into(),
        tier: route.tier.into(),
        provider: route.provider.into(),
        adapter_id: route.adapter_id.map(str::to_owned),
        path: exists.then(|| route.path.to_string_lossy().into_owned()),
        status: if !route.inspect_user_path || exists {
            "needs_preparation"
        } else {
            "not_found"
        }
        .into(),
        record_count: 0,
        detail: if !route.inspect_user_path || exists {
            route.detail.into()
        } else {
            format!("{} was not found at the default local path.", route.label)
        },
        next_step: route.next_step.into(),
        privacy_note: route.privacy_note.into(),
        evidence: if exists {
            vec![format!("found {}", route.path.display())]
        } else if !route.inspect_user_path {
            vec!["not inspected on app start".into()]
        } else {
            Vec::new()
        },
    }
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

#[allow(dead_code)]
fn immich_source_path(home: &Path) -> Option<PathBuf> {
    if let Some(path) = immich_upload_location(home) {
        if immich_original_media_available(&path) {
            return Some(path);
        }
    }
    for path in immich_external_library_locations(home) {
        if immich_original_media_available(&path) {
            return Some(path);
        }
    }
    [
        home.join("immich/library/upload"),
        home.join("immich/library/library"),
        home.join("immich/library"),
    ]
    .into_iter()
    .find(|candidate| immich_original_media_available(candidate))
}

#[allow(dead_code)]
fn immich_likely_default_path(home: &Path) -> Option<PathBuf> {
    if let Some(path) = immich_upload_location(home) {
        if path.is_dir() && immich_marker_present(&path) {
            return Some(path);
        }
    }
    [
        home.join("immich/library/upload"),
        home.join("immich/library/library"),
        home.join("immich/library"),
    ]
    .into_iter()
    .find(|candidate| candidate.is_dir() && immich_marker_present(candidate))
}

#[allow(dead_code)]
fn immich_marker_present(path: &Path) -> bool {
    path.join(".immich").is_file()
}

fn immich_external_library_locations(home: &Path) -> Vec<PathBuf> {
    let compose_file = home.join("immich/docker-compose.yml");
    let Ok(content) = fs::read_to_string(compose_file) else {
        return Vec::new();
    };
    let env_vars = immich_env_vars(home);
    content
        .lines()
        .filter_map(|line| immich_external_volume_host_path_with_env(home, line, &env_vars))
        .collect()
}

#[cfg(test)]
fn immich_external_volume_host_path(home: &Path, line: &str) -> Option<PathBuf> {
    immich_external_volume_host_path_with_env(home, line, &HashMap::new())
}

fn immich_external_volume_host_path_with_env(
    home: &Path,
    line: &str,
    env_vars: &HashMap<String, String>,
) -> Option<PathBuf> {
    let line = line.trim();
    let volume = line
        .strip_prefix("- ")
        .map(str::trim)
        .unwrap_or(line)
        .trim_matches('"')
        .trim_matches('\'');
    let (host, rest) = volume.split_once(':')?;
    if !rest.contains("/external/") {
        return None;
    }
    let host = expand_immich_env_value(host.trim(), env_vars)?;
    if host.is_empty() || host.starts_with('$') {
        return None;
    }
    Some(resolve_user_path(home, &host))
}

fn immich_upload_location(home: &Path) -> Option<PathBuf> {
    immich_env_vars(home)
        .remove("UPLOAD_LOCATION")
        .filter(|value| !value.is_empty())
        .map(|value| resolve_user_path(home, &value))
}

fn immich_env_vars(home: &Path) -> HashMap<String, String> {
    let env_file = home.join("immich/.env");
    let Ok(content) = fs::read_to_string(env_file) else {
        return HashMap::new();
    };
    let mut values = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        let value = value.trim().trim_matches('"').trim_matches('\'');
        values.insert(key.to_owned(), value.to_owned());
    }
    values
}

fn expand_immich_env_value(value: &str, env_vars: &HashMap<String, String>) -> Option<String> {
    if let Some(rest) = value.strip_prefix("${") {
        let (name, suffix) = rest.split_once('}')?;
        return env_vars
            .get(name)
            .map(|resolved| format!("{resolved}{suffix}"));
    }
    if let Some(rest) = value.strip_prefix('$') {
        let name_len = rest
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
            .map(char::len_utf8)
            .sum::<usize>();
        if name_len == 0 {
            return None;
        }
        let (name, suffix) = rest.split_at(name_len);
        return env_vars
            .get(name)
            .map(|resolved| format!("{resolved}{suffix}"));
    }
    Some(value.to_owned())
}

fn resolve_user_path(home: &Path, value: &str) -> PathBuf {
    if let Some(rest) = value.strip_prefix("~/") {
        return home.join(rest);
    }
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        home.join("immich").join(path)
    }
}

fn immich_original_media_available(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }
    let upload = path.join("upload");
    let library = path.join("library");
    let roots = [upload.as_path(), library.as_path()]
        .into_iter()
        .filter(|candidate| candidate.is_dir())
        .collect::<Vec<_>>();
    if roots.is_empty() {
        has_media_candidate(path)
    } else {
        roots.into_iter().any(has_media_candidate)
    }
}

fn has_media_candidate(path: &Path) -> bool {
    let mut stack = vec![path.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                stack.push(entry.path());
            } else if file_type.is_file() && is_media_extension(&entry.path()) {
                return true;
            }
        }
    }
    false
}

fn is_media_extension(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
            .as_deref(),
        Some("jpg" | "jpeg" | "heic" | "png" | "mov" | "mp4" | "webp" | "dng" | "tif" | "tiff")
    )
}

#[cfg(test)]
mod tests {
    use super::{
        apple_calendar, apple_contacts, apple_mail_export_path, apple_photos,
        immich_external_volume_host_path, immich_original_media_available, immich_source_path,
    };

    #[test]
    fn apple_photos_is_export_folder_route_not_private_library_route() {
        let source = apple_photos();

        assert_eq!(source.adapter_id.as_deref(), Some("photo-media-folder"));
        assert_eq!(source.status, "needs_preparation");
        assert!(source.path.is_none());
        assert!(source.detail.contains("export originals"));
    }

    #[test]
    fn calendar_and_contacts_are_export_routes_on_startup() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join("Library/Calendars")).unwrap();
        std::fs::create_dir_all(home.path().join("Library/Application Support/AddressBook"))
            .unwrap();

        let calendar = apple_calendar(home.path(), true);
        let contacts = apple_contacts(home.path(), false);

        assert_eq!(calendar.adapter_id.as_deref(), Some("apple-calendar-local"));
        assert_eq!(calendar.status, "needs_preparation");
        assert!(calendar.path.is_none());
        assert!(calendar.detail.contains("exported .ics"));
        assert_eq!(contacts.adapter_id.as_deref(), Some("apple-contacts-local"));
        assert_eq!(contacts.status, "needs_preparation");
        assert!(contacts.path.is_none());
        assert!(contacts.detail.contains("exported vCards"));
    }

    #[test]
    fn calendar_and_contacts_exports_are_detected_after_user_prepares_folder() {
        let home = tempfile::tempdir().unwrap();
        let calendar_export = home.path().join("Desktop/Amara Unfold Calendar Exports");
        let contacts_export = home.path().join("Desktop/Amara Unfold Contacts Exports");
        std::fs::create_dir_all(&calendar_export).unwrap();
        std::fs::create_dir_all(&contacts_export).unwrap();
        std::fs::write(calendar_export.join("birthdays.ics"), b"BEGIN:VCALENDAR\n").unwrap();
        std::fs::write(contacts_export.join("people.vcf"), b"BEGIN:VCARD\n").unwrap();

        let calendar = apple_calendar(home.path(), true);
        let contacts = apple_contacts(home.path(), true);

        assert_eq!(calendar.status, "needs_permission");
        assert_eq!(
            calendar.path.as_deref(),
            Some(calendar_export.to_str().unwrap())
        );
        assert_eq!(contacts.status, "needs_permission");
        assert_eq!(
            contacts.path.as_deref(),
            Some(contacts_export.to_str().unwrap())
        );
    }

    #[test]
    fn immich_generated_media_does_not_make_source_ready() {
        let home = tempfile::tempdir().unwrap();
        let root = home.path().join("immich/library");
        std::fs::create_dir_all(root.join("thumbs/user/aa")).unwrap();
        std::fs::create_dir_all(root.join("encoded-video/user")).unwrap();
        std::fs::create_dir_all(root.join("upload")).unwrap();
        std::fs::create_dir_all(root.join("library")).unwrap();
        std::fs::write(root.join("thumbs/user/aa/preview.jpg"), b"generated").unwrap();
        std::fs::write(root.join("encoded-video/user/clip.mp4"), b"generated").unwrap();

        assert!(!immich_original_media_available(&root));
        assert_eq!(immich_source_path(home.path()), None);
    }

    #[test]
    fn immich_original_upload_media_makes_source_ready() {
        let home = tempfile::tempdir().unwrap();
        let root = home.path().join("immich/library");
        std::fs::create_dir_all(root.join("upload/user")).unwrap();
        std::fs::create_dir_all(root.join("thumbs/user")).unwrap();
        std::fs::write(root.join("upload/user/photo.jpg"), b"original").unwrap();
        std::fs::write(root.join("thumbs/user/preview.jpg"), b"generated").unwrap();

        assert!(immich_original_media_available(&root));
        assert_eq!(immich_source_path(home.path()), Some(root.join("upload")));
    }

    #[test]
    fn immich_external_readonly_mount_is_detected() {
        let home = tempfile::tempdir().unwrap();
        let external = home.path().join("Pictures/Immich Originals");
        std::fs::create_dir_all(&external).unwrap();
        std::fs::write(external.join("photo.jpg"), b"original").unwrap();
        std::fs::create_dir_all(home.path().join("immich")).unwrap();
        std::fs::write(
            home.path().join("immich/docker-compose.yml"),
            format!(
                "services:\n  immich-server:\n    volumes:\n      - {}:/external/T9-DigiPics:ro\n",
                external.display()
            ),
        )
        .unwrap();

        assert_eq!(immich_source_path(home.path()), Some(external));
    }

    #[test]
    fn immich_external_mount_wins_when_upload_location_has_no_originals() {
        let home = tempfile::tempdir().unwrap();
        let upload = home.path().join("immich/library");
        std::fs::create_dir_all(upload.join("upload")).unwrap();
        std::fs::create_dir_all(upload.join("library")).unwrap();
        std::fs::write(upload.join("upload/.immich"), b"marker").unwrap();
        std::fs::write(upload.join("library/.immich"), b"marker").unwrap();

        let external = home.path().join("T9/Sorted/DigiPics");
        std::fs::create_dir_all(external.join("2023/03/24")).unwrap();
        std::fs::write(external.join("2023/03/24/photo.jpg"), b"original").unwrap();
        std::fs::create_dir_all(home.path().join("immich")).unwrap();
        std::fs::write(
            home.path().join("immich/.env"),
            format!("UPLOAD_LOCATION={}\n", upload.display()),
        )
        .unwrap();
        std::fs::write(
            home.path().join("immich/docker-compose.yml"),
            format!(
                "services:\n  immich-server:\n    volumes:\n      - ${{UPLOAD_LOCATION}}:/data\n      - {}:/external/T9-DigiPics:ro\n",
                external.display()
            ),
        )
        .unwrap();

        assert_eq!(immich_source_path(home.path()), Some(external));
    }

    #[test]
    fn immich_external_mount_resolves_env_var_host_path() {
        let home = tempfile::tempdir().unwrap();
        let external = home.path().join("Pictures/Immich Originals");
        std::fs::create_dir_all(external.join("2024")).unwrap();
        std::fs::write(external.join("2024/photo.heic"), b"original").unwrap();
        std::fs::create_dir_all(home.path().join("immich")).unwrap();
        std::fs::write(
            home.path().join("immich/.env"),
            format!("EXTERNAL_LIBRARY_LOCATION={}\n", external.display()),
        )
        .unwrap();
        std::fs::write(
            home.path().join("immich/docker-compose.yml"),
            "services:\n  immich-server:\n    volumes:\n      - ${EXTERNAL_LIBRARY_LOCATION}:/external/photos:ro\n",
        )
        .unwrap();

        assert_eq!(immich_source_path(home.path()), Some(external));
    }

    #[test]
    fn immich_external_mount_resolves_env_var_host_path_with_suffix() {
        let home = tempfile::tempdir().unwrap();
        let root = home.path().join("Pictures");
        let external = root.join("Immich Originals");
        std::fs::create_dir_all(&external).unwrap();
        std::fs::write(external.join("video.mov"), b"original").unwrap();
        std::fs::create_dir_all(home.path().join("immich")).unwrap();
        std::fs::write(
            home.path().join("immich/.env"),
            format!("PHOTO_ROOT={}\n", root.display()),
        )
        .unwrap();
        std::fs::write(
            home.path().join("immich/docker-compose.yml"),
            "services:\n  immich-server:\n    volumes:\n      - $PHOTO_ROOT/Immich Originals:/external/photos:ro\n",
        )
        .unwrap();

        assert_eq!(immich_source_path(home.path()), Some(external));
    }

    #[test]
    fn immich_card_remains_unavailable_when_originals_folder_is_found() {
        let home = tempfile::tempdir().unwrap();
        let root = home.path().join("immich/library/upload/user");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("photo.jpg"), b"original").unwrap();

        let source = super::immich(home.path(), true);

        assert_eq!(source.status, "needs_preparation");
        assert_eq!(source.adapter_id, None);
        assert_eq!(source.path, None);
        assert!(source.detail.contains("not available in this build"));
        assert!(source.privacy_note.contains("does not inspect"));
    }

    #[test]
    fn immich_card_does_not_surface_a_default_marker() {
        let home = tempfile::tempdir().unwrap();
        let upload = home.path().join("immich/library/upload");
        std::fs::create_dir_all(&upload).unwrap();
        std::fs::write(upload.join(".immich"), b"marker").unwrap();

        let source = super::immich(home.path(), true);

        assert_eq!(source.status, "needs_preparation");
        assert_eq!(source.adapter_id, None);
        assert_eq!(source.path, None);
        assert!(source.detail.contains("not available in this build"));
    }

    #[test]
    fn apple_mail_card_is_export_first_without_default_store_path() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join("Library/Mail")).unwrap();

        let source = super::apple_mail(home.path(), false);

        assert_eq!(source.status, "needs_preparation");
        assert_eq!(source.adapter_id.as_deref(), Some("apple-mail-local"));
        assert_eq!(source.path, None);
        assert!(source.next_step.contains("Export Mailbox"));
        assert!(source.privacy_note.contains("ongoing Mail.app access"));
    }

    #[test]
    fn launch_listing_does_not_probe_user_locations() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join("Pictures/Photos Library.photoslibrary")).unwrap();
        std::fs::create_dir_all(
            home.path()
                .join("Desktop/Amara Unfold Mail Exports/Inbox.mbox"),
        )
        .unwrap();
        std::fs::write(
            home.path()
                .join("Desktop/Amara Unfold Mail Exports/Inbox.mbox/mbox"),
            b"From sender@example.test\n",
        )
        .unwrap();
        std::fs::create_dir_all(home.path().join("Desktop/Amara Unfold Calendar Exports")).unwrap();
        std::fs::write(
            home.path()
                .join("Desktop/Amara Unfold Calendar Exports/birthdays.ics"),
            b"BEGIN:VCALENDAR\n",
        )
        .unwrap();
        std::fs::create_dir_all(home.path().join("Desktop/Amara Unfold Contacts Exports")).unwrap();
        std::fs::write(
            home.path()
                .join("Desktop/Amara Unfold Contacts Exports/people.vcf"),
            b"BEGIN:VCARD\n",
        )
        .unwrap();
        std::fs::create_dir_all(home.path().join("immich/library/upload/user")).unwrap();
        std::fs::write(
            home.path().join("immich/library/upload/user/photo.jpg"),
            b"original",
        )
        .unwrap();

        let sources = [
            super::apple_photos(),
            super::apple_calendar(home.path(), false),
            super::apple_contacts(home.path(), false),
            super::apple_mail(home.path(), false),
            super::immich(home.path(), false),
            super::apple_messages(home.path(), false),
            super::apple_notes(home.path(), false),
            super::apple_journal(home.path(), false),
        ];

        assert!(sources.iter().all(|source| source.path.is_none()));
        assert!(sources.iter().all(|source| !source
            .evidence
            .iter()
            .any(|item| item.starts_with("found "))));
    }

    #[test]
    fn apple_mail_export_discovery_is_explicit_and_finds_prepared_mbox_shape_only() {
        let home = tempfile::tempdir().unwrap();
        let export = home
            .path()
            .join("Desktop/Amara Unfold Mail Exports/Inbox.mbox");
        std::fs::create_dir_all(&export).unwrap();
        std::fs::write(export.join("mbox"), b"From sender@example.test\n").unwrap();

        let quiet_source = super::apple_mail(home.path(), false);
        let source = super::apple_mail(home.path(), true);

        assert_eq!(quiet_source.status, "needs_preparation");
        assert_eq!(quiet_source.path, None);
        assert_eq!(apple_mail_export_path(home.path()), Some(export.clone()));
        assert_eq!(source.status, "ready");
        assert_eq!(
            source.path.as_deref(),
            Some(export.to_string_lossy().as_ref())
        );
        assert!(source.detail.contains("package shape"));
        assert!(source
            .evidence
            .iter()
            .any(|item| item.contains("not inspected")));
    }

    #[test]
    fn apple_mail_export_discovery_does_not_use_live_mail_store() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join("Library/Mail/V10/Inbox.mbox")).unwrap();
        std::fs::write(
            home.path().join("Library/Mail/V10/Inbox.mbox/mbox"),
            b"raw store should stay ignored",
        )
        .unwrap();

        assert_eq!(apple_mail_export_path(home.path()), None);
        let source = super::apple_mail(home.path(), true);
        assert_eq!(source.status, "needs_preparation");
        assert_eq!(source.path, None);
    }

    #[test]
    fn immich_external_volume_parser_ignores_non_external_mounts() {
        let home = tempfile::tempdir().unwrap();

        assert_eq!(
            immich_external_volume_host_path(home.path(), "      - /tmp/immich-model-cache:/cache"),
            None
        );
        assert_eq!(
            immich_external_volume_host_path(
                home.path(),
                "      - ~/Pictures/Immich:/external/photos:ro"
            ),
            Some(home.path().join("Pictures/Immich"))
        );
    }
}
