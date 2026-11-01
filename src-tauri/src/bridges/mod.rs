use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Write;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

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
        PathBuf::from(r"C:\Users\chrom\gog-wrapper\gog.bat")
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
        let google_account = Self::check_google_auth()
            .unwrap_or_else(|_| "elikenny333@gmail.com".to_string());
        let google_connected = !google_account.is_empty();

        let obsidian_available = Self::check_obsidian().is_ok();
        let obsidian_vault_path = Self::resolve_vault_dir().to_string_lossy().to_string();
        let browser_relay_available = Self::check_browser_relay().is_ok();
        let voice_engine_available = Self::check_voice_engine().is_ok();

        let llm_provider = if Self::get_env_key("NVIDIA_API_KEY").is_some()
            || Self::get_env_key("NIM_API_KEY").is_some()
        {
            "NVIDIA NIM Cascade (Live)".to_string()
        } else if Self::get_env_key("GEMINI_API_KEY").is_some() {
            "Google Gemini API (Live)".to_string()
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
        let forbidden = ['&', '|', ';', '`', '$', '>', '<', '\n', '\r'];
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
        let mut cmd = Command::new("where");
        cmd.arg("Obsidian.com");
        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);
        let output = cmd.output()?;
        if output.status.success() {
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
        let mut cmd = Command::new("where");
        cmd.arg("kokoro");
        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);
        let output = cmd.output()?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Ok("Windows SAPI SpeechSynthesizer".to_string())
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

    /// Strip `<think>...</think>` blocks emitted by reasoning models (ported from NimCascadeRouter.cs)
    fn strip_think(txt: &str) -> String {
        let mut s = txt.to_string();
        if let Ok(re) = regex::Regex::new(r"(?s)<think>.*?</think>") {
            s = re.replace_all(&s, "").to_string();
        }
        if let Some(idx) = s.rfind("</think>") {
            s = s[idx + 8..].to_string();
        }
        s.trim().to_string()
    }

    /// Execute a single HTTP POST to NVIDIA NIM via curl.exe using stdin (--data-binary @-)
    /// so Windows command-line escaping never mangles JSON payloads.
    fn post_nim_request(key: &str, payload: &serde_json::Value) -> anyhow::Result<String> {
        let payload_bytes = serde_json::to_vec(payload)?;

        let mut cmd = Command::new("curl.exe");
        cmd.args([
            "-sS",
            "--max-time",
            "25",
            "-H",
            &format!("Authorization: Bearer {}", key),
            "-H",
            "Content-Type: application/json",
            "--data-binary",
            "@-",
            "https://integrate.api.nvidia.com/v1/chat/completions",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let mut child = cmd.spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(&payload_bytes)?;
        }

        let output = child.wait_with_output()?;
        if !output.status.success() {
            anyhow::bail!(
                "curl.exe exited with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let resp_str = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&resp_str)?;

        if let Some(err_msg) = json.pointer("/error/message").and_then(|v| v.as_str()) {
            anyhow::bail!("NIM API error: {}", err_msg);
        }

        let content = json
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .map(Self::strip_think)
            .unwrap_or_default();

        if !content.is_empty() {
            return Ok(content);
        }

        anyhow::bail!("Empty content from model (reasoning_content may have consumed tokens)")
    }

    /// Authentic port of NimCascadeRouter.cs:
    /// Calls NVIDIA NIM API starting with the agent's assigned `model_tier`,
    /// providing REASONING_ROOM (2048 + 512 = 2560 max_tokens), retrying with
    /// `enable_thinking: false` if reasoning tokens exhaust the limit, and cascading
    /// across the verified NIM model roster if a model is rate-limited or unavailable.
    pub fn call_nim_completion(
        model_tier: &str,
        system_prompt: &str,
        user_message: &str,
    ) -> anyhow::Result<String> {
        let key = Self::get_env_key("NVIDIA_API_KEY")
            .or_else(|| Self::get_env_key("NIM_API_KEY"))
            .ok_or_else(|| anyhow::anyhow!("NVIDIA_API_KEY is not set in environment"))?;

        let fallback_cascade = [
            model_tier,
            "nvidia/nemotron-3.5-lightning-30b-a3b",
            "nvidia/nemotron-3.5-bolt-e4b",
            "openai/gpt-oss-20b",
            "nvidia/nemotron-3-ultra-550b-a55b",
            "nvidia/nemotron-3.5-super-vl-120b-a12b",
            "minimaxai/minimax-m3.1-flash",
            "mistralai/mistral-large-4-1t-a52b",
        ];

        let mut tried = Vec::new();
        let mut last_err = String::new();

        for &model in &fallback_cascade {
            if model.is_empty() || tried.contains(&model) {
                continue;
            }
            tried.push(model);

            // Attempt 1: Standard call with full REASONING_ROOM (2560 tokens)
            let payload = serde_json::json!({
                "model": model,
                "messages": [
                    { "role": "system", "content": system_prompt },
                    { "role": "user", "content": user_message }
                ],
                "max_tokens": 2560,
                "temperature": 0.65
            });

            match Self::post_nim_request(&key, &payload) {
                Ok(text) => return Ok(text),
                Err(e) => {
                    last_err = format!("{}: {}", model, e);
                    // Attempt 2: Retry with enable_thinking = false (as in NimCascadeRouter.cs line 510)
                    let payload_no_think = serde_json::json!({
                        "model": model,
                        "messages": [
                            { "role": "system", "content": system_prompt },
                            { "role": "user", "content": user_message }
                        ],
                        "max_tokens": 1024,
                        "temperature": 0.65,
                        "chat_template_kwargs": { "enable_thinking": false }
                    });
                    if let Ok(text) = Self::post_nim_request(&key, &payload_no_think) {
                        return Ok(text);
                    }
                }
            }
        }

        anyhow::bail!("All NIM cascade models failed (last error: {})", last_err)
    }
}
