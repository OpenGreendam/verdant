//! Read the original source pair without running installers or writing archive members.
//!
//! Names follow the original installer: loose files use the case-sensitive
//! basenames `sn16k.tkn` / `sn16k.bin`; cabinet members use the exact MSI IDs
//! `sn16k.tkn58` / `sn16k.bin63`. Source hashes and installation admission belong
//! to the caller. Duplicate candidates are errors, including identical copies.

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Cursor, Read};
use std::path::Path;

use miniz_oxide::inflate::TINFLStatus;
use miniz_oxide::inflate::core::{
    DecompressorOxide, decompress, inflate_flags::TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
};
use zip::{CompressionMethod, ZipArchive};

const SOURCE_NAMES: [&str; 2] = ["sn16k.tkn", "sn16k.bin"];
const CAB_IDS: [&str; 2] = ["sn16k.tkn58", "sn16k.bin63"];
const MAX_FILE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_SOURCE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_ENTRIES: usize = 16_384;
const MAX_BLOCKS: usize = 131_072;
const MAX_NAME_BYTES: usize = 4096;
const MAX_TOTAL_NAME_BYTES: usize = 8 * 1024 * 1024;
const MAX_ARCHIVE_DEPTH: usize = 8;
const MAX_DIRECTORY_DEPTH: usize = 32;
const CAB_BLOCK_BYTES: usize = 32_768;

/// Return the unchanged token and bank bytes from a directory, ZIP, stored RAR,
/// or standalone STORED/MSZIP cabinet. ZIPs and stored RARs can contain nested
/// containers. No member is extracted to disk, and no executable is launched.
///
/// Reads and expansion share a 1 GiB budget, with 512 MiB per input/member,
/// 64 MiB per source file, 16,384 entries, and eight archive nesting levels.
/// Unsafe paths, links, ambiguous names, corrupt data, encryption, split archives,
/// and unsupported compression fail before a pair is returned.
pub fn extract_pair(path: &Path) -> Result<(Vec<u8>, Vec<u8>), String> {
    let mut extractor = Extractor::default();
    let metadata = fs::symlink_metadata(path).map_err(|error| format!("input metadata: {error}"))?;
    if metadata.file_type().is_symlink() {
        return Err("input must not be a symbolic link".into());
    }
    if metadata.is_dir() {
        extractor.directory(path, 0)?;
    } else if metadata.is_file() {
        let name = path.file_name().and_then(|name| name.to_str()).ok_or("input name is not UTF-8")?;
        extractor.input_file(path, name, true)?;
    } else {
        return Err("input must be a regular file or directory".into());
    }
    let [token, bank] = extractor.pair;
    match (token, bank) {
        (Some(token), Some(bank)) => Ok((token, bank)),
        _ => Err("could not obtain both sn16k.tkn and sn16k.bin; provide the original pair or MagicVoice1.cab in a supported local container".into()),
    }
}

#[derive(Default)]
struct Budget {
    bytes: u64,
    entries: usize,
    blocks: usize,
    names: usize,
}

impl Budget {
    fn bytes(&mut self, count: u64) -> Result<(), String> {
        self.bytes = self.bytes.checked_add(count).filter(|total| *total <= MAX_TOTAL_BYTES)
            .ok_or("input and expanded data exceed the 1 GiB extraction limit")?;
        Ok(())
    }

    fn entries(&mut self, count: usize) -> Result<(), String> {
        self.entries = self.entries.checked_add(count).filter(|total| *total <= MAX_ENTRIES)
            .ok_or("input exceeds the extraction entry limit")?;
        Ok(())
    }

    fn block(&mut self) -> Result<(), String> {
        self.blocks = self.blocks.checked_add(1).filter(|total| *total <= MAX_BLOCKS)
            .ok_or("input exceeds the archive block limit")?;
        Ok(())
    }

    fn name(&mut self, count: usize) -> Result<(), String> {
        if count > MAX_NAME_BYTES {
            return Err("archive member name exceeds the path limit".into());
        }
        self.names = self.names.checked_add(count).filter(|total| *total <= MAX_TOTAL_NAME_BYTES)
            .ok_or("input exceeds the total member-name limit")?;
        Ok(())
    }
}

#[derive(Default)]
struct Extractor {
    budget: Budget,
    pair: [Option<Vec<u8>>; 2],
}

impl Extractor {
    fn source_slot(&mut self, index: usize, size: u64) -> Result<&mut Option<Vec<u8>>, String> {
        source_size(size)?;
        let slot = &mut self.pair[index];
        if slot.is_some() {
            return Err(format!("ambiguous source: more than one {} candidate", SOURCE_NAMES[index]));
        }
        Ok(slot)
    }

    fn put(&mut self, index: usize, bytes: Vec<u8>) -> Result<(), String> {
        let slot = self.source_slot(index, bytes.len() as u64)?;
        *slot = Some(bytes);
        Ok(())
    }

    fn directory(&mut self, path: &Path, depth: usize) -> Result<(), String> {
        if depth > MAX_DIRECTORY_DEPTH {
            return Err("input directory nesting exceeds the extraction limit".into());
        }
        let entries = fs::read_dir(path).map_err(|error| format!("reading input directory: {error}"))?;
        for entry in entries {
            self.budget.entries(1)?;
            let entry = entry.map_err(|error| format!("reading directory entry: {error}"))?;
            let kind = entry.file_type().map_err(|error| format!("reading entry type: {error}"))?;
            let name = entry.file_name();
            let name = name.to_str().ok_or("directory member name is not UTF-8")?;
            self.budget.name(name.len())?;
            safe_name(name, kind.is_dir())?;
            if kind.is_symlink() {
                return Err("input directory contains a symbolic link".into());
            }
            if kind.is_dir() {
                self.directory(&entry.path(), depth + 1)?;
            } else if kind.is_file() {
                self.input_file(&entry.path(), name, false)?;
            } else {
                return Err("input directory contains a non-regular member".into());
            }
        }
        Ok(())
    }

    fn input_file(&mut self, path: &Path, name: &str, root: bool) -> Result<(), String> {
        let bytes = self.read_file(path)?;
        if let Some(index) = SOURCE_NAMES.iter().position(|expected| *expected == name) {
            self.put(index, bytes)
        } else {
            self.blob(name, &bytes, 0, root)
        }
    }

    fn read_file(&mut self, path: &Path) -> Result<Vec<u8>, String> {
        // Recheck the final component immediately before opening; reads remain
        // bounded even if the user changes the supplied directory concurrently.
        let metadata = fs::symlink_metadata(path).map_err(|error| format!("input metadata: {error}"))?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err("input member must be a regular file without a symbolic link".into());
        }
        let mut file = File::open(path).map_err(|error| format!("opening input: {error}"))?;
        let opened = file.metadata().map_err(|error| format!("opened input metadata: {error}"))?;
        if !opened.is_file() || opened.len() != metadata.len() {
            return Err("input member changed while it was being opened".into());
        }
        let size = member_size(opened.len())?;
        if path.file_name().and_then(|name| name.to_str()).is_some_and(|name| SOURCE_NAMES.contains(&name)) {
            source_size(opened.len())?;
        }
        self.budget.bytes(opened.len())?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(end_offset(size, 1)?).map_err(|_| "could not allocate bounded input buffer")?;
        (&mut file).take(opened.len() + 1).read_to_end(&mut bytes)
            .map_err(|error| format!("reading input: {error}"))?;
        if bytes.len() != size {
            return Err("input length changed while it was being read".into());
        }
        Ok(bytes)
    }

    fn blob(&mut self, name: &str, bytes: &[u8], depth: usize, root: bool) -> Result<(), String> {
        if let Some(index) = SOURCE_NAMES.iter().position(|expected| *expected == basename(name)) {
            *self.source_slot(index, bytes.len() as u64)? = Some(copied(bytes)?);
            return Ok(());
        }
        let extension = basename(name).rsplit_once('.').map(|(_, extension)| extension);
        let is_zip = bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06")
            || zip_end(bytes).is_some() || extension.is_some_and(|value| value.eq_ignore_ascii_case("zip"));
        let is_rar = bytes.starts_with(b"Rar!\x1a\x07")
            || extension.is_some_and(|value| value.eq_ignore_ascii_case("rar"));
        let is_cab = bytes.starts_with(b"MSCF")
            || basename(name).eq_ignore_ascii_case("MagicVoice1.cab");
        if is_zip || is_rar || is_cab {
            if depth >= MAX_ARCHIVE_DEPTH {
                return Err("archive nesting exceeds the extraction limit".into());
            }
            if is_cab {
                let [token, bank] = cabinet(bytes, &mut self.budget)?;
                self.put(0, token)?;
                self.put(1, bank)?;
            } else if is_rar {
                if bytes.starts_with(b"Rar!\x1a\x07\x00") {
                    self.rar4(bytes, depth + 1)?;
                } else if bytes.starts_with(b"Rar!\x1a\x07\x01\x00") {
                    self.rar5(bytes, depth + 1)?;
                } else {
                    return Err("invalid or unsupported RAR signature".into());
                }
            } else {
                self.zip(bytes, depth + 1)?;
            }
            return Ok(());
        }
        if root {
            return Err("unsupported input container; provide a ZIP, stored RAR, standalone CAB, or directory containing the original pair".into());
        }
        Ok(())
    }

    fn zip(&mut self, bytes: &[u8], depth: usize) -> Result<(), String> {
        // Check counts and raw duplicate names before ZipArchive builds its
        // name map, which otherwise hides repeated central-directory entries.
        let directory = zip_directory(bytes, &mut self.budget)?;
        let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|error| format!("invalid ZIP: {error}"))?;
        if archive.len() != directory.entries {
            return Err("ZIP contains duplicate or inconsistent member names".into());
        }
        let mut names = HashSet::new();
        let mut ranges = Vec::with_capacity(directory.entries);
        for index in 0..archive.len() {
            let file = archive.by_index_raw(index).map_err(|error| format!("invalid ZIP member: {error}"))?;
            let central = offset(file.central_header_start())?;
            if central < directory.start || central >= directory.end {
                return Err("ZIP central-directory identity mismatch".into());
            }
            let mode = file.unix_mode().unwrap_or(0) & 0o170000;
            let is_dir = file.is_dir() || mode == 0o040000;
            if !matches!(mode, 0 | 0o040000 | 0o100000) || file.is_symlink() {
                return Err("ZIP links and special files are not supported".into());
            }
            let name = safe_name(file.name(), is_dir)?;
            self.budget.name(file.name().len())?;
            if names.contains(&name) {
                return Err("ZIP contains duplicate normalized member names".into());
            }
            if file.encrypted() {
                return Err("encrypted ZIP members are not supported".into());
            }
            let size = member_size(file.size())?;
            if !is_dir && SOURCE_NAMES.contains(&basename(&name)) {
                source_size(file.size())?;
            }
            self.budget.bytes(file.size())?;
            let start = offset(file.data_start())?;
            let packed = member_size(file.compressed_size())?;
            let raw = slice(bytes, start, packed)?;
            let end = zip_local_member(bytes, &file, central, directory.start)?;
            ranges.push((offset(file.header_start())?, end));
            let decoded;
            let data = match file.compression() {
                CompressionMethod::Stored => {
                    if packed != size {
                        return Err("stored ZIP member has inconsistent lengths".into());
                    }
                    raw
                }
                CompressionMethod::Deflated => {
                    decoded = inflate(raw, size)?;
                    &decoded
                }
                method => return Err(format!("unsupported ZIP compression method {method:?}")),
            };
            if crc32fast::hash(data) != file.crc32() {
                return Err("ZIP member CRC32 mismatch".into());
            }
            if is_dir {
                if !data.is_empty() {
                    return Err("ZIP directory contains file data".into());
                }
            } else {
                self.blob(&name, data, depth, false)?;
            }
            names.insert(name);
        }
        disjoint(&mut ranges, "ZIP member data")
    }

    fn rar4(&mut self, bytes: &[u8], depth: usize) -> Result<(), String> {
        let mut cursor = Bytes::at(bytes, 7)?;
        let mut main = false;
        let mut names = HashSet::new();
        while !cursor.remaining().is_empty() {
            self.budget.block()?;
            let start = cursor.pos;
            let expected_crc = cursor.u16()?;
            let kind = cursor.u8()?;
            let flags = cursor.u16()?;
            let size = usize::from(cursor.u16()?);
            if size < 7 {
                return Err("RAR4 header is shorter than its fixed fields".into());
            }
            let header = slice(bytes, start, size)?;
            if crc32fast::hash(&header[2..]) as u16 != expected_crc {
                return Err("RAR4 header CRC mismatch".into());
            }
            let mut body = Bytes::at(header, 7)?;
            let packed = if flags & 0x8000 != 0 { u64::from(body.u32()?) } else { 0 };
            let mut data_size = packed;
            match kind {
                0x73 => {
                    if main || start != 7 || flags & (0x0001 | 0x0008 | 0x0080) != 0 {
                        return Err("RAR4 duplicate, split, solid, or encrypted archive is not supported".into());
                    }
                    if flags & 0x8000 != 0 || header.len() < 13 {
                        return Err("invalid RAR4 main header".into());
                    }
                    main = true;
                }
                0x74 | 0x7a => {
                    if !main || flags & 0x8000 == 0 {
                        return Err("RAR4 file lacks a main header or packed-size field".into());
                    }
                    // Reject split, encrypted, solid, salted, and versioned
                    // files; dictionary bits 5..7 also identify directories.
                    if flags & (0x001f | 0x0400 | 0x0800) != 0 {
                        return Err("RAR4 split, encrypted, solid, or versioned files are not supported".into());
                    }
                    let mut unpacked = u64::from(body.u32()?);
                    let host = body.u8()?;
                    if host > 3 {
                        return Err("unsupported RAR4 host operating system".into());
                    }
                    let crc = body.u32()?;
                    body.u32()?;
                    body.u8()?;
                    let method = body.u8()?;
                    let name_size = usize::from(body.u16()?);
                    let attributes = body.u32()?;
                    if flags & 0x0100 != 0 {
                        data_size |= u64::from(body.u32()?) << 32;
                        unpacked |= u64::from(body.u32()?) << 32;
                    }
                    self.budget.name(name_size)?;
                    let name = rar4_name(body.take(name_size)?, flags & 0x0200 != 0)?;
                    let is_dir = flags & 0x00e0 == 0x00e0;
                    regular_rar_kind(host == 3, attributes, is_dir)?;
                    let name = safe_name(&name, is_dir)?;
                    if kind == 0x74 {
                        self.budget.entries(1)?;
                        if names.contains(&name) {
                            return Err("RAR4 contains duplicate member names".into());
                        }
                    }
                    member_size(unpacked)?;
                    member_size(data_size)?;
                    if method != 0x30 {
                        return Err("compressed RAR4 members are not supported; provide locally extracted source files".into());
                    }
                    if unpacked != data_size || (is_dir && unpacked != 0) {
                        return Err("stored RAR4 member has inconsistent lengths".into());
                    }
                    self.budget.bytes(unpacked)?;
                    let data = slice(bytes, start + size, offset(data_size)?)?;
                    if crc32fast::hash(data) != crc {
                        return Err("RAR4 member CRC32 mismatch".into());
                    }
                    if kind == 0x74 && !is_dir {
                        self.blob(&name, data, depth, false)?;
                    }
                    if kind == 0x74 {
                        names.insert(name);
                    }
                }
                0x7b => {
                    if !main || flags & 0x0001 != 0 || data_size != 0 {
                        return Err("invalid or multi-volume RAR4 end header".into());
                    }
                    if start + size != bytes.len() {
                        return Err("unexpected data after the RAR4 end header".into());
                    }
                    return Ok(());
                }
                0x75..=0x79 => {
                    if !main {
                        return Err("RAR4 block precedes its main header".into());
                    }
                }
                _ => return Err("unsupported RAR4 block type".into()),
            }
            member_size(data_size)?;
            cursor = Bytes::at(bytes, end_offset(start + size, offset(data_size)?)?)?;
        }
        Err("RAR4 archive is missing its end header".into())
    }

    fn rar5(&mut self, bytes: &[u8], depth: usize) -> Result<(), String> {
        let mut cursor = Bytes::at(bytes, 8)?;
        let mut main = false;
        let mut names = HashSet::new();
        while !cursor.remaining().is_empty() {
            self.budget.block()?;
            let crc = cursor.u32()?;
            let crc_start = cursor.pos;
            let size = cursor.vint()?;
            if size > 2 * 1024 * 1024 || cursor.pos - crc_start > 3 {
                return Err("RAR5 header exceeds the format limit".into());
            }
            let start = cursor.pos;
            let end = end_offset(start, offset(size)?)?;
            let header = slice(bytes, start, offset(size)?)?;
            if crc32fast::hash(slice(bytes, crc_start, end - crc_start)?) != crc {
                return Err("RAR5 header CRC32 mismatch".into());
            }
            let mut fields = Bytes::at(header, 0)?;
            let kind = fields.vint()?;
            let flags = fields.vint()?;
            if flags & !0x007f != 0 || flags & 0x0018 != 0 {
                return Err("unsupported or split RAR5 block flags".into());
            }
            let extra_size = if flags & 1 != 0 { offset(fields.vint()?)? } else { 0 };
            let data_size = if flags & 2 != 0 { member_size(fields.vint()?)? } else { 0 };
            let extra_start = header.len().checked_sub(extra_size).filter(|value| *value >= fields.pos)
                .ok_or("RAR5 extra area overlaps its block fields")?;
            rar5_extra(&header[extra_start..], matches!(kind, 2 | 3))?;
            let mut body = Bytes::at(&header[..extra_start], fields.pos)?;
            match kind {
                1 => {
                    if main || crc_start != 12 || data_size != 0 {
                        return Err("RAR5 duplicate or misplaced main header".into());
                    }
                    let archive_flags = body.vint()?;
                    if archive_flags & !0x0018 != 0 {
                        return Err("RAR5 split, solid, or unsupported archive flags".into());
                    }
                    main = true;
                }
                2 | 3 => {
                    if !main {
                        return Err("RAR5 file precedes its main header".into());
                    }
                    let file_flags = body.vint()?;
                    if file_flags & !0x0007 != 0 {
                        return Err("RAR5 unknown unpacked size or unsupported file flags".into());
                    }
                    let unpacked = member_size(body.vint()?)?;
                    let attributes = body.vint()?;
                    if file_flags & 2 != 0 {
                        body.u32()?;
                    }
                    let file_crc = if file_flags & 4 != 0 { Some(body.u32()?) } else { None };
                    let compression = body.vint()?;
                    if compression & 0x03c0 != 0 || compression & 0x003f > 1 || compression > 0x1f_ffff {
                        return Err("compressed or solid RAR5 members are not supported; provide locally extracted source files".into());
                    }
                    let host = body.vint()?;
                    if host > 1 {
                        return Err("unsupported RAR5 host operating system".into());
                    }
                    let name_size = offset(body.vint()?)?;
                    self.budget.name(name_size)?;
                    let name = std::str::from_utf8(body.take(name_size)?).map_err(|_| "invalid UTF-8 RAR5 member name")?;
                    let is_dir = kind == 2 && file_flags & 1 != 0;
                    regular_rar_kind(host == 1, u32::try_from(attributes).map_err(|_| "RAR5 attributes overflow")?, is_dir)?;
                    let name = safe_name(name, is_dir)?;
                    if kind == 2 {
                        self.budget.entries(1)?;
                        if names.contains(&name) {
                            return Err("RAR5 contains duplicate member names".into());
                        }
                    }
                    if unpacked != data_size || (is_dir && unpacked != 0) {
                        return Err("stored RAR5 member has inconsistent lengths".into());
                    }
                    let data = slice(bytes, end, data_size)?;
                    self.budget.bytes(unpacked as u64)?;
                    if file_crc.is_some_and(|expected| crc32fast::hash(data) != expected) {
                        return Err("RAR5 member CRC32 mismatch".into());
                    }
                    if kind == 2 && !is_dir {
                        self.blob(&name, data, depth, false)?;
                    }
                    if kind == 2 {
                        names.insert(name);
                    }
                }
                4 => return Err("encrypted RAR5 archives are not supported".into()),
                5 => {
                    if !main || body.vint()? != 0 || data_size != 0 {
                        return Err("invalid or multi-volume RAR5 end header".into());
                    }
                    if !body.remaining().is_empty() || end != bytes.len() {
                        return Err("unexpected data after the RAR5 end header".into());
                    }
                    return Ok(());
                }
                _ => return Err("unsupported RAR5 block type".into()),
            }
            if !body.remaining().is_empty() {
                return Err("RAR5 block has unexpected header fields".into());
            }
            cursor = Bytes::at(bytes, end_offset(end, data_size)?)?;
        }
        Err("RAR5 archive is missing its end header".into())
    }
}

fn basename(name: &str) -> &str {
    name.rsplit(['/', '\\']).next().unwrap_or(name)
}

fn safe_name(name: &str, is_dir: bool) -> Result<String, String> {
    safe_path_bytes(name.as_bytes(), is_dir)?;
    if name.chars().any(char::is_control) {
        return Err("archive member path contains a control character".into());
    }
    let name = if is_dir { name.strip_suffix(['/', '\\']).unwrap_or(name) } else { name };
    Ok(name.replace('\\', "/"))
}

fn safe_path_bytes(name: &[u8], is_dir: bool) -> Result<(), String> {
    if name.len() > MAX_NAME_BYTES || name.is_empty() || matches!(name.first(), Some(b'/' | b'\\'))
        || name.iter().any(|byte| *byte < 0x20 || matches!(byte, 0x7f | b':' | b'<' | b'>' | b'"' | b'|' | b'?' | b'*')) {
        return Err("invalid or absolute archive member path".into());
    }
    let name = if is_dir && matches!(name.last(), Some(b'/' | b'\\')) { &name[..name.len() - 1] } else { name };
    for component in name.split(|byte| matches!(byte, b'/' | b'\\')) {
        if component.is_empty() || component == b"." || component == b".." || matches!(component.last(), Some(b'.' | b' ')) {
            return Err("invalid, ambiguous, or traversing archive member path".into());
        }
        let stem = component.split(|byte| *byte == b'.').next().unwrap_or(component);
        if [b"CON", b"PRN", b"AUX", b"NUL"].iter().any(|value| stem.eq_ignore_ascii_case(*value))
            || (stem.len() == 4 && (stem[..3].eq_ignore_ascii_case(b"COM") || stem[..3].eq_ignore_ascii_case(b"LPT"))
                && matches!(stem[3], b'1'..=b'9'))
        {
            return Err("archive member path uses a reserved device name".into());
        }
    }
    Ok(())
}

fn member_size(size: u64) -> Result<usize, String> {
    if size > MAX_FILE_BYTES {
        return Err("input or archive member exceeds the 512 MiB limit".into());
    }
    offset(size)
}

fn source_size(size: u64) -> Result<(), String> {
    if size == 0 || size > MAX_SOURCE_BYTES {
        return Err("source token or bank is empty or exceeds the 64 MiB limit".into());
    }
    Ok(())
}

fn offset(value: u64) -> Result<usize, String> {
    usize::try_from(value).map_err(|_| "archive offset exceeds this platform's address space".into())
}

fn end_offset(start: usize, size: usize) -> Result<usize, String> {
    start.checked_add(size).ok_or_else(|| "archive offset overflow".into())
}

fn slice(bytes: &[u8], start: usize, size: usize) -> Result<&[u8], String> {
    bytes.get(start..end_offset(start, size)?).ok_or_else(|| "truncated or out-of-bounds archive data".into())
}

fn zeroed(size: usize) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(size).map_err(|_| "could not allocate bounded extraction buffer")?;
    bytes.resize(size, 0);
    Ok(bytes)
}

fn copied(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(data.len()).map_err(|_| "could not allocate source file buffer")?;
    bytes.extend_from_slice(data);
    Ok(bytes)
}

struct Bytes<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Bytes<'a> {
    fn at(data: &'a [u8], pos: usize) -> Result<Self, String> {
        if pos > data.len() {
            return Err("truncated archive structure".into());
        }
        Ok(Self { data, pos })
    }

    fn remaining(&self) -> &'a [u8] {
        &self.data[self.pos..]
    }

    fn take(&mut self, size: usize) -> Result<&'a [u8], String> {
        let bytes = slice(self.data, self.pos, size)?;
        self.pos += size;
        Ok(bytes)
    }

    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, String> {
        let bytes = self.take(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&mut self) -> Result<u32, String> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn u64(&mut self) -> Result<u64, String> {
        let bytes = self.take(8)?;
        Ok(u64::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]]))
    }

    fn vint(&mut self) -> Result<u64, String> {
        let mut value = 0;
        for shift in (0..70).step_by(7) {
            let byte = self.u8()?;
            if shift == 63 && byte & 0xfe != 0 {
                return Err("RAR5 variable-length integer overflows u64".into());
            }
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err("unterminated RAR5 variable-length integer".into())
    }

    fn cstring(&mut self) -> Result<&'a [u8], String> {
        let length = self.remaining().iter().take(MAX_NAME_BYTES + 1).position(|byte| *byte == 0)
            .ok_or("unterminated or over-limit CAB member name")?;
        let name = self.take(length)?;
        self.take(1)?;
        Ok(name)
    }
}

fn inflate(input: &[u8], size: usize) -> Result<Vec<u8>, String> {
    let mut output = zeroed(end_offset(size, 1)?)?;
    let mut decoder = DecompressorOxide::new();
    inflate_into(&mut decoder, input, &mut output, 0, size)?;
    output.truncate(size);
    Ok(output)
}

fn inflate_into(
    decoder: &mut DecompressorOxide,
    input: &[u8],
    output: &mut [u8],
    history: usize,
    expected: usize,
) -> Result<(), String> {
    decoder.init();
    let (status, consumed, produced) = decompress(
        decoder, input, output, history, TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
    );
    if status != TINFLStatus::Done || consumed != input.len() || produced != expected {
        return Err("invalid, incomplete, trailing, or oversized DEFLATE stream".into());
    }
    Ok(())
}

fn disjoint(ranges: &mut [(usize, usize)], label: &str) -> Result<(), String> {
    ranges.sort_unstable();
    if ranges.windows(2).any(|pair| pair[0].1 > pair[1].0) {
        return Err(format!("overlapping {label} ranges"));
    }
    Ok(())
}

struct ZipDirectory {
    entries: usize,
    start: usize,
    end: usize,
}

fn zip_end(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 22 {
        return None;
    }
    let begin = bytes.len().saturating_sub(22 + usize::from(u16::MAX));
    (begin..=bytes.len() - 22).rev().find(|start| {
        &bytes[*start..*start + 4] == b"PK\x05\x06"
            && *start + 22 + usize::from(u16::from_le_bytes([bytes[*start + 20], bytes[*start + 21]])) == bytes.len()
    })
}

fn zip_directory(bytes: &[u8], budget: &mut Budget) -> Result<ZipDirectory, String> {
    let end = zip_end(bytes).ok_or("ZIP is missing a complete end-of-central-directory record")?;
    let mut fields = Bytes::at(bytes, end + 4)?;
    if fields.u16()? != 0 || fields.u16()? != 0 {
        return Err("multi-disk ZIP archives are not supported".into());
    }
    let disk_count = fields.u16()?;
    let mut count = u64::from(fields.u16()?);
    let mut directory_size = u64::from(fields.u32()?);
    let mut directory_offset = u64::from(fields.u32()?);
    let mut directory_end = end;
    if disk_count != count as u16 {
        return Err("inconsistent ZIP member counts".into());
    }
    let has_zip64 = end >= 20 && &bytes[end - 20..end - 16] == b"PK\x06\x07";
    if has_zip64 || count == u64::from(u16::MAX) || directory_size == u64::from(u32::MAX) || directory_offset == u64::from(u32::MAX) {
        let locator = end.checked_sub(20).ok_or("ZIP64 locator is missing")?;
        let mut locator_fields = Bytes::at(slice(bytes, locator, 20)?, 0)?;
        if locator_fields.take(4)? != b"PK\x06\x07" || locator_fields.u32()? != 0 {
            return Err("invalid or multi-disk ZIP64 locator".into());
        }
        let record = offset(locator_fields.u64()?)?;
        if locator_fields.u32()? != 1 {
            return Err("multi-disk ZIP64 archives are not supported".into());
        }
        let mut record_fields = Bytes::at(bytes, record)?;
        if record_fields.take(4)? != b"PK\x06\x06" {
            return Err("invalid ZIP64 end record".into());
        }
        let record_size = offset(record_fields.u64()?)?;
        if record_size < 44 || end_offset(record, end_offset(12, record_size)?)? != locator {
            return Err("invalid ZIP64 end record length".into());
        }
        record_fields.take(4)?;
        if record_fields.u32()? != 0 || record_fields.u32()? != 0 {
            return Err("multi-disk ZIP64 archives are not supported".into());
        }
        let disk_count = record_fields.u64()?;
        count = record_fields.u64()?;
        if count != disk_count {
            return Err("inconsistent ZIP64 member counts".into());
        }
        directory_size = record_fields.u64()?;
        directory_offset = record_fields.u64()?;
        directory_end = record;
    }
    let count = offset(count)?;
    budget.entries(count)?;
    let start = directory_end.checked_sub(offset(directory_size)?).ok_or("invalid ZIP central-directory length")?;
    if offset(directory_offset)? > start {
        return Err("invalid ZIP central-directory offset".into());
    }
    let mut cursor = Bytes::at(&bytes[..directory_end], start)?;
    let mut raw_names = HashSet::new();
    for _ in 0..count {
        let fixed = cursor.take(46)?;
        if &fixed[..4] != b"PK\x01\x02" {
            return Err("invalid ZIP central-directory member".into());
        }
        let flags = u16::from_le_bytes([fixed[8], fixed[9]]);
        let method = u16::from_le_bytes([fixed[10], fixed[11]]);
        if flags & !0x080e != 0 || !matches!(method, 0 | 8) {
            return Err("encrypted or unsupported ZIP member method/flags".into());
        }
        if fixed[34..36] != [0, 0] {
            return Err("ZIP member refers to another disk".into());
        }
        let name_size = usize::from(u16::from_le_bytes([fixed[28], fixed[29]]));
        budget.name(name_size)?;
        let name = cursor.take(name_size)?;
        if name.is_empty() || !raw_names.insert(name) {
            return Err("ZIP contains an empty or duplicate member name".into());
        }
        safe_path_bytes(name, matches!(name.last(), Some(b'/' | b'\\')))?;
        if flags & 0x0800 != 0 && std::str::from_utf8(name).is_err() {
            return Err("invalid UTF-8 ZIP member name".into());
        }
        let extra_size = usize::from(u16::from_le_bytes([fixed[30], fixed[31]]));
        let comment_size = usize::from(u16::from_le_bytes([fixed[32], fixed[33]]));
        cursor.take(extra_size)?;
        cursor.take(comment_size)?;
    }
    if cursor.pos != directory_end {
        return Err("ZIP central-directory size/count mismatch".into());
    }
    Ok(ZipDirectory { entries: count, start, end: directory_end })
}

fn zip_local_member<R: Read>(
    bytes: &[u8], file: &zip::read::ZipFile<'_, R>, central: usize, directory: usize,
) -> Result<usize, String> {
    let header = offset(file.header_start())?;
    let fixed = slice(bytes, header, 30)?;
    let central_fixed = slice(bytes, central, 46)?;
    if &fixed[..4] != b"PK\x03\x04" || fixed[6..10] != central_fixed[8..12] {
        return Err("ZIP local/central header mismatch".into());
    }
    let name_size = usize::from(u16::from_le_bytes([fixed[26], fixed[27]]));
    let extra_size = usize::from(u16::from_le_bytes([fixed[28], fixed[29]]));
    let name = slice(bytes, header + 30, name_size)?;
    if name != file.name_raw() {
        return Err("ZIP local/central file name mismatch".into());
    }
    let extra = slice(bytes, end_offset(header + 30, name_size)?, extra_size)?;
    let data_start = end_offset(end_offset(header + 30, name_size)?, extra_size)?;
    if data_start != offset(file.data_start())? {
        return Err("ZIP member data offset mismatch".into());
    }
    let end = end_offset(data_start, member_size(file.compressed_size())?)?;
    if end > directory {
        return Err("ZIP member overlaps the central directory".into());
    }
    let flags = u16::from_le_bytes([fixed[6], fixed[7]]);
    let mut local = Bytes::at(fixed, 14)?;
    let crc = local.u32()?;
    let mut packed = u64::from(local.u32()?);
    let mut unpacked = u64::from(local.u32()?);
    if flags & 8 == 0 {
        if packed == u64::from(u32::MAX) || unpacked == u64::from(u32::MAX) {
            let mut extras = Bytes::at(extra, 0)?;
            let mut found = false;
            while !extras.remaining().is_empty() {
                let kind = extras.u16()?;
                let length = usize::from(extras.u16()?);
                let value = extras.take(length)?;
                if kind == 1 {
                    if found {
                        return Err("duplicate ZIP64 local size field".into());
                    }
                    found = true;
                    let mut sizes = Bytes::at(value, 0)?;
                    if unpacked == u64::from(u32::MAX) { unpacked = sizes.u64()?; }
                    if packed == u64::from(u32::MAX) { packed = sizes.u64()?; }
                }
            }
            if !found {
                return Err("ZIP64 local sizes are missing".into());
            }
        }
        if crc != file.crc32() || packed != file.compressed_size() || unpacked != file.size() {
            return Err("ZIP local/central size or checksum mismatch".into());
        }
        return Ok(end);
    }
    if (crc != 0 && crc != file.crc32())
        || !matches!(packed, 0 | 0xffff_ffff) && packed != file.compressed_size()
        || !matches!(unpacked, 0 | 0xffff_ffff) && unpacked != file.size()
    {
        return Err("ZIP descriptor member has inconsistent local fields".into());
    }
    let wide = packed == u64::from(u32::MAX) || unpacked == u64::from(u32::MAX)
        || central_fixed[20..24] == [255; 4] || central_fixed[24..28] == [255; 4];
    // The signature is optional, and a CRC may itself equal the signature.
    for signed in [true, false] {
        let length = (if wide { 20 } else { 12 }) + if signed { 4 } else { 0 };
        let Ok(data) = slice(&bytes[..directory], end, length) else { continue };
        let mut descriptor = Bytes::at(data, 0)?;
        if signed && descriptor.u32()? != 0x0807_4b50 { continue; }
        let crc = descriptor.u32()?;
        let packed = if wide { descriptor.u64()? } else { u64::from(descriptor.u32()?) };
        let unpacked = if wide { descriptor.u64()? } else { u64::from(descriptor.u32()?) };
        if crc == file.crc32() && packed == file.compressed_size() && unpacked == file.size() {
            return end_offset(end, length);
        }
    }
    Err("ZIP data descriptor is missing or inconsistent".into())
}

fn regular_rar_kind(unix: bool, attributes: u32, is_dir: bool) -> Result<(), String> {
    if unix {
        let kind = attributes & 0o170000;
        if kind != 0 && kind != if is_dir { 0o040000 } else { 0o100000 } {
            return Err("RAR links and special files are not supported".into());
        }
    } else if attributes & 0x0400 != 0 {
        return Err("RAR reparse points are not supported".into());
    }
    Ok(())
}

fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| char::from(*byte)).collect()
}

fn rar4_name(bytes: &[u8], unicode: bool) -> Result<String, String> {
    if !unicode {
        return Ok(latin1(bytes));
    }
    let Some(separator) = bytes.iter().position(|byte| *byte == 0) else {
        return String::from_utf8(bytes.to_vec()).map_err(|_| "invalid UTF-8 RAR4 member name".into());
    };
    let ansi = &bytes[..separator];
    let mut encoded = Bytes::at(&bytes[separator + 1..], 0)?;
    let high = u16::from(encoded.u8()?) << 8;
    let mut output = Vec::new();
    let mut flags = 0u8;
    let mut bits = 0;
    while !encoded.remaining().is_empty() {
        if bits == 0 {
            flags = encoded.u8()?;
            bits = 8;
        }
        let mode = flags >> 6;
        flags <<= 2;
        bits -= 2;
        match mode {
            0 => output.push(u16::from(encoded.u8()?)),
            1 => output.push(high | u16::from(encoded.u8()?)),
            2 => output.push(encoded.u16()?),
            3 => {
                let count = encoded.u8()?;
                let correction = if count & 0x80 != 0 { Some(encoded.u8()?) } else { None };
                let count = usize::from(count & 0x7f) + 2;
                let source = slice(ansi, output.len(), count)?;
                for byte in source {
                    output.push(match correction {
                        Some(correction) => high | u16::from(byte.wrapping_add(correction)),
                        None => u16::from(*byte),
                    });
                }
            }
            _ => unreachable!(),
        }
        if output.len() > MAX_NAME_BYTES {
            return Err("RAR4 Unicode name exceeds the path limit".into());
        }
    }
    String::from_utf16(&output).map_err(|_| "invalid UTF-16 RAR4 member name".into())
}

fn rar5_extra(bytes: &[u8], file: bool) -> Result<(), String> {
    let mut cursor = Bytes::at(bytes, 0)?;
    while !cursor.remaining().is_empty() {
        let size = offset(cursor.vint()?)?;
        let data = cursor.take(size)?;
        let mut record = Bytes::at(data, 0)?;
        let kind = record.vint()?;
        if file && matches!(kind, 1 | 4 | 5) {
            return Err("RAR5 encryption, file versions, and link/redirection records are not supported".into());
        }
    }
    Ok(())
}

struct CabFolder {
    start: usize,
    blocks: usize,
    compression: u16,
    required: usize,
}

struct CabSource {
    folder: usize,
    start: usize,
    end: usize,
    bytes: Vec<u8>,
}

fn cabinet(bytes: &[u8], budget: &mut Budget) -> Result<[Vec<u8>; 2], String> {
    let mut header = Bytes::at(bytes, 0)?;
    if header.take(4)? != b"MSCF" || header.u32()? != 0 {
        return Err("invalid CAB signature or reserved header field".into());
    }
    let cabinet_size = member_size(u64::from(header.u32()?))?;
    if cabinet_size != bytes.len() || cabinet_size < 36 || header.u32()? != 0 {
        return Err("CAB declared size differs from the supplied cabinet".into());
    }
    let files_offset = offset(u64::from(header.u32()?))?;
    if header.u32()? != 0 || header.u8()? != 3 || header.u8()? != 1 {
        return Err("unsupported CAB format version or reserved field".into());
    }
    let folder_count = usize::from(header.u16()?);
    let file_count = usize::from(header.u16()?);
    let flags = header.u16()?;
    header.u16()?;
    let cabinet_index = header.u16()?;
    if flags & !7 != 0 || flags & 3 != 0 || cabinet_index != 0 {
        return Err("CAB sets, continued cabinets, and unknown flags are not supported".into());
    }
    budget.entries(folder_count.checked_add(file_count).ok_or("CAB entry count overflow")?)?;
    let (folder_reserve, data_reserve) = if flags & 4 != 0 {
        let size = usize::from(header.u16()?);
        let folder_reserve = usize::from(header.u8()?);
        let data_reserve = usize::from(header.u8()?);
        if size > 60_000 {
            return Err("CAB header reserve exceeds the format limit".into());
        }
        header.take(size)?;
        (folder_reserve, data_reserve)
    } else {
        (0, 0)
    };
    let mut folders = Vec::with_capacity(folder_count);
    for _ in 0..folder_count {
        let start = offset(u64::from(header.u32()?))?;
        let blocks = usize::from(header.u16()?);
        let compression = header.u16()?;
        header.take(folder_reserve)?;
        if !matches!(compression, 0 | 1) {
            return Err(format!("unsupported CAB compression type {compression}; only STORED and MSZIP are supported"));
        }
        folders.push(CabFolder { start, blocks, compression, required: 0 });
    }
    if files_offset < header.pos {
        return Err("CAB file table overlaps the folder table".into());
    }
    let mut files = Bytes::at(bytes, files_offset)?;
    let mut names = HashSet::new();
    let mut sources: [Option<CabSource>; 2] = [None, None];
    for _ in 0..file_count {
        let size = member_size(u64::from(files.u32()?))?;
        let start = offset(u64::from(files.u32()?))?;
        let folder_index = usize::from(files.u16()?);
        files.take(4)?;
        let attributes = files.u16()?;
        let raw_name = files.cstring()?;
        budget.name(raw_name.len())?;
        let name = if attributes & 0x80 != 0 {
            std::str::from_utf8(raw_name).map_err(|_| "invalid UTF-8 CAB member name")?.to_owned()
        } else {
            latin1(raw_name)
        };
        let normalized = safe_name(&name, false)?;
        if !names.insert(normalized) {
            return Err("CAB contains duplicate member names".into());
        }
        if folder_index >= 0xfffd {
            return Err("continued CAB files require an unsupported cabinet set".into());
        }
        let folder = folders.get_mut(folder_index).ok_or("CAB member refers to a missing folder")?;
        let end = end_offset(start, size)?;
        member_size(end as u64)?;
        folder.required = folder.required.max(end);
        if let Some(index) = CAB_IDS.iter().position(|expected| *expected == name) {
            source_size(size as u64)?;
            if sources[index].is_some() {
                return Err("CAB contains an ambiguous source member".into());
            }
            let mut output = Vec::new();
            output.try_reserve_exact(size).map_err(|_| "could not allocate CAB source buffer")?;
            sources[index] = Some(CabSource { folder: folder_index, start, end, bytes: output });
        }
    }
    if sources.iter().any(Option::is_none) {
        return Err("CAB is missing the exact sn16k.tkn58 / sn16k.bin63 source IDs".into());
    }
    let files_end = files.pos;
    let mut ranges = Vec::with_capacity(folder_count);
    // The prefix is the last 32 KiB of this folder. Each CK block starts a
    // fresh DEFLATE stream with that prefix visible to distance references.
    // One spare output byte detects expansion beyond cbUncomp, including at
    // the 32 KiB block limit. A different folder starts with no prefix.
    let mut window = zeroed(CAB_BLOCK_BYTES * 2 + 1)?;
    let mut decoder = DecompressorOxide::new();
    for (folder_index, folder) in folders.iter().enumerate() {
        if folder.blocks != 0 && folder.start < files_end {
            return Err("CAB folder data overlaps its metadata".into());
        }
        let mut blocks = Bytes::at(bytes, folder.start)?;
        let mut expanded = 0usize;
        let mut history = 0usize;
        for _ in 0..folder.blocks {
            budget.block()?;
            let checksum_start = blocks.pos + 4;
            let checksum = blocks.u32()?;
            let packed = usize::from(blocks.u16()?);
            let unpacked = usize::from(blocks.u16()?);
            blocks.take(data_reserve)?;
            let data = blocks.take(packed)?;
            if unpacked == 0 || unpacked > CAB_BLOCK_BYTES {
                return Err("CAB split blocks and invalid expanded block lengths are not supported".into());
            }
            budget.bytes(unpacked as u64)?;
            let end = end_offset(expanded, unpacked)?;
            member_size(end as u64)?;
            // CFDATA reserve bytes are outside the checksum: the two size
            // fields and the compressed payload are checksummed separately.
            if checksum != 0 && (cab_checksum(&bytes[checksum_start..checksum_start + 4]) ^ cab_checksum(data)) != checksum {
                return Err("CAB data block checksum mismatch".into());
            }
            let decoded = if folder.compression == 0 {
                if packed != unpacked {
                    return Err("stored CAB block expanded length mismatch".into());
                }
                data
            } else {
                if packed > CAB_BLOCK_BYTES + 12 || !data.starts_with(b"CK") {
                    return Err("invalid or over-limit MSZIP block".into());
                }
                let output_end = end_offset(history, unpacked)?;
                inflate_into(&mut decoder, &data[2..], &mut window[..output_end + 1], history, unpacked)?;
                &window[history..output_end]
            };
            for source in sources.iter_mut().flatten().filter(|source| source.folder == folder_index) {
                let begin = source.start.max(expanded);
                let finish = source.end.min(end);
                if begin < finish {
                    source.bytes.extend_from_slice(&decoded[begin - expanded..finish - expanded]);
                }
            }
            if folder.compression == 1 {
                let total = history + unpacked;
                history = total.min(CAB_BLOCK_BYTES);
                window.copy_within(total - history..total, 0);
            }
            expanded = end;
        }
        if folder.required > expanded {
            return Err("CAB member lies outside its expanded folder".into());
        }
        if folder.blocks != 0 {
            ranges.push((folder.start, blocks.pos));
        }
    }
    disjoint(&mut ranges, "CAB folder data")?;
    let [token, bank] = sources;
    let token = token.ok_or("CAB token source is missing")?;
    let bank = bank.ok_or("CAB bank source is missing")?;
    if token.bytes.len() != token.end - token.start || bank.bytes.len() != bank.end - bank.start {
        return Err("CAB source member expanded length mismatch".into());
    }
    Ok([token.bytes, bank.bytes])
}

fn cab_checksum(bytes: &[u8]) -> u32 {
    let mut checksum = 0;
    let mut chunks = bytes.chunks_exact(4);
    for chunk in &mut chunks {
        checksum ^= u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }
    // CAB packs the remaining one to three bytes in reverse significance.
    let tail = chunks.remainder().iter().fold(0u32, |tail, byte| (tail << 8) | u32::from(*byte));
    checksum ^ tail
}
