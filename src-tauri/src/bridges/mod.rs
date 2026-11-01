use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Write;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(target_os = "windows")]
const CURL_BIN: &str = "curl.exe";
#[cfg(not(target_os = "windows"))]
const CURL_BIN: &str = "curl";

const NIM_ENDPOINT: &str = "https://integrate.api.nvidia.com/v1/chat/completions";
const KEY_PROBE_MODEL: &str = "nvidia/nemotron-3.5-bolt-e4b";

pub const NIM_FALLBACKS: [&str; 7] = [
    "nvidia/nemotron-3.5-lightning-30b-a3b",
    "nvidia/nemotron-3.5-bolt-e4b",
    "openai/gpt-oss-20b",
    "nvidia/nemotron-3-ultra-550b-a55b",
    "nvidia/nemotron-3.5-super-vl-120b-a12b",
    "minimaxai/minimax-m3.1-flash",
    "mistralai/mistral-large-4-1t-a52b",
];

const MAX_MODELS_PER_CALL: usize = 4;
const CASCADE_BUDGET: Duration = Duration::from_secs(100);

#[derive(Debug, Clone, PartialEq)]
pub struct ChatTurn {
    pub role: &'static str,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeStatus {
    pub google_connected: bool,
    pub google_account: String,
    pub obsidian_available: bool,
    pub obsidian_vault_path: String,
    pub browser_relay_available: bool,
    pub voice_engine_available: bool,
    pub llm_provider: String,
}

pub struct EcosystemBridges;

impl EcosystemBridges {
    /// Resolve path to gog.bat / gog.exe
    pub fn resolve_gog_path() -> PathBuf {
        if let Ok(p) = std::env::var("GOG_CLI_PATH") {
            let pb = PathBuf::from(&p);
            if pb.exists() {
                return pb;
            }
        }
        if let Some(home) = dirs::home_dir() {
            let wrapper = home.join("gog-wrapper").join("gog.bat");
            if wrapper.exists() {
                return wrapper;
            }
            let gobin = home.join("go").join("bin").join("gog.exe");
            if gobin.exists() {
                return gobin;
            }
        }
        PathBuf::from("gog")
    }

    /// Resolve path to local Obsidian vault
    pub fn resolve_vault_dir() -> PathBuf {
        if let Ok(v) = std::env::var("HIVE_OBSIDIAN_VAULT") {
            let pb = PathBuf::from(v);
            if pb.exists() {
                return pb;
            }
        }
        if let Some(docs) = dirs::document_dir() {
            let candidate = docs.join("HIVE-Vault");
            let _ = std::fs::create_dir_all(&candidate);
            return candidate;
        }
        let fallback = crate::db::Database::app_root_dir().join("vault");
        let _ = std::fs::create_dir_all(&fallback);
        fallback
    }

    /// Detect status of all local bridges
    pub fn get_status() -> BridgeStatus {
        Self::check_all()
    }

    pub fn check_all() -> BridgeStatus {
        let (google_connected, google_account) = match Self::check_google_auth() {
            Ok(out) => (
                true,
                out.lines()
                    .map(str::trim)
                    .find(|l| !l.is_empty())
                    .unwrap_or_default()
                    .to_string(),
            ),
            Err(_) => (false, String::new()),
        };

        let obsidian_available = Self::check_obsidian().is_ok();
        let obsidian_vault_path = Self::resolve_vault_dir().to_string_lossy().to_string();
        let browser_relay_available = Self::check_browser_relay().is_ok();
        let voice_engine_available = Self::check_voice_engine().is_ok();

        let llm_provider = if Self::llm_configured() {
            "NVIDIA NIM (API key set)".to_string()
        } else {
            "Unconfigured (Set NVIDIA_API_KEY)".to_string()
        };

        BridgeStatus {
            google_connected,
            google_account,
            obsidian_available,
            obsidian_vault_path,
            browser_relay_available,
            voice_engine_available,
            llm_provider,
        }
    }

    /// Validate CLI arguments to prevent command injection
    fn sanitize_arg(arg: &str) -> anyhow::Result<String> {
        let forbidden = ['&', '|', ';', '`', '$', '>', '<', '\n', '\r', '%', '^', '"', '(', ')', '!', '\0'];
        if arg.chars().any(|c| forbidden.contains(&c)) {
            anyhow::bail!("Rejected argument containing unsafe shell character: {}", arg);
        }
        Ok(arg.to_string())
    }

    /// Google CLI (gog) bridge
    pub fn check_google_auth() -> anyhow::Result<String> {
        Self::gog_cli_exec(&["auth", "list"])
    }

    pub fn gog_query(service: &str, args: &[&str]) -> anyhow::Result<String> {
        let mut cmd_args = vec![service];
        cmd_args.extend_from_slice(args);
        Self::gog_cli_exec(&cmd_args)
    }

    pub fn gog_cli_exec(args: &[&str]) -> anyhow::Result<String> {
        let gog_path = Self::resolve_gog_path();
        let gog_str = gog_path.to_string_lossy().to_string();

        let allowed_top = [
            "auth", "status", "st", "me", "whoami", "drive", "drv", "ls", "search",
            "calendar", "cal", "gmail", "mail", "tasks", "task", "docs", "doc",
            "sheets", "sheet", "slides", "contacts", "people", "time", "version", "help",
        ];

        if let Some(first) = args.first() {
            let clean_first = first.trim_start_matches('-');
            if !allowed_top.contains(&clean_first) {
                anyhow::bail!("Subcommand '{}' is not in the gog security allowlist", first);
            }
        }

        let mut safe_args: Vec<String> = Vec::new();
        for a in args {
            let trimmed = a.trim_matches('\'');
            safe_args.push(Self::sanitize_arg(trimmed)?);
        }
        if !safe_args.iter().any(|a| a == "--no-input") {
            safe_args.push("--no-input".to_string());
        }

        let mut cmd_args: Vec<&str> = vec!["/C", &gog_str];
        for s in &safe_args {
            cmd_args.push(s.as_str());
        }

        let mut cmd = Command::new("cmd");
        cmd.args(&cmd_args);
        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);
        let output = cmd.output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let err_str = String::from_utf8_lossy(&output.stderr).to_string();
            let out_str = String::from_utf8_lossy(&output.stdout).to_string();
            anyhow::bail!(
                "gog command exited with {}: {} {}",
                output.status,
                err_str.trim(),
                out_str.trim()
            )
        }
    }

    /// Obsidian CLI & Curated Vault Bridge
    pub fn check_obsidian() -> anyhow::Result<String> {
        let mut cmd = Command::new(if cfg!(target_os = "windows") { "where" } else { "which" });
        cmd.arg(if cfg!(target_os = "windows") { "Obsidian.com" } else { "obsidian" });
        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);
        let found = cmd.output().ok().filter(|o| o.status.success());
        if let Some(output) = found {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            let vault = Self::resolve_vault_dir();
            if vault.exists() {
                Ok(vault.to_string_lossy().to_string())
            } else {
                anyhow::bail!("Obsidian vault not found")
            }
        }
    }

    pub fn write_obsidian_note(title: &str, content: &str, folder: Option<&str>) -> anyhow::Result<PathBuf> {
        let mut dir = Self::resolve_vault_dir();
        if let Some(sub) = folder {
            let safe_sub: String = sub
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .collect();
            if !safe_sub.is_empty() {
                dir = dir.join(safe_sub);
            }
        }
        std::fs::create_dir_all(&dir)?;

        let safe_title: String = title
            .chars()
            .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' })
            .collect();
        let filename = format!("{}.md", safe_title.trim());
        let file_path = dir.join(filename);

        let full_doc = format!(
            "---\ntitle: \"{}\"\nmanaged_by: H.I.V.E. ARCHIVIST\nupdated_at: \"{}\"\n---\n\n{}\n",
            title,
            chrono::Utc::now().to_rfc3339(),
            content
        );
        std::fs::write(&file_path, full_doc)?;
        Ok(file_path)
    }

    fn collect_markdown_files(dir: &std::path::Path, depth: usize, out: &mut Vec<PathBuf>) {
        if depth > 3 {
            return;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    Self::collect_markdown_files(&p, depth + 1, out);
                } else if p.extension().and_then(|e| e.to_str()) == Some("md") {
                    out.push(p);
                }
            }
        }
    }

    pub fn read_obsidian_note(query: &str) -> anyhow::Result<String> {
        let dir = Self::resolve_vault_dir();
        let q_lower = query.to_lowercase();
        let mut files = Vec::new();
        Self::collect_markdown_files(&dir, 0, &mut files);
        for path in files {
            if let Some(fname) = path.file_name().map(|f| f.to_string_lossy().to_lowercase()) {
                if fname.contains(&q_lower) {
                    return Ok(std::fs::read_to_string(&path)?);
                }
            }
        }
        anyhow::bail!("No note matching '{}' found in vault {:?}", query, dir)
    }

    pub fn search_obsidian_vault(query: &str) -> anyhow::Result<Vec<(String, String)>> {
        let dir = Self::resolve_vault_dir();
        let q_lower = query.to_lowercase();
        let mut results = Vec::new();
        let mut files = Vec::new();
        Self::collect_markdown_files(&dir, 0, &mut files);
        for path in files {
            let fname = path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default();
            if let Ok(body) = std::fs::read_to_string(&path) {
                if fname.to_lowercase().contains(&q_lower) || body.to_lowercase().contains(&q_lower) {
                    let snippet: String = body.lines().take(5).collect::<Vec<_>>().join(" ");
                    results.push((fname, snippet));
                }
            }
        }
        Ok(results)
    }

    pub fn search_obsidian_notes(query: &str) -> anyhow::Result<Vec<serde_json::Value>> {
        let pairs = Self::search_obsidian_vault(query)?;
        Ok(pairs
            .into_iter()
            .map(|(title, snippet)| serde_json::json!({ "title": title, "snippet": snippet }))
            .collect())
    }

    /// OpenCode Chrome Relay Bridge
    pub fn check_browser_relay() -> anyhow::Result<String> {
        let mut cmd = Command::new("where");
        cmd.arg("browser-relay");
        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);
        let output = cmd.output()?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            anyhow::bail!("browser-relay not found in PATH")
        }
    }

    pub fn browser_relay_exec(subcmd: &str, args: &[&str]) -> anyhow::Result<String> {
        Self::execute_browser_relay(subcmd, args)
    }

    pub fn execute_browser_relay(subcmd: &str, args: &[&str]) -> anyhow::Result<String> {
        let allowed = ["status", "tabs", "snapshot", "links", "text", "navigate", "eval"];
        if !allowed.contains(&subcmd) {
            anyhow::bail!(
                "Browser relay command '{}' requires Level-3 Owner Gate or is not allowlisted",
                subcmd
            );
        }
        let mut cmd_args = vec!["/C", "browser-relay", subcmd];
        for a in args {
            Self::sanitize_arg(a)?;
            cmd_args.push(a);
        }
        let mut cmd = Command::new("cmd");
        cmd.args(&cmd_args);
        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);
        let output = cmd.output()?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if output.status.success() {
            Ok(stdout)
        } else {
            anyhow::bail!("browser-relay exited with {}: {} {}", output.status, stdout.trim(), stderr.trim())
        }
    }

    /// Voice Synthesis Bridge (Kokoro / Windows SAPI Neural WAV Synthesizer)
    pub fn check_voice_engine() -> anyhow::Result<String> {
        if !cfg!(target_os = "windows") {
            anyhow::bail!("Voice synthesis uses Windows SAPI and is only available on Windows");
        }
        let mut cmd = Command::new("powershell");
        cmd.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Add-Type -AssemblyName System.Speech; (New-Object System.Speech.Synthesis.SpeechSynthesizer).GetInstalledVoices().Count",
        ]);
        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);
        let output = cmd.output()?;
        let voices = String::from_utf8_lossy(&output.stdout).trim().parse::<u32>().unwrap_or(0);
        if output.status.success() && voices > 0 {
            Ok(format!("Windows SAPI ({} voices installed)", voices))
        } else {
            anyhow::bail!("Windows SAPI has no installed voices")
        }
    }

    /// Synthesizes speech strictly to cache and returns a playable data:audio/wav;base64 URI.
    /// Never plays automatically; called only on explicit user click when unmuted.
    pub fn synthesize_speech_data_url(text: &str, agent_codename: &str) -> anyhow::Result<String> {
        let path = Self::synthesize_speech(text, agent_codename)?;
        let bytes = std::fs::read(&path)?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        Ok(format!("data:audio/wav;base64,{}", b64))
    }

    pub fn synthesize_speech(text: &str, agent_codename: &str) -> anyhow::Result<PathBuf> {
        let audio_dir = crate::db::Database::app_root_dir()
            .join("cache")
            .join("audio");
        std::fs::create_dir_all(&audio_dir)?;

        let mut hasher = Sha256::new();
        hasher.update(agent_codename.as_bytes());
        hasher.update(b":");
        hasher.update(text.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        let filename = format!("{}_{}.wav", agent_codename, &hash[..16]);
        let dest = audio_dir.join(filename);

        if !dest.exists() {
            let clean_text: String = text
                .chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace() || ",.?!:;'-".contains(*c))
                .take(400)
                .collect();

            let rate = match agent_codename {
                "PRIME" => 0,
                "CONDUCTOR" | "FORGE" | "SPARK" => 1,
                "SAGE" | "ARCHIVIST" | "SENTINEL" => -1,
                _ => 0,
            };

            let ps_script = format!(
                "Add-Type -AssemblyName System.Speech; \
                 $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
                 $s.Rate = {}; \
                 $s.SetOutputToWaveFile('{}'); \
                 $s.Speak('{}'); \
                 $s.Dispose();",
                rate,
                dest.to_string_lossy().replace('\'', "''"),
                clean_text.replace('\'', " ")
            );

            let mut cmd = Command::new("powershell");
            cmd.args(["-NoProfile", "-NonInteractive", "-Command", &ps_script]);
            #[cfg(target_os = "windows")]
            cmd.creation_flags(CREATE_NO_WINDOW);
            let status = cmd.output()?;

            if !status.status.success() || !dest.exists() {
                anyhow::bail!(
                    "Speech synthesis failed: {}",
                    String::from_utf8_lossy(&status.stderr)
                );
            }
        }

        Ok(dest)
    }

    /// Retrieve API key from process env or Windows User/Machine Registry
    pub fn get_env_key(name: &str) -> Option<String> {
        if let Ok(v) = std::env::var(name) {
            if !v.trim().is_empty() {
                return Some(v.trim().to_string());
            }
        }
        #[cfg(target_os = "windows")]
        {
            use winreg::enums::*;
            use winreg::RegKey;
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(env) = hkcu.open_subkey("Environment") {
                if let Ok(val) = env.get_value::<String, _>(name) {
                    if !val.trim().is_empty() {
                        return Some(val.trim().to_string());
                    }
                }
            }
            let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
            if let Ok(env) = hklm.open_subkey(r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment") {
                if let Ok(val) = env.get_value::<String, _>(name) {
                    if !val.trim().is_empty() {
                        return Some(val.trim().to_string());
                    }
                }
            }
        }
        None
    }

    fn think_re() -> &'static regex::Regex {
        static RE: OnceLock<regex::Regex> = OnceLock::new();
        RE.get_or_init(|| regex::Regex::new(r"(?s)<think>.*?</think>").unwrap())
    }

    pub fn strip_think(txt: &str) -> String {
        let mut s = Self::think_re().replace_all(txt, "").to_string();
        if let Some(idx) = s.rfind("</think>") {
            s = s[idx + 8..].to_string();
        }
        s.trim().to_string()
    }

    pub fn llm_configured() -> bool {
        Self::nim_key().is_some()
    }

    fn runtime_key() -> &'static RwLock<Option<String>> {
        static KEY: OnceLock<RwLock<Option<String>>> = OnceLock::new();
        KEY.get_or_init(|| RwLock::new(None))
    }

    pub fn key_file_path() -> PathBuf {
        crate::db::Database::app_root_dir().join("secrets").join("nim_api_key")
    }

    fn nim_key() -> Option<String> {
        if let Some(k) = Self::runtime_key().read().ok().and_then(|g| g.clone()) {
            return Some(k);
        }
        Self::get_env_key("NVIDIA_API_KEY")
            .or_else(|| Self::get_env_key("NIM_API_KEY"))
            .or_else(|| {
                std::fs::read_to_string(Self::key_file_path())
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            })
    }

    pub fn verify_nim_key(key: &str) -> anyhow::Result<String> {
        let key = key.trim();
        if key.is_empty() || key.chars().any(|c| c.is_whitespace() || c.is_control()) {
            anyhow::bail!("That doesn't look like an API key");
        }
        let probe = Self::build_payload(
            KEY_PROBE_MODEL,
            "Reply with OK.",
            &[ChatTurn {
                role: "user",
                content: "ping".to_string(),
            }],
            1,
            true,
        );
        match Self::post_nim_request(key, &probe) {
            Ok(_) | Err(NimFailure::EmptyContent) => Ok("verified with a test request".to_string()),
            Err(NimFailure::Auth(d)) => anyhow::bail!("NVIDIA rejected that key ({})", d),
            Err(NimFailure::Unavailable(d)) if d.contains("exited with") || d.contains("could not start") => {
                anyhow::bail!("Couldn't reach NVIDIA to check the key: {}", d)
            }
            Err(other) => Ok(format!("accepted by NVIDIA (test request said: {})", other)),
        }
    }

    pub fn store_nim_key(key: &str) -> anyhow::Result<String> {
        let key = key.trim().to_string();
        let verdict = Self::verify_nim_key(&key)?;
        let location = Self::persist_key(&key)?;
        if let Ok(mut g) = Self::runtime_key().write() {
            *g = Some(key);
        }
        Self::cooldowns().lock().unwrap_or_else(|e| e.into_inner()).clear();
        Ok(format!("Key {} and saved to {}", verdict, location))
    }

    #[cfg(target_os = "windows")]
    fn persist_key(key: &str) -> anyhow::Result<String> {
        use winreg::enums::*;
        use winreg::RegKey;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (env, _) = hkcu.create_subkey("Environment")?;
        env.set_value("NVIDIA_API_KEY", &key)?;
        Ok("your Windows user environment (NVIDIA_API_KEY)".to_string())
    }

    #[cfg(not(target_os = "windows"))]
    fn persist_key(key: &str) -> anyhow::Result<String> {
        let path = Self::key_file_path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, key)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(path.to_string_lossy().to_string())
    }

    pub fn is_config_error(err: &str) -> bool {
        err.contains("NVIDIA_API_KEY is not set") || err.contains("rejected the API key")
    }

    fn cooldowns() -> &'static Mutex<HashMap<String, Instant>> {
        static COOLDOWNS: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();
        COOLDOWNS.get_or_init(|| Mutex::new(HashMap::new()))
    }

    pub fn is_cooling(model: &str) -> bool {
        let mut map = Self::cooldowns().lock().unwrap_or_else(|e| e.into_inner());
        match map.get(model) {
            Some(until) if *until > Instant::now() => true,
            Some(_) => {
                map.remove(model);
                false
            }
            None => false,
        }
    }

    fn set_cooldown(model: &str, dur: Duration) {
        let mut map = Self::cooldowns().lock().unwrap_or_else(|e| e.into_inner());
        map.insert(model.to_string(), Instant::now() + dur);
    }

    fn clear_cooldown(model: &str) {
        let mut map = Self::cooldowns().lock().unwrap_or_else(|e| e.into_inner());
        map.remove(model);
    }

    pub fn cascade_order(primary: &str, is_cooling: &dyn Fn(&str) -> bool) -> Vec<String> {
        let mut ready = Vec::new();
        let mut cooling = Vec::new();
        for model in std::iter::once(primary).chain(NIM_FALLBACKS.iter().copied()) {
            let model = model.trim();
            if model.is_empty()
                || ready.iter().any(|m: &String| m == model)
                || cooling.iter().any(|m: &String| m == model)
            {
                continue;
            }
            if is_cooling(model) {
                cooling.push(model.to_string());
            } else {
                ready.push(model.to_string());
            }
        }
        ready.extend(cooling);
        ready
    }

    pub fn classify_http(status: u16, body: &str) -> NimFailure {
        let detail = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|j| {
                j.pointer("/error/message")
                    .or_else(|| j.pointer("/detail"))
                    .or_else(|| j.pointer("/message"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| body.chars().take(200).collect());
        match status {
            401 | 403 => NimFailure::Auth(format!("HTTP {}: {}", status, detail)),
            404 => NimFailure::ModelMissing(format!("HTTP 404: {}", detail)),
            408 | 429 => NimFailure::RateLimited(format!("HTTP {}: {}", status, detail)),
            500..=599 => NimFailure::Unavailable(format!("HTTP {}: {}", status, detail)),
            _ => NimFailure::Other(format!("HTTP {}: {}", status, detail)),
        }
    }

    fn post_nim_request(key: &str, payload: &serde_json::Value) -> Result<String, NimFailure> {
        let payload_bytes =
            serde_json::to_vec(payload).map_err(|e| NimFailure::Other(e.to_string()))?;

        let mut cmd = Command::new(CURL_BIN);
        cmd.args([
            "-sS",
            "--connect-timeout",
            "10",
            "--max-time",
            "45",
            "-w",
            "\n__HIVE_HTTP_STATUS__:%{http_code}",
            "-H",
            &format!("Authorization: Bearer {}", key),
            "-H",
            "Content-Type: application/json",
            "--data-binary",
            "@-",
            NIM_ENDPOINT,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let mut child = cmd
            .spawn()
            .map_err(|e| NimFailure::Unavailable(format!("could not start {}: {}", CURL_BIN, e)))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(&payload_bytes)
                .map_err(|e| NimFailure::Unavailable(e.to_string()))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| NimFailure::Unavailable(e.to_string()))?;
        if !output.status.success() {
            return Err(NimFailure::Unavailable(format!(
                "{} exited with {}: {}",
                CURL_BIN,
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }

        let raw = String::from_utf8_lossy(&output.stdout).to_string();
        let (body, status) = match raw.rsplit_once("\n__HIVE_HTTP_STATUS__:") {
            Some((b, code)) => (b.to_string(), code.trim().parse::<u16>().unwrap_or(0)),
            None => (raw.clone(), 0),
        };

        if status != 0 && status != 200 {
            return Err(Self::classify_http(status, &body));
        }

        let json: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| NimFailure::Other(format!("bad JSON from NIM: {}", e)))?;

        if let Some(err_msg) = json.pointer("/error/message").and_then(|v| v.as_str()) {
            return Err(NimFailure::Other(err_msg.to_string()));
        }

        let content = json
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .map(Self::strip_think)
            .unwrap_or_default();

        if content.is_empty() {
            Err(NimFailure::EmptyContent)
        } else {
            Ok(content)
        }
    }

    fn build_payload(
        model: &str,
        system_prompt: &str,
        turns: &[ChatTurn],
        max_tokens: u32,
        disable_thinking: bool,
    ) -> serde_json::Value {
        let mut messages = vec![serde_json::json!({ "role": "system", "content": system_prompt })];
        for t in turns {
            messages.push(serde_json::json!({ "role": t.role, "content": t.content }));
        }
        let mut payload = serde_json::json!({
            "model": model,
            "messages": messages,
            "max_tokens": max_tokens,
            "temperature": 0.7
        });
        if disable_thinking {
            payload["chat_template_kwargs"] = serde_json::json!({ "enable_thinking": false });
        }
        payload
    }

    pub fn call_nim_chat(
        model_tier: &str,
        system_prompt: &str,
        turns: &[ChatTurn],
    ) -> anyhow::Result<String> {
        let key = Self::nim_key()
            .ok_or_else(|| anyhow::anyhow!("NVIDIA_API_KEY is not set. Add it in Settings → Bridges"))?;

        let fallback_turn = [ChatTurn {
            role: "user",
            content: "Go ahead.".to_string(),
        }];
        let turns: &[ChatTurn] = if turns.is_empty() { &fallback_turn } else { turns };

        let started = Instant::now();
        let mut last_err = String::from("no models attempted");

        for (attempt, model) in Self::cascade_order(model_tier, &Self::is_cooling)
            .into_iter()
            .enumerate()
        {
            if attempt >= MAX_MODELS_PER_CALL || started.elapsed() > CASCADE_BUDGET {
                break;
            }

            let first = Self::post_nim_request(
                &key,
                &Self::build_payload(&model, system_prompt, turns, 2560, false),
            );
            let outcome = match first {
                Err(NimFailure::EmptyContent) => Self::post_nim_request(
                    &key,
                    &Self::build_payload(&model, system_prompt, turns, 1024, true),
                ),
                other => other,
            };

            match outcome {
                Ok(text) => {
                    Self::clear_cooldown(&model);
                    return Ok(text);
                }
                Err(NimFailure::Auth(detail)) => {
                    anyhow::bail!(
                        "NVIDIA NIM rejected the API key ({}). Check NVIDIA_API_KEY.",
                        detail
                    );
                }
                Err(failure) => {
                    if let Some(dur) = failure.cooldown() {
                        Self::set_cooldown(&model, dur);
                    }
                    last_err = format!("{}: {}", model, failure);
                }
            }
        }

        anyhow::bail!("All NIM cascade models failed (last error: {})", last_err)
    }

    pub fn call_nim_completion(
        model_tier: &str,
        system_prompt: &str,
        user_message: &str,
    ) -> anyhow::Result<String> {
        Self::call_nim_chat(
            model_tier,
            system_prompt,
            &[ChatTurn {
                role: "user",
                content: user_message.to_string(),
            }],
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NimFailure {
    Auth(String),
    RateLimited(String),
    Unavailable(String),
    ModelMissing(String),
    EmptyContent,
    Other(String),
}

impl NimFailure {
    pub fn cooldown(&self) -> Option<Duration> {
        match self {
            NimFailure::RateLimited(_) => Some(Duration::from_secs(120)),
            NimFailure::Unavailable(_) => Some(Duration::from_secs(300)),
            NimFailure::ModelMissing(_) => Some(Duration::from_secs(1800)),
            _ => None,
        }
    }
}

impl std::fmt::Display for NimFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NimFailure::Auth(d) => write!(f, "auth rejected ({})", d),
            NimFailure::RateLimited(d) => write!(f, "rate limited ({})", d),
            NimFailure::Unavailable(d) => write!(f, "unavailable ({})", d),
            NimFailure::ModelMissing(d) => write!(f, "model not found ({})", d),
            NimFailure::EmptyContent => write!(f, "empty response (reasoning used up the token budget)"),
            NimFailure::Other(d) => write!(f, "{}", d),
        }
    }
}
