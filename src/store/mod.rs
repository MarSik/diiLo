use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::{env, io};

use cache::CountChange::{ADD, NONE, REMOVE, SET};
use cache::{CountCache, CountCacheEntry};
use chrono::{DateTime, FixedOffset, Local};
use gray_matter::engine::YAML;
use gray_matter::{Matter, ParsedEntityStruct};
use log::debug;
use regex::Regex;
use serde::Serialize;
use serializer::LedgerSerializer;

pub mod cache;
pub mod conversions;
pub mod filter;
pub mod serializer;
pub mod types;

use crate::app::errs::AppError;

use types::{CountTracking, LedgerEntryDto};
pub use types::{
    LedgerEntry, LedgerEvent, LocationId, ObjectType, Part, PartId, PartMetadata, PartTypeId,
    ProjectId, SourceId,
};

pub struct Store {
    basepath: PathBuf,

    // Free parts in storage
    // added - how many were stored in the location (accumulating sum)
    // removed - how many were retrieved from location (accumulating sum)
    // required - how many should be in the location (before warning, order, etc.)
    count_cache: CountCache,
    // Ordered parts from external sources
    // added - how many were received from the source (bought)
    // removed - how many were sent to the source (returned, warranty, sold)
    source_cache: CountCache,
    // Used parts in projects
    // added - how many were soldered to a project
    // removed - how many were unsoldered from a project
    // required - how many are needed in a project
    project_cache: CountCache,

    // Uncommited ledger events
    // open_ledger: Vec<LedgerEntry>, // TODO allow recording events without persisting and then commit on user's command
    ledger_name: String,

    // Cached values
    parts: HashMap<PartTypeId, Part>,
    labels: HashMap<String, HashSet<String>>,

    // internal helper instances
    re_cleanup_name: Regex,
}

impl Store {
    pub fn new(basepath: PathBuf) -> anyhow::Result<Self> {
        fs::create_dir_all(basepath.join("md"))?;
        fs::create_dir_all(basepath.join("ledger"))?;

        let ledger_name = Self::ledger_name_now();

        Ok(Self {
            basepath: PathBuf::from(&basepath),
            count_cache: CountCache::new(),
            source_cache: CountCache::new(),
            project_cache: CountCache::new(),
            // open_ledger: Vec::new(),
            ledger_name,
            parts: HashMap::new(),
            labels: HashMap::new(),
            re_cleanup_name: regex::Regex::new("[\n\t _/.]+").unwrap(),
        })
    }

    pub fn load_part(path: impl AsRef<Path>) -> anyhow::Result<Part> {
        debug!("Loading: {:?}", path.as_ref());

        let input = fs::read_to_string(path.as_ref())?;
        let matter = Matter::<YAML>::new();
        //let mut entity = matter.parse_with_struct::<PartMetadata>(&input).unwrap();

        let parsed_entity = matter.parse(&input);
        let data: PartMetadata = if let Some(pod) = parsed_entity.data {
            pod.deserialize()?
        } else {
            PartMetadata::default()
        };

        let mut entity = ParsedEntityStruct {
            data,
            content: parsed_entity.content,
            excerpt: parsed_entity.excerpt,
            orig: parsed_entity.orig,
            matter: parsed_entity.matter,
        };

        // Make sure at least one type is defined
        if entity.data.types.is_empty() {
            entity.data.types.insert(ObjectType::Part);
        }

        Ok(Part {
            id: if let Some(id) = &entity.data.id {
                id.as_str().into()
            } else {
                Self::part_path_to_id(path.as_ref())
            },
            filename: Some(PathBuf::from(path.as_ref())),
            metadata: entity.data,
            content: entity.content,
        })
    }

    // Takes basename and strips the extension
    fn part_path_to_id(p: impl AsRef<Path>) -> Rc<str> {
        p.as_ref()
            .file_name()
            .and_then(|os| os.to_str())
            .unwrap()
            .split('.')
            .next()
            .unwrap()
            .into()
    }

    pub fn name_to_id(&self, name: &str) -> String {
        self.re_cleanup_name
            .replace_all(name.trim(), "_")
            .to_string()
    }

    // Drop information caches and reload all parts from the stored
    // markdown files.
    pub fn scan_parts(&mut self) -> anyhow::Result<()> {
        self.parts.clear();
        self.labels.clear();

        let dir = walkdir::WalkDir::new(Path::new(&self.basepath).join("md"));
        for f in dir.into_iter().flatten() {
            if f.file_type().is_file() {
                let part = Self::load_part(f.path())?;
                self.insert_part_to_cache(part);
            }
        }

        Ok(())
    }

    pub(crate) fn insert_part_to_cache(&mut self, part: Part) {
        // Populate label caches
        for (k, vs) in &part.metadata.labels {
            if !self.labels.contains_key(k) {
                self.labels.insert(k.clone(), HashSet::new());
            }
            for v in vs {
                self.labels.get_mut(k).map(|svs| svs.insert(v.clone()));
            }
        }

        // Populate all part cache
        self.parts.insert(part.id.clone(), part);
    }

    pub fn store_part(&mut self, part: &mut Part) -> Result<(), AppError> {
        if part.filename.is_none() {
            part.filename = Some(self.basepath.join("md").join(part.id.to_string()));
            part.filename.as_mut().unwrap().set_extension("md");
        }

        if part.metadata.id.is_none() {
            part.metadata.id = Some(part.id.to_string());
        }

        let mut f =
            File::create(part.filename.as_ref().unwrap().clone()).map_err(AppError::IoError)?;
        f.write_all(b"---\n").map_err(AppError::IoError)?;
        serde_yaml::to_writer(&f, &part.metadata).map_err(AppError::ObjectSerializationError)?;
        f.write_all(b"\n---\n").map_err(AppError::IoError)?;
        f.write_all(part.content.as_bytes())
            .map_err(AppError::IoError)?;

        Ok(())
    }

    // Initialize new ledger that will be used until program closes
    // or until create_ledger is called again
    pub fn open_ledger(&mut self, name: Option<&str>) -> Result<File, io::Error> {
        self.ledger_name = name.unwrap_or(Self::ledger_name_now().as_str()).to_string();
        let f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.basepath.join("ledger").join(self.ledger_name.as_str()))?;
        Ok(f)
    }

    // Store one event to the ledger (persistently)
    pub fn record_event(&mut self, entry: &LedgerEntry) -> Result<(), AppError> {
        let dto: LedgerEntryDto = entry.into();

        let f = self
            .open_ledger(Some(self.ledger_name.clone().as_str()))
            .map_err(AppError::IoError)?;
        let mut ser = LedgerSerializer::from_file(f);
        dto.serialize(&mut ser)
            .map_err(AppError::LedgerSerializationError)?;
        Ok(())
    }

    fn load_events_from_file(&mut self, filename: &str) -> anyhow::Result<Vec<LedgerEntry>> {
        // TODO This is not effective, but allows handling parsing errors in the loop
        //      Direct iterator based approach would be better.
        let mut output = Vec::new();
        let mut loaded = Vec::new();

        let f = File::open(filename)?;
        let f = BufReader::new(f);
        let lines = f.lines();
        for l in lines.map_while(Result::ok) {
            let v = serde_keyvalue::from_key_values::<LedgerEntryDto>(l.as_str())?;
            loaded.push(v);
        }

        // Inject time to each entry based on the last known time
        let mut last_t = DateTime::<FixedOffset>::MIN_UTC.fixed_offset();
        for v in loaded {
            debug!("Loaded ledger {:?}", v);
            let mut o: LedgerEntry = (&v).into(); // TODO change to better API that allows passing lat time in
            if v.time.is_none() {
                o.t = last_t
            }
            last_t = o.t;

            // Enhance PartId with piece size if needed
            if let Some(part) = self.part_by_id(o.part.part_type()) {
                if part.metadata.track == CountTracking::Pieces
                    && o.part.piece_size_option().is_none()
                {
                    o.part = o.part.piece(part.metadata.piece_size.max(1));
                }
            }

            debug!("Converted to event {:?}", o);
            output.push(o);
        }

        Ok(output)
    }

    // Drop all count caches and reload all ledgers from files
    pub fn load_events(&mut self) -> anyhow::Result<Vec<LedgerEntry>> {
        let mut output = Vec::new();

        let dir = std::fs::read_dir(Path::new(&self.basepath).join("ledger"))?;
        for f in dir.flatten() {
            if let Ok(ft) = f.file_type() {
                if ft.is_file() {
                    let events = self.load_events_from_file(f.path().to_str().unwrap())?;
                    output.extend(events);
                }
            }
        }

        // sort by time
        output.sort_by_key(|f| f.t);

        // repopulate count caches
        self.count_cache.clear();
        self.source_cache.clear();

        for e in &output {
            self.update_count_cache(e);
        }

        Ok(vec![])
    }

    pub fn update_count_cache(&mut self, e: &LedgerEntry) {
        match &e.ev {
            LedgerEvent::TakeFrom(location) => {
                // Keep serial or lot number, but handle pieces in case the count is not a multiple of piece size
                let (count, keep_count) = if let PartId::Piece(_, s) = e.part {
                    let mut full_pieces = e.count / s;
                    let remainder = e.count % s;
                    let keep = if remainder > 0 {
                        // One extra piece needs to be cut
                        full_pieces += 1;
                        s - remainder
                    } else {
                        0
                    };

                    (full_pieces * s, keep)
                } else {
                    (e.count, 0)
                };

                self.count_cache
                    .update_count(&e.part, location, NONE, ADD(count), NONE);
                if keep_count > 0 {
                    self.count_cache.update_count(
                        &e.part.piece(keep_count),
                        location,
                        ADD(keep_count),
                        NONE,
                        NONE,
                    );
                }
            }
            LedgerEvent::StoreTo(location) => {
                // Keep serial or lot number, but handle pieces in case the count is not a multiple of piece size
                let (full_count, partial_count) = if let PartId::Piece(_, s) = e.part {
                    let full_pieces = e.count / s;
                    let remainder = e.count % s;
                    (full_pieces * s, remainder)
                } else {
                    (e.count, 0)
                };

                self.count_cache
                    .update_count(&e.part, location, ADD(full_count), NONE, NONE);
                if partial_count > 0 {
                    self.count_cache.update_count(
                        &e.part.piece(partial_count),
                        location,
                        ADD(partial_count),
                        NONE,
                        NONE,
                    );
                }
            }
            LedgerEvent::ForceCount(location) => {
                // TODO handle partial count for Pieces
                let count = self.count_cache.get_count(&e.part, location);
                let (new_added, new_removed) = if (e.count as isize) > count.count() {
                    (count.removed() + e.count, count.removed())
                } else {
                    (count.added(), count.added().saturating_sub(e.count))
                };

                self.count_cache.set_count(CountCacheEntry::new(
                    PartId::clone(&e.part),
                    LocationId::clone(location),
                    new_added,
                    new_removed,
                    count.required(),
                ));
            }
            LedgerEvent::RequireIn(location) => {
                // Requirement of type does not specify an exact part or piece, just the type
                self.count_cache
                    .update_count(&e.part.simple(), location, NONE, NONE, SET(e.count));
            }
            LedgerEvent::RequireInProject(project) => {
                // Requirement of type does not specify an exact part or piece, just the type
                self.project_cache.update_count(
                    &e.part.simple(),
                    project,
                    NONE,
                    NONE,
                    SET(e.count),
                );
            }
            LedgerEvent::OrderFrom(source) => {
                // Order of type does not specify an exact part or piece, just the type
                self.source_cache.update_count(
                    &e.part.simple(),
                    &source.into(),
                    NONE,
                    NONE,
                    ADD(e.count),
                );
            }
            LedgerEvent::CancelOrderFrom(source) => {
                // Order of type does not specify an exact part or piece, just the type
                self.source_cache.update_count(
                    &e.part.simple(),
                    &source.into(),
                    NONE,
                    NONE,
                    REMOVE(e.count),
                );
            }
            LedgerEvent::DeliverFrom(source) => {
                // Delivery could contain a serial number, keep it
                self.source_cache
                    .update_count(&e.part, &source.into(), ADD(e.count), NONE, NONE);
            }
            LedgerEvent::ReturnTo(source) => {
                // Return could contain a serial number, keep it
                self.source_cache
                    .update_count(&e.part, &source.into(), NONE, ADD(e.count), NONE);
            }
            LedgerEvent::UnsolderFrom(project) => {
                // Keep serial or lot number, but handle pieces in case the count is not a multiple of piece size
                let (count, keep_count) = if let PartId::Piece(_, s) = e.part {
                    let mut full_pieces = e.count / s;
                    let remainder = e.count % s;
                    let keep = if remainder > 0 {
                        // One extra piece needs to be cut
                        full_pieces += 1;
                        s - remainder
                    } else {
                        0
                    };

                    (full_pieces * s, keep)
                } else {
                    (e.count, 0)
                };

                self.project_cache
                    .update_count(&e.part, project, NONE, ADD(count), NONE);
                if keep_count > 0 {
                    self.project_cache.update_count(
                        &e.part.piece(keep_count),
                        project,
                        ADD(keep_count),
                        NONE,
                        NONE,
                    );
                }
            }
            LedgerEvent::SolderTo(project) => {
                // Keep serial or lot number, but handle pieces in case the count is not a multiple of piece size
                let (full_count, partial_count) = if let PartId::Piece(_, s) = e.part {
                    let full_pieces = e.count / s;
                    let remainder = e.count % s;
                    (full_pieces * s, remainder)
                } else {
                    (e.count, 0)
                };

                self.project_cache
                    .update_count(&e.part, project, ADD(full_count), NONE, NONE);
                if partial_count > 0 {
                    self.project_cache.update_count(
                        &e.part.piece(partial_count),
                        project,
                        ADD(partial_count),
                        NONE,
                        NONE,
                    );
                }
            }
        }
    }

    fn ledger_name_now() -> String {
        Local::now().format("%Y-%m-%d-%H-%M.txt").to_string()
    }

    // Return all objects, includes all types (parts, locations, sources, etc.)
    pub fn all_objects(&self) -> &HashMap<PartTypeId, Part> {
        &self.parts
    }

    pub fn all_label_keys(&self) -> Vec<(String, usize)> {
        self.labels
            .keys()
            .map(|k| (k.clone(), self.labels.get(k).unwrap().len()))
            .collect()
    }

    pub fn all_label_values(&self, k: &str) -> Vec<(String, usize)> {
        let default = HashSet::with_capacity(0);
        self.labels
            .get(k)
            .unwrap_or(&default)
            .iter()
            .map(|v| (v.clone(), self.parts_by_label(k, v).len()))
            .collect()
    }

    pub fn part_by_id(&self, part_id: &PartTypeId) -> Option<&Part> {
        self.parts.get(part_id)
    }

    pub fn parts_by_location(&self, location_id: &LocationId) -> Vec<(&Part, CountCacheEntry)> {
        let mut out = Vec::new();

        for en in self.count_by_location(location_id) {
            if let Some(p) = self.parts.get(en.part().part_type()) {
                out.push((p, en));
            }
        }

        out
    }

    pub fn parts_by_source(&self, source_id: &SourceId) -> Vec<(&Part, CountCacheEntry)> {
        let mut out = Vec::new();

        for en in self.count_by_source(source_id) {
            if let Some(p) = self.parts.get(en.part().part_type()) {
                out.push((p, en));
            }
        }

        out
    }

    pub fn locations_by_part(&self, part_id: &PartId) -> Vec<(&Part, CountCacheEntry)> {
        let mut out = Vec::new();

        for en in self.count_by_part(part_id) {
            if let Some(p) = self.parts.get(en.location().part_type()) {
                out.push((p, en));
            }
        }

        out
    }

    pub fn parts_by_label(&self, key: &str, value: &str) -> Vec<&Part> {
        self.parts
            .values()
            .filter(|p| {
                p.metadata
                    .labels
                    .get_vec(key)
                    .map(|vs| vs.contains(&value.to_string()))
                    .unwrap_or(false)
            })
            .collect()
    }

    pub fn count_by_part(&self, part_id: &PartId) -> Vec<CountCacheEntry> {
        self.count_cache.by_part(part_id)
    }

    pub fn count_by_part_type(&self, part_type_id: &PartTypeId) -> Vec<CountCacheEntry> {
        self.count_cache.by_part_type(part_type_id)
    }

    pub fn count_by_location(&self, location_id: &LocationId) -> Vec<CountCacheEntry> {
        self.count_cache.by_location(location_id)
    }

    pub fn count_by_location_type(&self, location_id: &PartTypeId) -> Vec<CountCacheEntry> {
        self.count_cache.by_location_type(location_id)
    }

    pub fn count_by_source(&self, source_id: &SourceId) -> Vec<CountCacheEntry> {
        self.source_cache.by_location(&source_id.into())
    }

    pub fn add_label_key(&mut self, label_key: &str) {
        if !self.labels.contains_key(label_key) {
            self.labels.insert(label_key.to_string(), HashSet::new());
        }
    }

    pub fn add_label(&mut self, label_key: &str, label_value: &str) {
        self.add_label_key(label_key);
        self.labels
            .get_mut(label_key)
            .unwrap()
            .insert(label_value.to_string());
    }

    pub fn show_empty_in_location(
        &mut self,
        part_id: &PartId,
        location_id: &LocationId,
        show_empty: bool,
    ) {
        self.count_cache
            .show_empty(part_id, location_id, show_empty);
    }

    pub fn show_empty_in_source(
        &mut self,
        part_id: &PartId,
        location_id: &LocationId,
        show_empty: bool,
    ) {
        self.source_cache
            .show_empty(part_id, location_id, show_empty);
    }

    pub(crate) fn count_by_part_location(
        &self,
        part_id: &PartId,
        location_id: &LocationId,
    ) -> CountCacheEntry {
        self.count_cache.get_count(part_id, location_id)
    }

    pub(crate) fn count_by_part_project(
        &self,
        part_id: &PartId,
        project_id: &PartId,
    ) -> CountCacheEntry {
        self.project_cache.get_count(part_id, project_id)
    }

    pub(crate) fn count_by_part_source(
        &self,
        part_id: &PartId,
        source_id: &SourceId,
    ) -> CountCacheEntry {
        self.source_cache.get_count(part_id, &source_id.into())
    }

    pub(crate) fn count_by_project(&self, project_id: &LocationId) -> Vec<CountCacheEntry> {
        self.project_cache.by_location(project_id)
    }

    pub(crate) fn count_by_project_type(&self, project_id: &PartTypeId) -> Vec<CountCacheEntry> {
        self.project_cache.by_location_type(project_id)
    }

    pub(crate) fn parts_by_project(
        &self,
        project_id: &LocationId,
    ) -> Vec<(&Part, CountCacheEntry)> {
        let mut out = Vec::new();

        for en in self.count_by_project(project_id) {
            if let Some(p) = self.parts.get(en.part().part_type()) {
                out.push((p, en));
            }
        }

        out
    }

    pub fn show_empty_in_project(
        &mut self,
        part_id: &PartId,
        project_id: &PartId,
        show_empty: bool,
    ) -> CountCacheEntry {
        self.project_cache
            .show_empty(part_id, project_id, show_empty)
    }

    pub(crate) fn get_projects_by_part(&self, part_id: &PartId) -> Vec<CountCacheEntry> {
        self.project_cache.by_part(part_id)
    }

    pub(crate) fn get_sources_by_part(&self, part_id: &PartId) -> Vec<CountCacheEntry> {
        self.source_cache.by_part(part_id)
    }

    pub(crate) fn remove(&mut self, part_type_id: &PartTypeId) -> Result<(), AppError> {
        let part = self
            .parts
            .get(part_type_id)
            .ok_or(AppError::NoSuchObject(part_type_id.to_string()))?;

        // Delete file
        part.filename
            .as_ref()
            .map(fs::remove_file)
            .unwrap_or(Ok(()))
            .map_err(AppError::IoError)?;
        self.parts.remove(part_type_id);

        // Clear caches
        self.count_cache.remove(part_type_id);
        self.source_cache.remove(part_type_id);
        self.project_cache.remove(part_type_id);

        Ok(())
    }
}

// Compute proper storage path based on Free Desktop environment variables.
// This is currently Linux only
pub fn default_store_path() -> anyhow::Result<PathBuf> {
    let xdg_path = env::var("XDG_DATA_HOME");
    if let Ok(xdg_path) = xdg_path {
        Ok(PathBuf::from(xdg_path).join("diilo"))
    } else {
        let home_path = env::var("HOME")?;
        Ok(PathBuf::from(home_path)
            .join(".local")
            .join("share")
            .join("diilo"))
    }
}
