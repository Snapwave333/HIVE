use crate::db::models::UiDeployment;
use crate::db::Database;
use crate::events::{EventBus, HiveEvent};
use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};
use tauri::utils::assets::{AssetKey, AssetsIter, CspHash};
use tauri::{Assets, Manager, Runtime};

pub type BundleFiles = Vec<(String, Vec<u8>)>;
type BundleSource = Box<dyn Fn() -> BundleFiles + Send + Sync>;

const PREVIEW_PREFIX: &str = "/__preview/";
const SCANNED_EXTENSIONS: [&str; 4] = ["js", "html", "css", "json"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstitutionReport {
    pub passed: bool,
    pub version: String,
    pub invariants_checked: usize,
    pub violations: Vec<String>,
    pub timestamp: String,
}

#[derive(Clone)]
pub struct DeploymentEngine {
    db: Database,
    event_bus: EventBus,
    root: PathBuf,
    active_bundle: Arc<RwLock<Option<PathBuf>>>,
    bundle_source: Arc<OnceLock<BundleSource>>,
    reload_hook: Arc<OnceLock<Box<dyn Fn() + Send + Sync>>>,
}

pub fn is_safe_version(version: &str) -> bool {
    !version.is_empty()
        && version.len() <= 64
        && version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'))
        && !version.starts_with('.')
}

fn normalize_rel(path: &str) -> Option<PathBuf> {
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    let p = Path::new(trimmed);
    if p.components().all(|c| matches!(c, Component::Normal(_))) {
        Some(p.to_path_buf())
    } else {
        None
    }
}

pub fn bundle_hash(files: &[(String, Vec<u8>)]) -> String {
    let mut sorted: Vec<&(String, Vec<u8>)> = files.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hasher = Sha256::new();
    for (path, bytes) in sorted {
        hasher.update(path.trim_start_matches('/').as_bytes());
        hasher.update([0u8]);
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    format!("{:x}", hasher.finalize())
}

pub fn bundle_scan_text(files: &[(String, Vec<u8>)]) -> String {
    files
        .iter()
        .filter(|(p, _)| {
            Path::new(p)
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| SCANNED_EXTENSIONS.contains(&e))
                .unwrap_or(false)
        })
        .map(|(_, b)| String::from_utf8_lossy(b).to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

fn read_bundle_dir(dir: &Path) -> Result<BundleFiles> {
    fn walk(base: &Path, dir: &Path, out: &mut BundleFiles) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, out)?;
            } else {
                let rel = path
                    .strip_prefix(base)?
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().to_string())
                    .collect::<Vec<_>>()
                    .join("/");
                out.push((format!("/{}", rel), std::fs::read(&path)?));
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    walk(dir, dir, &mut files)?;
    Ok(files)
}

impl DeploymentEngine {
    pub fn new(db: Database, event_bus: EventBus) -> Self {
        Self::with_root(db, event_bus, Self::frontend_root_dir())
    }

    pub fn with_root(db: Database, event_bus: EventBus, root: PathBuf) -> Self {
        Self {
            db,
            event_bus,
            root,
            active_bundle: Arc::new(RwLock::new(None)),
            bundle_source: Arc::new(OnceLock::new()),
            reload_hook: Arc::new(OnceLock::new()),
        }
    }

    pub fn frontend_root_dir() -> PathBuf {
        Database::app_root_dir().join("frontend")
    }

    pub fn versions_dir(&self) -> PathBuf {
        self.root.join("versions")
    }

    pub fn bundle_dir(&self, version: &str) -> PathBuf {
        self.versions_dir().join(version).join("bundle")
    }

    pub fn active_bundle_dir(&self) -> Option<PathBuf> {
        self.active_bundle.read().ok().and_then(|g| g.clone())
    }

    fn set_active_bundle(&self, dir: Option<PathBuf>) {
        if let Ok(mut g) = self.active_bundle.write() {
            *g = dir;
        }
    }

    pub fn set_bundle_source(&self, source: BundleSource) {
        let _ = self.bundle_source.set(source);
    }

    pub fn set_reload_hook(&self, hook: Box<dyn Fn() + Send + Sync>) {
        let _ = self.reload_hook.set(hook);
    }

    fn request_reload(&self) {
        if let Some(hook) = self.reload_hook.get() {
            hook();
        }
    }

    fn running_bundle(&self) -> Result<BundleFiles> {
        let source = self
            .bundle_source
            .get()
            .context("The running frontend bundle isn't available to snapshot yet")?;
        let files = source();
        if files.is_empty() {
            anyhow::bail!("The running build has no embedded frontend files to snapshot");
        }
        Ok(files)
    }

    pub fn validate_constitution(candidate_source: &str) -> ConstitutionReport {
        let raw_const = include_str!("../../../app/constitution.json");
        let parsed: serde_json::Value = serde_json::from_str(raw_const).unwrap_or_default();
        let mut violations = Vec::new();
        let mut checked = 0;

        if let Some(invariants) = parsed["invariants"].as_object() {
            for (key, val) in invariants {
                checked += 1;
                if val["immutable"].as_bool() != Some(true) {
                    violations.push(format!("Invariant '{}' is not marked immutable", key));
                }
            }
        }

        let forbidden = [
            "Superpowers",
            "Metaverse",
            "VirtualWorld3D",
            "disable_owner_authority",
            "auto_play_audio: true",
            "delete_permanent_agent",
        ];

        for term in forbidden {
            if candidate_source.contains(term) {
                violations.push(format!(
                    "Constitutional violation: forbidden pattern '{}' detected in candidate bundle",
                    term
                ));
            }
        }

        ConstitutionReport {
            passed: violations.is_empty() && checked >= 8,
            version: parsed["version"].as_str().unwrap_or("1.0.0").to_string(),
            invariants_checked: checked,
            violations,
            timestamp: Utc::now().to_rfc3339(),
        }
    }

    fn verify_bundle(&self, dep: &UiDeployment) -> Result<PathBuf> {
        let dir = PathBuf::from(&dep.directory_path);
        if !dir.join("index.html").is_file() {
            anyhow::bail!("Snapshot for {} has no index.html", dep.version);
        }
        let files = read_bundle_dir(&dir)?;
        let actual = bundle_hash(&files);
        if actual != dep.sha256_hash {
            anyhow::bail!(
                "Snapshot for {} failed integrity check (expected {}, found {})",
                dep.version,
                dep.sha256_hash.chars().take(12).collect::<String>(),
                &actual[..12]
            );
        }
        Ok(dir)
    }

    pub fn stage_bundle(
        &self,
        version: &str,
        author_agent: &str,
        summary: &str,
        files: &[(String, Vec<u8>)],
        activate: bool,
    ) -> Result<UiDeployment> {
        if !is_safe_version(version) {
            anyhow::bail!("Version '{}' must only use letters, numbers, '.', '-', '_' or '+'", version);
        }
        if !files.iter().any(|(p, _)| p.trim_start_matches('/') == "index.html") {
            anyhow::bail!("Bundle has no index.html");
        }

        let scan = format!("{}\n{}", summary, bundle_scan_text(files));
        let report = Self::validate_constitution(&scan);
        if !report.passed {
            let _ = self.db.log_audit(
                "ui_deployment_blocked",
                author_agent,
                &format!("Version {} blocked: {:?}", version, report.violations),
                "BLOCKED",
            );
            anyhow::bail!(
                "Candidate rejected by Constitution Validator: {:?}",
                report.violations
            );
        }

        let ver_root = self.versions_dir().join(version);
        let bundle_dir = self.bundle_dir(version);
        let staging = ver_root.join(format!("bundle.staging-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&staging)
            .with_context(|| format!("Failed to create staging dir {:?}", staging))?;

        let mut file_hashes = serde_json::Map::new();
        for (path, bytes) in files {
            let Some(rel) = normalize_rel(path) else {
                continue;
            };
            let dest = staging.join(&rel);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&dest, bytes)?;
            let mut h = Sha256::new();
            h.update(bytes);
            file_hashes.insert(
                rel.to_string_lossy().replace('\\', "/"),
                serde_json::Value::String(format!("{:x}", h.finalize())),
            );
        }

        let written = read_bundle_dir(&staging)?;
        let sha = bundle_hash(&written);
        if bundle_dir.exists() {
            std::fs::remove_dir_all(&bundle_dir)?;
        }
        std::fs::rename(&staging, &bundle_dir)?;

        let manifest = serde_json::json!({
            "version": version,
            "author": author_agent,
            "summary": summary,
            "bundle_sha256": sha,
            "file_count": written.len(),
            "files": file_hashes,
            "constitution_report": report,
            "created_at": Utc::now().to_rfc3339(),
        });
        std::fs::write(
            ver_root.join("manifest.json"),
            serde_json::to_string_pretty(&manifest)?,
        )?;

        let dep = UiDeployment {
            version: version.to_string(),
            directory_path: bundle_dir.to_string_lossy().to_string(),
            sha256_hash: sha,
            author_agent: author_agent.to_string(),
            verification_log: Some(format!(
                "Constitution v{}: {} invariants checked, 0 violations across {} scanned bundle files. {} files snapshotted. {}",
                report.version,
                report.invariants_checked,
                files
                    .iter()
                    .filter(|(p, _)| Path::new(p)
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| SCANNED_EXTENSIONS.contains(&e))
                        .unwrap_or(false))
                    .count(),
                written.len(),
                summary
            )),
            is_active: activate,
            created_at: Utc::now().to_rfc3339(),
        };

        self.db.record_ui_deployment(&dep)?;
        if activate {
            self.set_active_bundle(Some(bundle_dir));
            std::fs::write(self.root.join("current.json"), serde_json::to_string_pretty(&dep)?)?;
        }

        self.db.log_audit(
            "ui_deployment",
            author_agent,
            &format!(
                "Version {} snapshotted (sha256 {}, active={}): {}",
                version,
                &dep.sha256_hash[..12],
                activate,
                summary
            ),
            "VERIFIED",
        )?;

        self.event_bus.publish(HiveEvent::UiDeploymentEvent {
            version: version.to_string(),
            status: if activate { "active".to_string() } else { "staged_preview".to_string() },
            author_agent: author_agent.to_string(),
        });

        Ok(dep)
    }

    pub fn stage_and_deploy_version(
        &self,
        version: &str,
        author_agent: &str,
        summary: &str,
        activate: bool,
    ) -> Result<UiDeployment> {
        let files = self.running_bundle()?;
        let dep = self.stage_bundle(version, author_agent, summary, &files, activate)?;
        if activate {
            self.request_reload();
        }
        Ok(dep)
    }

    pub fn ensure_baseline(&self, build_version: &str) -> Result<Option<UiDeployment>> {
        let files = self.running_bundle()?;
        let sha = bundle_hash(&files);
        let tag = format!("v{}+{}", build_version.trim_start_matches('v'), &sha[..8]);

        let existing = self.db.get_ui_deployments()?;
        if existing.iter().any(|d| d.version == tag && d.sha256_hash == sha && self.verify_bundle(d).is_ok()) {
            return Ok(None);
        }
        let dep = self.stage_bundle(
            &tag,
            "FORGE",
            "Baseline snapshot of the installed build",
            &files,
            true,
        )?;
        Ok(Some(dep))
    }

    pub fn load_active(&self) -> Option<UiDeployment> {
        let active = self
            .db
            .get_ui_deployments()
            .ok()?
            .into_iter()
            .find(|d| d.is_active)?;
        match self.verify_bundle(&active) {
            Ok(dir) => {
                self.set_active_bundle(Some(dir));
                Some(active)
            }
            Err(e) => {
                let _ = self.db.log_audit(
                    "ui_integrity_check",
                    "SENTINEL",
                    &format!("Not serving {}: {}", active.version, e),
                    "FAILED",
                );
                self.set_active_bundle(None);
                None
            }
        }
    }

    pub fn rollback(&self, reason: &str) -> Result<UiDeployment> {
        let deployments = self.db.get_ui_deployments()?;
        let current = deployments.iter().find(|d| d.is_active).map(|d| d.version.clone());
        let target = deployments
            .iter()
            .filter(|d| !d.is_active && Some(&d.version) != current.as_ref())
            .find_map(|d| self.verify_bundle(d).ok().map(|dir| (d.clone(), dir)));

        let Some((target, dir)) = target else {
            anyhow::bail!(
                "No other verified UI snapshot to roll back to{}",
                current.map(|c| format!(" (currently on {})", c)).unwrap_or_default()
            );
        };

        let mut restored = target.clone();
        restored.is_active = true;
        self.db.record_ui_deployment(&restored)?;
        self.set_active_bundle(Some(dir));

        std::fs::create_dir_all(&self.root)?;
        std::fs::write(
            self.root.join("current.json"),
            serde_json::to_string_pretty(&restored)?,
        )?;

        self.db.log_audit(
            "ui_rollback",
            "SENTINEL",
            &format!("Rolled back active UI to {} — Reason: {}", restored.version, reason),
            "ROLLED_BACK",
        )?;

        self.event_bus.publish(HiveEvent::UiDeploymentEvent {
            version: restored.version.clone(),
            status: "rolled_back".to_string(),
            author_agent: "SENTINEL".to_string(),
        });

        self.request_reload();
        Ok(restored)
    }

    pub fn resolve_asset(&self, key: &str) -> Option<Vec<u8>> {
        if let Some(rest) = key.strip_prefix(PREVIEW_PREFIX) {
            let (version, path) = rest.split_once('/').unwrap_or((rest, "index.html"));
            if !is_safe_version(version) {
                return None;
            }
            let rel = normalize_rel(if path.is_empty() { "index.html" } else { path })?;
            return std::fs::read(self.bundle_dir(version).join(rel)).ok();
        }
        let dir = self.active_bundle_dir()?;
        let rel = normalize_rel(key)?;
        std::fs::read(dir.join(rel)).ok()
    }
}

pub struct SnapshotAssets<R: Runtime> {
    engine: DeploymentEngine,
    embedded: Arc<dyn Assets<R>>,
}

impl<R: Runtime> SnapshotAssets<R> {
    pub fn new(engine: DeploymentEngine, embedded: Arc<dyn Assets<R>>) -> Self {
        Self { engine, embedded }
    }

    pub fn embedded_files(embedded: &dyn Assets<R>) -> BundleFiles {
        let keys: Vec<String> = embedded.iter().map(|(k, _)| k.to_string()).collect();
        keys.into_iter()
            .filter_map(|k| {
                let bytes = embedded.get(&AssetKey::from(k.as_str()))?.into_owned();
                Some((k, bytes))
            })
            .collect()
    }
}

pub struct NoAssets;

impl<R: Runtime> Assets<R> for NoAssets {
    fn get(&self, _key: &AssetKey) -> Option<Cow<'_, [u8]>> {
        None
    }

    fn iter(&self) -> Box<AssetsIter<'_>> {
        Box::new(std::iter::empty())
    }

    fn csp_hashes(&self, _html_path: &AssetKey) -> Box<dyn Iterator<Item = CspHash<'_>> + '_> {
        Box::new(std::iter::empty())
    }
}

impl<R: Runtime> Assets<R> for SnapshotAssets<R> {
    fn get(&self, key: &AssetKey) -> Option<Cow<'_, [u8]>> {
        if let Some(bytes) = self.engine.resolve_asset(key.as_ref()) {
            return Some(Cow::Owned(bytes));
        }
        if key.as_ref().starts_with(PREVIEW_PREFIX) {
            return None;
        }
        self.embedded.get(key)
    }

    fn iter(&self) -> Box<AssetsIter<'_>> {
        self.embedded.iter()
    }

    fn csp_hashes(&self, html_path: &AssetKey) -> Box<dyn Iterator<Item = CspHash<'_>> + '_> {
        self.embedded.csp_hashes(html_path)
    }
}

pub fn install_reload_hook<R: Runtime>(engine: &DeploymentEngine, app: &tauri::AppHandle<R>) {
    let handle = app.clone();
    engine.set_reload_hook(Box::new(move || {
        if let Some(w) = handle.get_webview_window("main") {
            let _ = w.eval("window.location.reload()");
        }
    }));
}
