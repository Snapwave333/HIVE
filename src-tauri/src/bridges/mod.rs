use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

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
        let (google_connected, google_account) = match Self::check_google_auth() {
            Ok(out) => {
                let email = out
                    .lines()
                    .find_map(|l| l.split_whitespace().next())
                    .unwrap_or("elikenny333@gmail.com")
                    .to_string();
                (true, email)
            }
            Err(_) => (false, "elikenny333@gmail.com".to_string()),
        };

        let obsidian_available = Self::check_obsidian().is_ok();
        let browser_relay_available = Self::check_browser_relay().is_ok();
        let voice_engine_available = Self::check_voice_engine().is_ok();
        let vault_path = Self::resolve_vault_dir().to_string_lossy().to_string();
        let llm_provider = if Self::get_env_key("NVIDIA_API_KEY").is_some() {
            "NVIDIA NIM Cascade".to_string()
        } else if Self::get_env_key("ANTHROPIC_API_KEY").is_some() {
            "Anthropic Claude + NIM Cascade".to_string()
        } else {
            "Local Autonomous Engine + NIM Tier Routing".to_string()
        };

        BridgeStatus {
            google_connected,
            google_account,
            obsidian_available,
            obsidian_vault_path: vault_path,
            browser_relay_available,
            voice_engine_available,
            llm_provider,
        }
    }

    /// Validate CLI arguments to prevent shell injection
    fn sanitize_arg(arg: &str) -> anyhow::Result<String> {
        let forbidden = ['&', '|', ';', '`', '$', '>', '<'];
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

        // Allowlist top-level gog subcommands
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
            // Strip surrounding single quotes if user typed e.g. ''
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
        cmd.creation_flags(0x08000000);
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
        let output = Command::new("where").arg("Obsidian.com").output()?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Ok("Local Vault Mode".to_string())
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
        let note_path = dir.join(filename);
        std::fs::write(&note_path, content)?;
        Ok(note_path)
    }

    pub fn search_obsidian_notes(query: &str) -> anyhow::Result<Vec<serde_json::Value>> {
        let dir = Self::resolve_vault_dir();
        let mut results = Vec::new();
        let q_lower = query.to_lowercase();

        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("md") {
                    if let Ok(text) = std::fs::read_to_string(&path) {
                        let name = path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        if q_lower.is_empty()
                            || name.to_lowercase().contains(&q_lower)
                            || text.to_lowercase().contains(&q_lower)
                        {
                            results.push(serde_json::json!({
                                "title": name,
                                "path": path.to_string_lossy(),
                                "snippet": text.chars().take(200).collect::<String>(),
                            }));
                        }
                    }
                }
            }
        }
        Ok(results)
    }

    /// OpenCode Chrome Relay bridge
    pub fn check_browser_relay() -> anyhow::Result<String> {
        let output = Command::new("where").arg("browser-relay").output()?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            anyhow::bail!("browser-relay CLI not found on PATH")
        }
    }

    pub fn browser_relay_exec(subcmd: &str, args: &[&str]) -> anyhow::Result<String> {
        let allowed = ["status", "tabs", "list-tabs", "snapshot", "extract", "help", "--version"];
        if !allowed.contains(&subcmd) {
            anyhow::bail!("Browser relay command '{}' requires Level-3 Owner Gate or is not allowlisted", subcmd);
        }
        let mut cmd_args = vec!["/C", "browser-relay", subcmd];
        for a in args {
            Self::sanitize_arg(a)?;
            cmd_args.push(a);
        }
        let mut cmd = Command::new("cmd");
        cmd.args(&cmd_args);
        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000);
        let output = cmd.output()?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if output.status.success() {
            Ok(stdout)
        } else {
            Ok(format!("Relay status: {} {}", stdout.trim(), stderr.trim()))
        }
    }

    /// Voice Synthesis Bridge (Kokoro / Windows SAPI Neural WAV Cache)
    pub fn check_voice_engine() -> anyhow::Result<String> {
        let output = Command::new("where").arg("kokoro").output()?;
        if output.status.success() {
            Ok("Kokoro Neural TTS".to_string())
        } else {
            Ok("Kokoro + Windows SAPI WAV Synthesizer".to_string())
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
            // Strip markdown formatting before speaking
            let clean_text: String = text
                .chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace() || ",.?!:;'-".contains(*c))
                .take(400)
                .collect();

            // Agent-specific voice rate (-2 to +2)
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

            let status = Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
                .output();

            if status.is_err() || !dest.exists() {
                // Fallback valid PCM WAV header if SAPI unavailable
                let dummy_wav = [
                    0x52, 0x49, 0x46, 0x46, 0x24, 0x00, 0x00, 0x00, 0x57, 0x41, 0x56, 0x45,
                    0x66, 0x6d, 0x74, 0x20, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00,
                    0x44, 0xac, 0x00, 0x00, 0x88, 0x58, 0x01, 0x00, 0x02, 0x00, 0x10, 0x00,
                    0x64, 0x61, 0x74, 0x61, 0x00, 0x00, 0x00, 0x00,
                ];
                std::fs::write(&dest, dummy_wav)?;
            }
        }

        Ok(dest)
    }

    /// Retrieve API key from process env or Windows User Registry
    pub fn get_env_key(name: &str) -> Option<String> {
        if let Ok(v) = std::env::var(name) {
            if !v.trim().is_empty() {
                return Some(v);
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
                        return Some(val);
                    }
                }
            }
        }
        None
    }

    /// Call NVIDIA NIM API with agent's assigned model_tier if NVIDIA_API_KEY is present
    pub fn call_nim_completion(
        model_tier: &str,
        system_prompt: &str,
        user_message: &str,
    ) -> anyhow::Result<String> {
        let key = Self::get_env_key("NVIDIA_API_KEY")
            .ok_or_else(|| anyhow::anyhow!("NVIDIA_API_KEY not configured"))?;

        let payload = serde_json::json!({
            "model": model_tier,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": user_message }
            ],
            "max_tokens": 512,
            "temperature": 0.6
        });

        let payload_str = serde_json::to_string(&payload)?;
        let output = Command::new("curl.exe")
            .args([
                "-sS",
                "--max-time",
                "15",
                "-H",
                &format!("Authorization: Bearer {}", key),
                "-H",
                "Content-Type: application/json",
                "-d",
                &payload_str,
                "https://integrate.api.nvidia.com/v1/chat/completions",
            ])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("curl to NIM failed");
        }

        let resp_str = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&resp_str)?;
        if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                return Ok(trimmed.to_string());
            }
        }
        anyhow::bail!("NIM response did not contain message content")
    }
}
