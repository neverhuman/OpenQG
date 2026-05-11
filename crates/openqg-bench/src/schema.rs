use crate::util::{generated_json_text, sha256_digest, write_generated_json};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

const SPEC_ROOT: &str = "contracts/specs";
const GENERATED_ROOT: &str = "contracts/generated/schemas";

#[derive(Debug, Deserialize)]
struct Registry {
    tool: String,
    sync_command: String,
    entries: Vec<RegistryEntry>,
}

#[derive(Debug, Deserialize, Clone)]
struct RegistryEntry {
    name: String,
    source: PathBuf,
    output: PathBuf,
}

#[derive(Debug, serde::Serialize)]
struct SchemaWitness {
    name: String,
    source: String,
    output: String,
    tool: String,
    command: String,
    checked: bool,
    source_sha256: String,
    output_sha256: String,
}

pub fn sync(root: &Path, registry: &Path) -> Result<()> {
    let registry = load_registry(root, registry)?;
    for entry in &registry.entries {
        sync_entry(root, &registry, entry)?;
    }
    println!("synced {} schema files", registry.entries.len());
    Ok(())
}

pub fn check(root: &Path, registry: &Path) -> Result<()> {
    let registry = load_registry(root, registry)?;
    let spec_dir = root.join(SPEC_ROOT);
    let generated_dir = root.join(GENERATED_ROOT);

    ensure_all_specs_are_registered(&registry, &spec_dir)?;
    ensure_all_generated_outputs_are_registered(&registry, &generated_dir)?;

    let mut checked = 0usize;
    for entry in &registry.entries {
        check_entry(root, &registry, entry)?;
        checked += 1;
    }
    println!("checked {checked} schema files");
    Ok(())
}

fn sync_entry(root: &Path, registry: &Registry, entry: &RegistryEntry) -> Result<()> {
    let schema = load_schema(root, entry)?;
    let json = yaml_to_json(schema)?;
    let output = resolve(root, &entry.output);
    write_generated_json(&output, &registry.tool, &registry.sync_command, &json)?;
    Ok(())
}

fn check_entry(root: &Path, registry: &Registry, entry: &RegistryEntry) -> Result<()> {
    let schema = load_schema(root, entry)?;
    let source_path = resolve(root, &entry.source);
    let source_sha256 = sha256_digest(&fs::read(&source_path)?);
    let json = yaml_to_json(schema)?;
    let expected = generated_json_text(&registry.tool, &registry.sync_command, &json)?;
    let output = resolve(root, &entry.output);
    let actual = fs::read_to_string(&output)
        .with_context(|| format!("read generated schema {}", output.display()))?;
    if actual != expected {
        bail!(
            "schema {} ({}) is out of sync with {}",
            entry.name,
            output.display(),
            entry.source.display()
        );
    }
    write_witness(root, registry, entry, &actual, source_sha256)?;
    Ok(())
}

fn load_registry(root: &Path, registry: &Path) -> Result<Registry> {
    let path = resolve(root, registry);
    let text =
        fs::read_to_string(&path).with_context(|| format!("read registry {}", path.display()))?;
    let registry: Registry = serde_yaml::from_str(&text)
        .with_context(|| format!("parse registry {}", path.display()))?;
    if registry.tool.trim().is_empty() {
        bail!("registry {} missing tool", path.display());
    }
    if registry.sync_command.trim().is_empty() {
        bail!("registry {} missing sync command", path.display());
    }
    if registry.entries.is_empty() {
        bail!("registry {} has no entries", path.display());
    }
    Ok(registry)
}

fn load_schema(root: &Path, entry: &RegistryEntry) -> Result<YamlValue> {
    let path = resolve(root, &entry.source);
    let text = fs::read_to_string(&path)
        .with_context(|| format!("read schema spec {}", path.display()))?;
    serde_yaml::from_str(&text).with_context(|| format!("parse schema spec {}", path.display()))
}

fn write_witness(
    root: &Path,
    registry: &Registry,
    entry: &RegistryEntry,
    output_text: &str,
    source_sha256: String,
) -> Result<()> {
    let witness = SchemaWitness {
        name: entry.name.clone(),
        source: entry.source.display().to_string(),
        output: entry.output.display().to_string(),
        tool: registry.tool.clone(),
        command: registry.sync_command.clone(),
        checked: true,
        source_sha256,
        output_sha256: sha256_digest(output_text.as_bytes()),
    };
    let witness_path = root
        .join("target/openqg/contracts")
        .join(format!("{}.witness.json", entry.name));
    write_generated_json(&witness_path, &registry.tool, "just schema-check", &witness)?;
    Ok(())
}

fn yaml_to_json(schema: YamlValue) -> Result<JsonValue> {
    serde_json::to_value(schema).context("convert YAML schema to JSON")
}

fn ensure_all_specs_are_registered(registry: &Registry, spec_dir: &Path) -> Result<()> {
    let mut registered = BTreeSet::new();
    for entry in &registry.entries {
        registered.insert(normalize(entry.source.as_path()));
    }

    if !spec_dir.exists() {
        bail!(
            "schema spec directory {} does not exist",
            spec_dir.display()
        );
    }

    for file in walk_yaml_files(spec_dir)? {
        let path = normalize(file.as_path());
        if !registered.contains(&path) {
            bail!("unregistered schema spec {}", file.display());
        }
    }
    Ok(())
}

fn ensure_all_generated_outputs_are_registered(
    registry: &Registry,
    generated_dir: &Path,
) -> Result<()> {
    let mut registered = BTreeSet::new();
    for entry in &registry.entries {
        registered.insert(normalize(entry.output.as_path()));
    }

    if !generated_dir.exists() {
        bail!(
            "generated schema directory {} does not exist",
            generated_dir.display()
        );
    }

    for file in walk_json_files(generated_dir)? {
        let path = normalize(file.as_path());
        if !registered.contains(&path) {
            bail!("unregistered generated schema {}", file.display());
        }
    }
    Ok(())
}

fn walk_yaml_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root) {
        let entry = entry?;
        if entry.file_type().is_file()
            && matches!(
                entry.path().extension().and_then(|s| s.to_str()),
                Some("yml") | Some("yaml")
            )
        {
            files.push(entry.path().to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

fn walk_json_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root) {
        let entry = entry?;
        if entry.file_type().is_file()
            && matches!(
                entry.path().extension().and_then(|s| s.to_str()),
                Some("json")
            )
        {
            files.push(entry.path().to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

fn resolve(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        if matches!(component, Component::CurDir) {
            continue;
        }
        normalized.push(component.as_os_str());
    }
    normalized
}
