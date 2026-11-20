//! Portable transport containers, separate from the native document schema.
//! `project.pack`: path, output, optional recovery/allowed_creator and
//! allow_external_media (false by default). `project.unpack`: path, output and
//! optional max_expanded_bytes/max_archive_bytes that may lower native caps.
//! Both operations publish a new destination and never replace an existing one.

use std::{collections::{BTreeMap, BTreeSet}, ffi::OsString, fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write}, path::{Component, Path, PathBuf}};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use verdant_domain::{Project, ProjectStore};
use verdant_media::{AssetPolicy, AudioSource, ImportLimits, MediaError, MediaImporter};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

use crate::CliError;

const FORMAT: &str = "verdant-portable-project/1";
const DOCUMENT: &str = "project.verp";
const MANIFEST: &str = "manifest.json";
const MAX_ENTRIES: usize = 4096;
const MAX_PROJECT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_ASSET_BYTES: u64 = 512 * 1024 * 1024;
const MAX_EXPANDED_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_ARCHIVE_BYTES: u64 = MAX_EXPANDED_BYTES + 8 * 1024 * 1024;
const MAX_DIRECTORY_BYTES: u64 = 2 * 1024 * 1024;
const MAX_RATIO: u64 = 200;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PackParams {
    path: PathBuf,
    output: PathBuf,
    recovery: Option<std::string::String>,
    allowed_creator: Option<std::string::String>,
    #[serde(default)]
    allow_external_media: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UnpackParams {
    path: PathBuf,
    output: PathBuf,
    max_expanded_bytes: Option<u64>,
    max_archive_bytes: Option<u64>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentEntry { path: String, sha256: String, byte_len: u64 }

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format: String,
    project_id: String,
    revision: u64,
    project: DocumentEntry,
    assets: Vec<AudioSource>,
}

pub fn dispatch(op: &str, params: Value) -> Option<Result<Value, CliError>> {
    Some(match op {
        "project.pack" => serde_json::from_value(params).map_err(argument).and_then(pack),
        "project.unpack" => serde_json::from_value(params).map_err(argument).and_then(unpack),
        _ => return None,
    })
}

fn pack(params: PackParams) -> Result<Value, CliError> {
    let output = zip_output(&params.output)?;
    let parent = OutputParent::new(&output)?;
    parent.ensure_absent()?;
    let mut project = crate::project::read_snapshot(&params.path, params.recovery.as_deref(), params.allowed_creator.as_deref())?;
    let mut policy = verdant_service::media::project_policy(&params.path, AssetPolicy::new([]).map_err(media)?)
        .map_err(media)?;
    let mut sources = BTreeMap::<String, AudioSource>::new();
    let mut hashes = BTreeMap::<String, String>::new();
    for track in &mut project.tracks {
        let mut clips = verdant_media::read_clips(track).map_err(media)?;
        for clip in &mut clips {
            let source = &clip.source;
            if source.byte_len > MAX_ASSET_BYTES { return Err(resource("audio asset exceeds 512 MiB")); }
            if Path::new(&source.path).is_absolute() {
                if !params.allow_external_media {
                    return Err(CliError::new("DECISION_REQUIRED", "packing absolute media requires allow_external_media=true")
                        .with_details(json!({"track_id":track.id,"clip_id":clip.id,"sha256":source.sha256})));
                }
                policy.allow_file(Path::new(&source.path)).map_err(media)?;
            }
            let name = if let Some(name) = hashes.get(&source.sha256) {
                if !same_source(source, &sources[name]) {
                    return Err(invalid("audio sources with the same digest disagree about their identity"));
                }
                name.clone()
            } else {
                let extension = Path::new(&source.path).extension().and_then(|extension| extension.to_str())
                    .map(str::to_ascii_lowercase).filter(|extension| audio_extension(extension))
                    .unwrap_or_else(|| "audio".into());
                let name = format!("assets/{}.{}", source.sha256, extension);
                hashes.insert(source.sha256.clone(), name.clone());
                sources.insert(name.clone(), source.clone());
                name
            };
            clip.source.path = name;
        }
        verdant_media::write_clips(track, &clips).map_err(media)?;
    }
    if sources.len() + 2 > MAX_ENTRIES { return Err(resource("portable package exceeds 4096 members")); }
    let mut document = serde_json::to_value(&project).map_err(internal)?;
    let removed_metadata_fields = scrub_metadata(&mut document);
    let mut document = serde_json::to_vec_pretty(&document).map_err(internal)?;
    document.push(b'\n');
    if document.len() as u64 > MAX_PROJECT_BYTES { return Err(resource("project.verp exceeds 64 MiB")); }
    let manifest = Manifest {
        format: FORMAT.into(), project_id: project.project_id.clone(), revision: project.revision,
        project: DocumentEntry { path: DOCUMENT.into(), sha256: sha256(&document), byte_len: document.len() as u64 },
        assets: sources.iter().map(|(name, source)| { let mut source = source.clone(); source.path.clone_from(name); source }).collect(),
    };
    // Re-parse the transport document through the native reader, and validate
    // references before writing any member. Identity and musical edits survive.
    let transported = parse_document(&document)?;
    validate_project(transported.project(), &manifest)?;
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(internal)?;
    if manifest_bytes.len() as u64 > MAX_MANIFEST_BYTES { return Err(resource("manifest.json exceeds 1 MiB")); }
    let expanded = sources.values().try_fold(document.len() as u64 + manifest_bytes.len() as u64, |total, source|
        total.checked_add(source.byte_len).filter(|total| *total <= MAX_EXPANDED_BYTES)
            .ok_or_else(|| resource("portable package exceeds 2 GiB expanded")))?;
    let mut temporary = tempfile::Builder::new().prefix(".verdant-pack-").tempfile_in(parent.bound_path()).map_err(io)?;
    let importer = MediaImporter::new(policy, ImportLimits { max_source_bytes: MAX_ASSET_BYTES, ..Default::default() });
    {
        let mut archive = ZipWriter::new(temporary.as_file_mut());
        // Stored members avoid compression work on already-compressed media
        // and ensure every package we produce satisfies the expansion ratio cap.
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored).unix_permissions(0o600);
        archive.start_file(DOCUMENT, options).map_err(zip_error)?;
        archive.write_all(&document).map_err(io)?;
        archive.start_file(MANIFEST, options).map_err(zip_error)?;
        archive.write_all(&manifest_bytes).map_err(io)?;
        for (name, source) in &sources {
            archive.start_file(name, options).map_err(zip_error)?;
            verdant_service::media::copy_source(source, &importer, &mut archive, MAX_ASSET_BYTES).map_err(media)?;
        }
        archive.finish().map_err(zip_error)?;
    }
    temporary.as_file().sync_all().map_err(io)?;
    let archive_bytes = temporary.as_file().metadata().map_err(io)?.len();
    if archive_bytes > MAX_ARCHIVE_BYTES { return Err(resource("portable ZIP exceeds its archive byte limit")); }
    let archive_sha256 = hash_file(temporary.as_file_mut())?;
    parent.check_identity()?;
    temporary.persist_noclobber(parent.destination()).map_err(|error| io(error.error))?;
    parent.sync()?;
    Ok(json!({"output":parent.public_destination(),"format":FORMAT,"project_id":project.project_id,
        "revision":project.revision,"project":DOCUMENT,"sha256":archive_sha256,"byte_len":archive_bytes,
        "asset_count":manifest.assets.len(),"expanded_bytes":expanded,"removed_metadata_fields":removed_metadata_fields,
        "limits":limits(MAX_EXPANDED_BYTES, MAX_ARCHIVE_BYTES)}))
}

fn unpack(params: UnpackParams) -> Result<Value, CliError> {
    let max_expanded = lowered_limit(params.max_expanded_bytes, MAX_EXPANDED_BYTES, "max_expanded_bytes")?;
    let max_archive = lowered_limit(params.max_archive_bytes, MAX_ARCHIVE_BYTES, "max_archive_bytes")?;
    let parent = OutputParent::new(&params.output)?;
    parent.ensure_absent()?;
    let path = fs::canonicalize(&params.path).map_err(io)?;
    let policy = AssetPolicy::for_file(&path).map_err(media)?;
    let mut input = policy.open_file(&path).map_err(media)?;
    let declared_count = preflight_zip(&mut input, max_archive)?;
    let mut archive = ZipArchive::new(input).map_err(zip_error)?;
    if archive.len() != declared_count || !(2..=MAX_ENTRIES).contains(&archive.len()) {
        return Err(invalid("ZIP member count does not match the admitted directory"));
    }
    let mut members = BTreeMap::<String, usize>::new();
    let mut total = 0u64;
    for index in 0..archive.len() {
        let member = archive.by_index(index).map_err(zip_error)?;
        let name = member.name();
        let limit = match name {
            DOCUMENT => MAX_PROJECT_BYTES,
            MANIFEST => MAX_MANIFEST_BYTES,
            name if asset_name(name) => MAX_ASSET_BYTES,
            _ => return Err(invalid("unexpected or nonportable ZIP member")),
        };
        if member.name_raw() != name.as_bytes() || member.enclosed_name().is_none() || !member.is_file() || member.encrypted()
            || member.unix_mode().is_some_and(|mode| !matches!(mode & 0o170000, 0 | 0o100000))
            || !matches!(member.compression(), CompressionMethod::Stored | CompressionMethod::Deflated) {
            return Err(invalid("ZIP members must be unencrypted regular stored/deflated files"));
        }
        if members.insert(name.to_owned(), index).is_some() { return Err(invalid("duplicate ZIP member")); }
        if member.size() > limit || member.size() > member.compressed_size().saturating_mul(MAX_RATIO) {
            return Err(resource("ZIP member exceeds its byte or 200x expansion limit"));
        }
        total = total.checked_add(member.size()).filter(|total| *total <= max_expanded)
            .ok_or_else(|| resource("ZIP exceeds its admitted expanded-byte limit"))?;
    }
    let manifest_index = *members.get(MANIFEST).ok_or_else(|| invalid("missing manifest.json"))?;
    let document_index = *members.get(DOCUMENT).ok_or_else(|| invalid("missing project.verp"))?;
    let manifest_bytes = read_member(&mut archive, manifest_index, MAX_MANIFEST_BYTES)?;
    let manifest: Manifest = serde_json::from_slice(&manifest_bytes).map_err(argument)?;
    validate_manifest(&manifest)?;
    if manifest.assets.len() + 2 != members.len() { return Err(invalid("manifest does not enumerate exactly the ZIP members")); }
    let document = read_member(&mut archive, document_index, MAX_PROJECT_BYTES)?;
    if document.len() as u64 != manifest.project.byte_len || sha256(&document) != manifest.project.sha256 {
        return Err(invalid("project.verp does not match its manifest SHA-256 and length"));
    }
    let store = parse_document(&document)?;
    let project = store.project();
    validate_project(project, &manifest)?;
    let temporary = tempfile::Builder::new().prefix(".verdant-unpack-").tempdir_in(parent.bound_path()).map_err(io)?;
    let assets = temporary.path().join("assets");
    if !manifest.assets.is_empty() { fs::create_dir(&assets).map_err(io)?; }
    let verifier = MediaImporter::new(AssetPolicy::new([]).map_err(media)?, ImportLimits::default());
    for source in &manifest.assets {
        let index = *members.get(&source.path).ok_or_else(|| invalid("manifest asset is missing from the ZIP"))?;
        let bytes = read_member(&mut archive, index, source.byte_len)?;
        if bytes.len() as u64 != source.byte_len { return Err(invalid("asset length differs from the manifest")); }
        let mut target = new_file(&temporary.path().join(&source.path))?;
        target.write_all(&bytes).map_err(io)?;
        verifier.verify_bytes(source, bytes).map_err(media)?;
        target.sync_all().map_err(io)?;
    }
    write_new(&temporary.path().join(DOCUMENT), &document)?;
    write_new(&temporary.path().join(MANIFEST), &manifest_bytes)?;
    if !manifest.assets.is_empty() { sync_directory(&assets)?; }
    sync_directory(temporary.path())?;
    parent.check_identity()?;
    publish_directory(&parent, temporary.path())?;
    // Disarm cleanup after publication so a reused staging name can never be
    // mistaken for this operation's private directory by TempDir's destructor.
    let _ = temporary.keep();
    parent.sync()?;
    Ok(json!({"output":parent.public_destination(),"path":parent.public_destination().join(DOCUMENT),
        "format":FORMAT,"project_id":project.project_id,"revision":project.revision,
        "asset_count":manifest.assets.len(),"expanded_bytes":total,"verified":true,"limits":limits(max_expanded, max_archive)}))
}

fn validate_manifest(manifest: &Manifest) -> Result<(), CliError> {
    if manifest.format != FORMAT || manifest.project.path != DOCUMENT || manifest.project_id.is_empty()
        || manifest.project.byte_len == 0 || manifest.project.byte_len > MAX_PROJECT_BYTES
        || !valid_hash(&manifest.project.sha256) || manifest.assets.len() + 2 > MAX_ENTRIES {
        return Err(invalid("unsupported or invalid portable-project manifest"));
    }
    let mut names = BTreeSet::new();
    let mut hashes = BTreeSet::new();
    for source in &manifest.assets {
        source.validate().map_err(media)?;
        if source.byte_len > MAX_ASSET_BYTES || !asset_name(&source.path)
            || source.path.strip_prefix("assets/").and_then(|name| name.split_once('.')).is_none_or(|(hash, _)| hash != source.sha256)
            || !names.insert(&source.path) || !hashes.insert(&source.sha256) {
            return Err(invalid("invalid, duplicate or nonportable manifest asset"));
        }
    }
    Ok(())
}

fn validate_project(project: &Project, manifest: &Manifest) -> Result<(), CliError> {
    validate_manifest(manifest)?;
    if project.project_id != manifest.project_id || project.revision != manifest.revision {
        return Err(invalid("project identity or revision differs from the manifest"));
    }
    let expected: BTreeMap<_, _> = manifest.assets.iter().map(|source| (source.path.as_str(), source)).collect();
    let mut used = BTreeSet::new();
    let mut ids = BTreeSet::new();
    for track in &project.tracks {
        for clip in verdant_media::read_clips(track).map_err(media)? {
            if !ids.insert(clip.id) { return Err(invalid("duplicate project audio clip identity")); }
            if expected.get(clip.source.path.as_str()).is_none_or(|source| **source != clip.source) {
                return Err(invalid("project media reference is not an exact portable manifest asset"));
            }
            used.insert(clip.source.path);
        }
    }
    if used.len() != expected.len() { return Err(invalid("manifest includes an unreferenced asset")); }
    Ok(())
}

fn parse_document(bytes: &[u8]) -> Result<ProjectStore, CliError> {
    let text = std::str::from_utf8(bytes).map_err(|_| invalid("project.verp must be UTF-8"))?;
    let (project, _) = verdant_domain::io::parse_project(text).map_err(|error| invalid(error.to_string()))?;
    let store = ProjectStore::new(project);
    store.validate_document().map_err(|error| invalid(error.to_string()))?;
    Ok(store)
}

fn same_source(a: &AudioSource, b: &AudioSource) -> bool {
    a.sha256 == b.sha256 && a.byte_len == b.byte_len && a.sample_rate == b.sample_rate
        && a.channels == b.channels && a.frames == b.frames
}

fn audio_extension(extension: &str) -> bool {
    matches!(extension, "wav" | "wave" | "flac" | "mp3" | "ogg" | "oga" | "m4a" | "mp4" | "aac" | "aif" | "aiff" | "aifc" | "caf" | "audio")
}

fn valid_hash(hash: &str) -> bool {
    hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn asset_name(name: &str) -> bool {
    let Some((hash, extension)) = name.strip_prefix("assets/").and_then(|name| name.split_once('.')) else { return false; };
    valid_hash(hash) && audio_extension(extension)
}

/// Strip host hints and transport credentials from the serialized copy only.
/// Domain text/identities and opaque plugin state retain their authored values;
/// plugin installation paths are optional catalog hints and may be omitted.
fn scrub_metadata(value: &mut Value) -> usize {
    scrub_at(value, MetadataScope::Project)
}

#[derive(Clone, Copy)]
enum MetadataScope { Project, Track, Note, Reading, Voices, Voice, Plugin, Other }

fn scrub_at(value: &mut Value, scope: MetadataScope) -> usize {
    match value {
        Value::Object(fields) => {
            if matches!(scope, MetadataScope::Voices) {
                return fields.values_mut().map(|value| scrub_at(value, MetadataScope::Voice)).sum();
            }
            let mut removed = 0;
            fields.retain(|key, value| {
                if private_metadata_key(key) || unsafe_metadata_text(key) { removed += 1; return false; }
                let authored = matches!(key.as_str(), "id" | "instance_id" | "lyric" | "input_text" | "state_b64") && value.is_string()
                    || matches!((scope, key.as_str()),
                        (MetadataScope::Project, "project_id" | "format" | "version" | "status")
                        | (MetadataScope::Track, "name" | "voice_ref")
                        | (MetadataScope::Voice, "name" | "display_name")
                        | (MetadataScope::Plugin, "format" | "name" | "version")
                        | (MetadataScope::Reading, "pinyin" | "phonemes" | "language" | "alphabet" | "source" | "frontend_profile"));
                if authored {
                    return true;
                }
                if value.as_str().is_some_and(unsafe_metadata_text) { removed += 1; return false; }
                let next = match (scope, key.as_str()) {
                    (MetadataScope::Project, "tracks") => MetadataScope::Track,
                    (MetadataScope::Project, "voice_refs") => MetadataScope::Voices,
                    (MetadataScope::Track, "notes") => MetadataScope::Note,
                    (MetadataScope::Note, "pronunciation") => MetadataScope::Reading,
                    (_, "inserts" | "master_inserts") => MetadataScope::Plugin,
                    _ => MetadataScope::Other,
                };
                removed += scrub_at(value, next);
                true
            });
            removed
        }
        Value::Array(values) => {
            let before = values.len();
            values.retain(|value| !value.as_str().is_some_and(unsafe_metadata_text));
            before - values.len() + values.iter_mut().map(|value| scrub_at(value, scope)).sum::<usize>()
        }
        _ => 0,
    }
}

fn private_metadata_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase().replace(['-', '.'], "_");
    private_normalized_key(&lower)
}

fn private_normalized_key(lower: &str) -> bool {
    !lower.ends_with("_sha256") && (lower.split('_').any(|part|
        matches!(part, "password" | "secret" | "credential" | "credentials" | "token" | "authorization" | "cookie"
            | "cookies" | "auth" | "authentication" | "headers" | "server" | "session" | "requester"
            | "recipient" | "workspace" | "download" | "dsh"))
        || ["api_key", "apikey", "private_key", "privatekey", "password", "secret", "token"].iter()
            .any(|suffix| lower.ends_with(*suffix))
        || matches!(lower, "sessionid" | "media_authorizations" | "asset_roots" | "artifact_url" | "endpoint"
            | "weights" | "tensors" | "model_binary" | "token_binary" | "bank_data" | "voice_package"
            | "voice_assets" | "voice_registry" | "worker" | "cache"))
}

fn unsafe_metadata_text(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    if lower.contains("http:") || lower.contains("https:") {
        let Ok(url) = url::Url::parse(text.trim()) else { return true; };
        return private_url(&url);
    }
    lower.contains("file:") || lower.contains("ssh:")
        || lower.contains("bearer ") || lower.contains("-----begin ")
        || text.split(|character: char| character.is_whitespace() || matches!(character, '=' | ':' | '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}'))
            .any(|part| part.starts_with('/') || part.starts_with('\\') || part.starts_with("../") || part.starts_with("..\\")
                || part.starts_with("~/"))
}

fn private_url(url: &url::Url) -> bool {
    if !matches!(url.scheme(), "http" | "https") || !url.username().is_empty() || url.password().is_some()
        || url.query_pairs().any(|(key, _)| {
            let key = key.to_ascii_lowercase().replace(['-', '.'], "_");
            private_normalized_key(&key) || matches!(key.as_str(), "key" | "code" | "sig" | "signature" | "expires" | "policy")
                || key.ends_with("_signature")
        })
        || url.fragment().is_some_and(|fragment| fragment.contains(['=', '&'])) {
        return true;
    }
    match url.host() {
        Some(url::Host::Domain(host)) => {
            let host = host.trim_end_matches('.');
            !host.contains('.') || [".localhost", ".local", ".internal", ".lan", ".ts.net", ".invalid"].iter().any(|suffix| host.ends_with(*suffix))
        }
        Some(url::Host::Ipv4(address)) => private_ipv4(address),
        Some(url::Host::Ipv6(address)) => address.is_loopback() || address.is_unspecified() || address.is_multicast()
            || address.is_unique_local() || address.is_unicast_link_local() || address.to_ipv4_mapped().is_some_and(private_ipv4),
        None => true,
    }
}

fn private_ipv4(address: std::net::Ipv4Addr) -> bool {
    address.is_private() || address.is_loopback() || address.is_link_local() || address.is_unspecified()
        || address.is_broadcast() || address.is_multicast()
}

fn read_member(archive: &mut ZipArchive<File>, index: usize, limit: u64) -> Result<Vec<u8>, CliError> {
    let mut member = archive.by_index(index).map_err(zip_error)?;
    let expected = member.size();
    if expected > limit { return Err(resource("ZIP member exceeds its admitted byte length")); }
    let capacity = usize::try_from(expected + 1).map_err(|_| resource("ZIP member address space"))?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(capacity).map_err(|_| resource("ZIP member allocation"))?;
    (&mut member).take(expected + 1).read_to_end(&mut bytes).map_err(io)?;
    if bytes.len() as u64 != expected { return Err(invalid("ZIP member size differs from its header")); }
    Ok(bytes)
}

fn copy_hashed(reader: &mut impl Read, writer: &mut impl Write, limit: u64) -> Result<(u64, String), CliError> {
    let mut buffer = [0u8; 64 * 1024];
    let mut count = 0u64;
    let mut hash = Sha256::new();
    loop {
        let read = reader.read(&mut buffer).map_err(io)?;
        if read == 0 { break; }
        count = count.checked_add(read as u64).filter(|count| *count <= limit)
            .ok_or_else(|| resource("decompressed member exceeds its admitted length"))?;
        hash.update(&buffer[..read]);
        writer.write_all(&buffer[..read]).map_err(io)?;
    }
    Ok((count, hex::encode(hash.finalize())))
}

fn sha256(bytes: &[u8]) -> String { hex::encode(Sha256::digest(bytes)) }

fn hash_file(file: &mut File) -> Result<String, CliError> {
    file.seek(SeekFrom::Start(0)).map_err(io)?;
    copy_hashed(file, &mut std::io::sink(), MAX_ARCHIVE_BYTES).map(|(_, hash)| hash)
}

/// Bound the central directory before ZipArchive allocates its member table.
/// ZIP64 and multidisk are unnecessary below the fixed 2 GiB expansion cap.
fn preflight_zip(file: &mut File, max_archive: u64) -> Result<usize, CliError> {
    let length = file.metadata().map_err(io)?.len();
    if !(22..=max_archive).contains(&length) { return Err(resource("ZIP archive byte limit")); }
    file.seek(SeekFrom::Start(0)).map_err(io)?;
    let mut signature = [0u8; 4];
    file.read_exact(&mut signature).map_err(io)?;
    if signature != *b"PK\x03\x04" { return Err(invalid("ZIP must start with an ordinary local member")); }
    let tail_length = length.min(65_535 + 22);
    file.seek(SeekFrom::End(-(tail_length as i64))).map_err(io)?;
    let mut tail = Vec::with_capacity(tail_length as usize);
    file.read_to_end(&mut tail).map_err(io)?;
    let offset = (0..=tail.len() - 22).rev().find(|&offset| tail[offset..offset + 4] == *b"PK\x05\x06"
        && offset + 22 + u16::from_le_bytes([tail[offset + 20], tail[offset + 21]]) as usize == tail.len())
        .ok_or_else(|| invalid("missing or inconsistent ZIP end-of-directory record"))?;
    let end = &tail[offset..];
    let word = |offset| u16::from_le_bytes([end[offset], end[offset + 1]]);
    let dword = |offset| u32::from_le_bytes([end[offset], end[offset + 1], end[offset + 2], end[offset + 3]]) as u64;
    let entries = usize::from(word(10));
    let directory_size = dword(12);
    let directory_offset = dword(16);
    if word(4) != 0 || word(6) != 0 || word(8) != word(10) || !(2..=MAX_ENTRIES).contains(&entries)
        || directory_size > MAX_DIRECTORY_BYTES || directory_size < entries as u64 * 46
        || directory_offset.checked_add(directory_size) != Some(length - tail_length + offset as u64) {
        return Err(invalid("ZIP directory exceeds limits or uses unsupported ZIP64/multidisk layout"));
    }
    file.seek(SeekFrom::Start(0)).map_err(io)?;
    Ok(entries)
}

fn new_file(path: &Path) -> Result<File, CliError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)] {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    options.open(path).map_err(io)
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), CliError> {
    let mut file = new_file(path)?;
    file.write_all(bytes).map_err(io)?;
    file.sync_all().map_err(io)
}

fn zip_output(path: &Path) -> Result<PathBuf, CliError> {
    if path.extension().is_none() { return Ok(path.with_extension("zip")); }
    if path.extension().and_then(|extension| extension.to_str()).is_some_and(|extension| extension.eq_ignore_ascii_case("zip")) {
        return Ok(path.to_path_buf());
    }
    Err(CliError::new("INVALID_ARGUMENT", "portable project output must use .zip; open the extracted .verp in the desktop"))
}

struct OutputParent { directory: File, canonical: PathBuf, bound: PathBuf, name: OsString }

impl OutputParent {
    fn new(path: &Path) -> Result<Self, CliError> {
        let name = path.file_name().filter(|name| Path::new(name).components().all(|component| matches!(component, Component::Normal(_))))
            .ok_or_else(|| CliError::new("INVALID_ARGUMENT", "output needs a new file or directory name"))?.to_os_string();
        let parent = path.parent().filter(|path| !path.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let canonical = fs::canonicalize(parent).map_err(io)?;
        let directory = open_directory(&canonical)?;
        #[cfg(target_os = "linux")]
        let bound = {
            use std::os::fd::AsRawFd;
            PathBuf::from(format!("/proc/self/fd/{}", directory.as_raw_fd()))
        };
        #[cfg(not(target_os = "linux"))]
        let bound = canonical.clone();
        let parent = Self { directory, canonical, bound, name };
        parent.check_identity()?;
        Ok(parent)
    }
    fn bound_path(&self) -> &Path { &self.bound }
    fn destination(&self) -> PathBuf { self.bound.join(&self.name) }
    fn public_destination(&self) -> PathBuf { self.canonical.join(&self.name) }
    fn ensure_absent(&self) -> Result<(), CliError> {
        match fs::symlink_metadata(self.destination()) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Ok(_) => Err(CliError::new("OUTPUT_EXISTS", "output already exists")),
            Err(error) => Err(io(error)),
        }
    }
    fn check_identity(&self) -> Result<(), CliError> {
        let opened = self.directory.metadata().map_err(io)?;
        let named = fs::symlink_metadata(&self.canonical).map_err(io)?;
        if !named.is_dir() || named.file_type().is_symlink() { return Err(invalid("output parent changed")); }
        #[cfg(unix)] {
            use std::os::unix::fs::MetadataExt;
            if opened.dev() != named.dev() || opened.ino() != named.ino() { return Err(invalid("output parent changed")); }
        }
        #[cfg(not(unix))]
        if opened.created().ok() != named.created().ok() { return Err(invalid("output parent changed")); }
        Ok(())
    }
    fn sync(&self) -> Result<(), CliError> {
        #[cfg(unix)]
        self.directory.sync_all().map_err(io)?;
        Ok(())
    }
}

#[cfg(unix)]
fn open_directory(path: &Path) -> Result<File, CliError> {
    use std::{ffi::CString, os::{fd::{AsRawFd, FromRawFd}, unix::ffi::OsStrExt}};
    let mut options = OpenOptions::new();
    use std::os::unix::fs::OpenOptionsExt;
    options.read(true).custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC);
    let mut directory = options.open("/").map_err(io)?;
    for component in path.components() {
        let component = match component { Component::RootDir => continue, Component::Normal(name) => name, _ => return Err(invalid("invalid output parent")) };
        let name = CString::new(component.as_bytes()).map_err(argument)?;
        // SAFETY: name is terminated, parent is live, and the fresh fd is
        // immediately owned. No path component may resolve through a symlink.
        let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC) };
        if fd < 0 { return Err(io(std::io::Error::last_os_error())); }
        directory = unsafe { File::from_raw_fd(fd) };
    }
    Ok(directory)
}

#[cfg(windows)]
fn open_directory(path: &Path) -> Result<File, CliError> {
    use std::os::windows::fs::OpenOptionsExt;
    OpenOptions::new().read(true).custom_flags(0x0200_0000 | 0x0020_0000).open(path).map_err(io)
}

#[cfg(not(any(unix, windows)))]
fn open_directory(path: &Path) -> Result<File, CliError> { File::open(path).map_err(io) }

fn sync_directory(path: &Path) -> Result<(), CliError> {
    #[cfg(unix)] {
        use std::os::unix::fs::OpenOptionsExt;
        // Staging paths may intentionally start at the parent's owned proc fd.
        // Their remaining components are created privately by this operation.
        OpenOptions::new().read(true).custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path).and_then(|directory| directory.sync_all()).map_err(io)?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn publish_directory(parent: &OutputParent, temporary: &Path) -> Result<(), CliError> {
    use std::{ffi::CString, os::{fd::AsRawFd, unix::ffi::OsStrExt}};
    let source = CString::new(temporary.file_name().ok_or_else(|| invalid("staging directory lacks a name"))?.as_bytes()).map_err(argument)?;
    let destination = CString::new(parent.name.as_bytes()).map_err(argument)?;
    let fd = parent.directory.as_raw_fd();
    #[cfg(target_os = "linux")]
    let result = unsafe { libc::renameat2(fd, source.as_ptr(), fd, destination.as_ptr(), libc::RENAME_NOREPLACE) };
    #[cfg(target_os = "macos")]
    let result = unsafe { libc::renameatx_np(fd, source.as_ptr(), fd, destination.as_ptr(), libc::RENAME_EXCL) };
    if result != 0 { return Err(io(std::io::Error::last_os_error())); }
    Ok(())
}

#[cfg(windows)]
fn publish_directory(parent: &OutputParent, temporary: &Path) -> Result<(), CliError> {
    // Windows rename refuses an existing destination directory, including an
    // empty one; this publishes the already-verified tree in a single rename.
    fs::rename(temporary, parent.destination()).map_err(io)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn publish_directory(_: &OutputParent, _: &Path) -> Result<(), CliError> {
    Err(CliError::new("UNSUPPORTED_CAPABILITY", "atomic no-clobber directory publication is unavailable on this platform"))
}

fn lowered_limit(value: Option<u64>, maximum: u64, name: &str) -> Result<u64, CliError> {
    let value = value.unwrap_or(maximum);
    if value == 0 || value > maximum {
        return Err(CliError::new("INVALID_ARGUMENT", format!("{name} must be in 1..={maximum}")));
    }
    Ok(value)
}

fn limits(max_expanded: u64, max_archive: u64) -> Value {
    let audio = ImportLimits::default();
    json!({"entries":MAX_ENTRIES,"project_bytes":MAX_PROJECT_BYTES,"manifest_bytes":MAX_MANIFEST_BYTES,
        "asset_bytes":MAX_ASSET_BYTES,"expanded_bytes":max_expanded,"archive_bytes":max_archive,
        "central_directory_bytes":MAX_DIRECTORY_BYTES,"expansion_ratio":MAX_RATIO,"zip64":false,
        "decoded_frames_per_asset":audio.max_decoded_frames,"decoded_pcm_bytes_per_asset":audio.max_pcm_bytes,
        "max_sample_rate":audio.max_sample_rate,"channels":[1,2]})
}

fn argument(error: impl std::fmt::Display) -> CliError { CliError::new("INVALID_ARGUMENT", error.to_string()) }
fn invalid(message: impl Into<String>) -> CliError { CliError::new("INVALID_PROJECT", message) }
fn resource(message: impl Into<String>) -> CliError { CliError::new("RESOURCE_EXHAUSTED", message) }
fn internal(error: impl std::fmt::Display) -> CliError { CliError::new("INTERNAL_ERROR", error.to_string()) }
fn zip_error(error: zip::result::ZipError) -> CliError { invalid(format!("portable ZIP: {error}")) }
fn io(error: std::io::Error) -> CliError {
    CliError::new(if error.kind() == std::io::ErrorKind::AlreadyExists { "OUTPUT_EXISTS" } else { "IO_ERROR" }, error.to_string())
}
fn media(error: MediaError) -> CliError {
    let code = match &error {
        MediaError::Limit(_) => "RESOURCE_EXHAUSTED",
        MediaError::Io(error) if error.kind() == std::io::ErrorKind::NotFound => "MISSING_ASSET",
        MediaError::UnauthorizedPath => "DECISION_REQUIRED",
        _ => "INVALID_PROJECT",
    };
    // Filesystem diagnostics may embed private host paths. Expose the class,
    // while the transport document and manifest contain only portable names.
    let message = match error { MediaError::Io(_) => "cannot read the referenced media asset".into(), error => error.to_string() };
    CliError::new(code, message)
}
