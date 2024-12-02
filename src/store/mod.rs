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
use multimap::MultiMap;
use regex::Regex;
use serde::Serialize;
use serializer::LedgerSerializer;

pub mod cache;
pub mod filter;
pub mod serializer;

use crate::app::errs::AppError;
use crate::store::serializer::{deserialize_labels, serialize_labels};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ObjectType {
    Part,
    Source,
    Project,
    Location,
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct PartMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(default)]
    pub name: String,

    #[serde(alias = "mfgid")]
    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub manufacturer_id: String,

    #[serde(skip_serializing_if = "String::is_empty")]
    #[serde(default)]
    pub manufacturer: String,

    #[serde(default)]
    #[serde(serialize_with = "serialize_labels")]
    #[serde(deserialize_with = "deserialize_labels")]
    pub labels: MultiMap<String, String>,

    #[serde(default)]
    #[serde(alias = "attrs", alias = "attr")]
    #[serde(serialize_with = "serialize_labels")]
    #[serde(deserialize_with = "deserialize_labels")]
    pub attributes: MultiMap<String, String>,

    #[serde(default)]
    pub types: HashSet<ObjectType>,

    #[serde(default)]
    pub summary: String,
}

#[derive(Debug, Clone, Default)]
pub struct Part {
    pub id: PartId,
    pub filename: Option<PathBuf>,
    pub metadata: PartMetadata,
    pub content: String,
}

pub fn is_false(v: &bool) -> bool {
    !*v
}

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
pub(crate) struct LedgerEntryDto {
    #[serde(alias = "t")]
    #[serde(skip_serializing_if = "Option::is_none")]
    time: Option<String>,

    #[serde(alias = "tx")]
    #[serde(skip_serializing_if = "Option::is_none")]
    transaction: Option<String>,

    #[serde(alias = "n", alias = "c")]
    count: usize,

    #[serde(rename = "part")]
    #[serde(skip_serializing_if = "String::is_empty")]
    part_id: String,
    #[serde(
        rename = "location",
        alias = "destination",
        alias = "dst",
        alias = "to"
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    location_id: Option<String>,

    #[serde(rename = "project", alias = "proj")]
    #[serde(skip_serializing_if = "Option::is_none")]
    project_id: Option<String>,

    #[serde(rename = "source", alias = "from", alias = "src", alias = "fr")]
    #[serde(skip_serializing_if = "Option::is_none")]
    source_id: Option<String>,

    #[serde(rename = "take", alias = "move", alias = "m", alias = "-")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    cmd_take: bool,
    #[serde(rename = "store", alias = "receive", alias = "a", alias = "+")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    cmd_store: bool,
    #[serde(rename = "require", alias = "req", alias = "?")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    cmd_require: bool,
    #[serde(rename = "solder", alias = "s")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    cmd_solder: bool,
    #[serde(rename = "unsolder", alias = "u")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    cmd_unsolder: bool,
    #[serde(rename = "order", alias = "o")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    cmd_order: bool,
    #[serde(rename = "cancel", alias = "co")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    cmd_cancel_order: bool,
    #[serde(rename = "deliver", alias = "d")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    cmd_deliver: bool,
    #[serde(rename = "return", alias = "ret", alias = "send")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    cmd_return: bool,
    #[serde(rename = "correct", alias = "set", alias = "=")]
    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    cmd_set: bool,
}

pub type PartId = Rc<str>;
pub type LocationId = PartId;
pub type SourceId = Rc<str>;

#[derive(Debug)]
pub struct LedgerEntry {
    pub t: DateTime<FixedOffset>,
    pub count: usize,
    pub part: PartId,
    pub ev: LedgerEvent,
}

impl From<&LedgerEntryDto> for LedgerEntry {
    fn from(val: &LedgerEntryDto) -> LedgerEntry {
        //The parse_datetime call takes 5 ms and is extremely slow!
        //let t = self.time.clone().map(parse_datetime).unwrap().unwrap();
        let t = val
            .time
            .as_deref()
            .map(DateTime::parse_from_rfc3339)
            .unwrap_or_else(|| Ok(Local::now().fixed_offset()))
            .unwrap();

        if val.cmd_store {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::StoreTo(val.location_id.clone().unwrap().into()),
            }
        } else if val.cmd_take {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::TakeFrom(val.location_id.clone().unwrap().into()),
            }
        } else if val.cmd_deliver {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::DeliverFrom(
                    val.source_id
                        .as_ref()
                        .or(val.location_id.as_ref())
                        .unwrap()
                        .as_str()
                        .into(),
                ),
            }
        } else if val.cmd_return {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::ReturnTo(
                    val.source_id
                        .as_ref()
                        .or(val.location_id.as_ref())
                        .unwrap()
                        .as_str()
                        .into(),
                ),
            }
        } else if val.cmd_order {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::OrderFrom(
                    val.source_id
                        .as_ref()
                        .or(val.location_id.as_ref())
                        .unwrap()
                        .as_str()
                        .into(),
                ),
            }
        } else if val.cmd_cancel_order {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::CancelOrderFrom(
                    val.source_id
                        .as_ref()
                        .or(val.location_id.as_ref())
                        .unwrap()
                        .as_str()
                        .into(),
                ),
            }
        } else if val.cmd_require && val.location_id.is_some() {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::RequireIn(val.location_id.clone().unwrap().into()),
            }
        } else if val.cmd_require && val.project_id.is_some() {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::RequireInProject(val.project_id.clone().unwrap().into()),
            }
        } else if val.cmd_require && val.source_id.is_some() {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::OrderFrom(val.source_id.clone().unwrap().into()),
            }
        } else if val.cmd_solder {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::SolderTo(
                    val.project_id
                        .as_ref()
                        .or(val.location_id.as_ref())
                        .unwrap()
                        .as_str()
                        .into(),
                ),
            }
        } else if val.cmd_unsolder {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::UnsolderFrom(
                    val.project_id
                        .as_ref()
                        .or(val.location_id.as_ref())
                        .unwrap()
                        .as_str()
                        .into(),
                ),
            }
        } else if val.cmd_set {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::ForceCount(val.location_id.clone().unwrap().into()),
            }
        } else {
            LedgerEntry {
                t,
                count: val.count,
                part: val.part_id.as_str().into(),
                ev: LedgerEvent::TakeFrom(val.location_id.clone().unwrap().into()),
            }
        }
    }
}

#[derive(Debug)]
pub enum LedgerEvent {
    TakeFrom(LocationId),
    StoreTo(LocationId),
    ForceCount(LocationId),
    RequireIn(LocationId),
    OrderFrom(SourceId),
    CancelOrderFrom(SourceId),
    DeliverFrom(SourceId),
    ReturnTo(SourceId),
    UnsolderFrom(LocationId),
    SolderTo(LocationId),
    RequireInProject(LocationId),
}

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
    parts: HashMap<PartId, Part>,
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
        let dto = match &entry.ev {
            LedgerEvent::TakeFrom(location) => LedgerEntryDto {
                time: Some(entry.t.to_rfc3339()),
                transaction: None,
                count: entry.count,
                part_id: entry.part.to_string(),
                location_id: Some(location.to_string()),
                cmd_take: true, // TODO check if location is a project -> unsolder
                ..Default::default()
            },
            LedgerEvent::StoreTo(location) => LedgerEntryDto {
                time: Some(entry.t.to_rfc3339()),
                transaction: None,
                count: entry.count,
                part_id: entry.part.to_string(),
                location_id: Some(location.to_string()),
                cmd_store: true, // TODO check if location is a project -> solder
                ..Default::default()
            },
            LedgerEvent::UnsolderFrom(location) => LedgerEntryDto {
                time: Some(entry.t.to_rfc3339()),
                transaction: None,
                count: entry.count,
                part_id: entry.part.to_string(),
                location_id: Some(location.to_string()),
                cmd_unsolder: true,
                ..Default::default()
            },
            LedgerEvent::SolderTo(location) => LedgerEntryDto {
                time: Some(entry.t.to_rfc3339()),
                transaction: None,
                count: entry.count,
                part_id: entry.part.to_string(),
                location_id: Some(location.to_string()),
                cmd_solder: true,
                ..Default::default()
            },
            LedgerEvent::ForceCount(location) => LedgerEntryDto {
                time: Some(entry.t.to_rfc3339()),
                transaction: None,
                count: entry.count,
                part_id: entry.part.to_string(),
                location_id: Some(location.to_string()),
                cmd_set: true,
                ..Default::default()
            },
            LedgerEvent::RequireIn(location) => LedgerEntryDto {
                time: Some(entry.t.to_rfc3339()),
                transaction: None,
                count: entry.count,
                part_id: entry.part.to_string(),
                location_id: Some(location.to_string()),
                cmd_require: true,
                ..Default::default()
            },
            LedgerEvent::RequireInProject(project_id) => LedgerEntryDto {
                time: Some(entry.t.to_rfc3339()),
                transaction: None,
                count: entry.count,
                part_id: entry.part.to_string(),
                project_id: Some(project_id.to_string()),
                cmd_require: true,
                ..Default::default()
            },
            LedgerEvent::ReturnTo(source_id) => LedgerEntryDto {
                time: Some(entry.t.to_rfc3339()),
                transaction: None,
                count: entry.count,
                part_id: entry.part.to_string(),
                source_id: Some(source_id.to_string()),
                cmd_return: true,
                ..Default::default()
            },
            LedgerEvent::DeliverFrom(source) => LedgerEntryDto {
                time: Some(entry.t.to_rfc3339()),
                transaction: None,
                count: entry.count,
                part_id: entry.part.to_string(),
                source_id: Some(source.to_string()),
                cmd_deliver: true,
                ..Default::default()
            },
            LedgerEvent::OrderFrom(source) => LedgerEntryDto {
                time: Some(entry.t.to_rfc3339()),
                transaction: None,
                count: entry.count,
                part_id: entry.part.to_string(),
                source_id: Some(source.to_string()),
                cmd_order: true,
                ..Default::default()
            },
            LedgerEvent::CancelOrderFrom(source) => LedgerEntryDto {
                time: Some(entry.t.to_rfc3339()),
                transaction: None,
                count: entry.count,
                part_id: entry.part.to_string(),
                source_id: Some(source.to_string()),
                cmd_cancel_order: true,
                ..Default::default()
            },
        };

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
                self.count_cache
                    .update_count(&e.part, location, NONE, ADD(e.count), NONE);
            }
            LedgerEvent::StoreTo(location) => {
                self.count_cache
                    .update_count(&e.part, location, ADD(e.count), NONE, NONE);
            }
            LedgerEvent::ForceCount(location) => {
                let count = self.count_cache.get_count(&e.part, location);
                let (new_added, new_removed) = if (e.count as isize) > count.count() {
                    (count.removed() + e.count, count.removed())
                } else {
                    (count.added(), count.added().saturating_sub(e.count))
                };

                self.count_cache.set_count(CountCacheEntry::new(
                    Rc::clone(&e.part),
                    Rc::clone(location),
                    new_added,
                    new_removed,
                    count.required(),
                ));
            }
            LedgerEvent::RequireIn(location) => {
                self.count_cache
                    .update_count(&e.part, location, NONE, NONE, SET(e.count));
            }
            LedgerEvent::RequireInProject(project) => {
                self.project_cache
                    .update_count(&e.part, project, NONE, NONE, SET(e.count));
            }
            LedgerEvent::OrderFrom(source) => {
                self.source_cache
                    .update_count(&e.part, source, NONE, NONE, ADD(e.count));
            }
            LedgerEvent::CancelOrderFrom(source) => {
                self.source_cache
                    .update_count(&e.part, source, NONE, NONE, REMOVE(e.count));
            }
            LedgerEvent::DeliverFrom(source) => {
                self.source_cache
                    .update_count(&e.part, source, ADD(e.count), NONE, NONE);
            }
            LedgerEvent::ReturnTo(source) => {
                self.source_cache
                    .update_count(&e.part, source, NONE, ADD(e.count), NONE);
            }
            LedgerEvent::UnsolderFrom(project) => {
                self.count_cache
                    .update_count(&e.part, project, NONE, ADD(e.count), NONE);
            }
            LedgerEvent::SolderTo(project) => {
                self.count_cache
                    .update_count(&e.part, project, ADD(e.count), NONE, NONE);
            }
        }
    }

    fn ledger_name_now() -> String {
        Local::now().format("%Y-%m-%d-%H-%M.txt").to_string()
    }

    // Return all objects, includes all types (parts, locations, sources, etc.)
    pub fn all_objects(&self) -> &HashMap<PartId, Part> {
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

    pub fn part_by_id(&self, part_id: &PartId) -> Option<&Part> {
        self.parts.get(part_id)
    }

    pub fn parts_by_location(&self, location_id: &LocationId) -> Vec<(&Part, CountCacheEntry)> {
        let mut out = Vec::new();

        for en in self.count_by_location(location_id) {
            if let Some(p) = self.parts.get(en.part()) {
                out.push((p, en));
            }
        }

        out
    }

    pub fn parts_by_source(&self, source_id: &LocationId) -> Vec<(&Part, CountCacheEntry)> {
        let mut out = Vec::new();

        for en in self.count_by_source(source_id) {
            if let Some(p) = self.parts.get(en.part()) {
                out.push((p, en));
            }
        }

        out
    }

    pub fn locations_by_part(&self, part_id: &PartId) -> Vec<(&Part, CountCacheEntry)> {
        let mut out = Vec::new();

        for en in self.count_by_part(part_id) {
            if let Some(p) = self.parts.get(en.location()) {
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

    pub fn count_by_location(&self, location_id: &LocationId) -> Vec<CountCacheEntry> {
        self.count_cache.by_location(location_id)
    }

    pub fn count_by_source(&self, source_id: &str) -> Vec<CountCacheEntry> {
        self.source_cache.by_location(source_id)
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

    pub(crate) fn get_by_location(
        &self,
        part_id: &PartId,
        location_id: &LocationId,
    ) -> CountCacheEntry {
        self.count_cache.get_count(part_id, location_id)
    }

    pub(crate) fn get_by_project(&self, part_id: &PartId, project_id: &PartId) -> CountCacheEntry {
        self.project_cache.get_count(part_id, project_id)
    }

    pub(crate) fn get_by_source(&self, part_id: &PartId, source_id: &SourceId) -> CountCacheEntry {
        self.source_cache.get_count(part_id, source_id)
    }

    pub(crate) fn count_by_project(&self, p_id: &str) -> Vec<CountCacheEntry> {
        self.project_cache.by_location(p_id)
    }

    pub(crate) fn parts_by_project(&self, project_id: &str) -> Vec<(&Part, CountCacheEntry)> {
        let mut out = Vec::new();

        for en in self.count_by_project(project_id) {
            if let Some(p) = self.parts.get(en.part()) {
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

    pub(crate) fn get_projects_by_part(&self, project_id: &str) -> Vec<CountCacheEntry> {
        self.project_cache.by_location(project_id)
    }

    pub(crate) fn get_sources_by_part(&self, source_id: &str) -> Vec<CountCacheEntry> {
        self.source_cache.by_location(source_id)
    }

    pub(crate) fn remove(&mut self, part_id: &str) -> Result<(), AppError> {
        let part = self
            .parts
            .get(part_id)
            .ok_or(AppError::NoSuchObject(part_id.to_string()))?;

        // Delete file
        part.filename
            .as_ref()
            .map(fs::remove_file)
            .unwrap_or(Ok(()))
            .map_err(AppError::IoError)?;
        self.parts.remove(part_id);

        // Clear caches
        self.count_cache.remove(part_id);
        self.source_cache.remove(part_id);
        self.project_cache.remove(part_id);

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
