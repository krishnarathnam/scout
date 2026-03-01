use anyhow::Result;
use std::env;
use std::fs;
use std::path::PathBuf;

fn config_path() -> PathBuf {
    env::var("SCOUT_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(".scout_config"))
}

#[derive(Debug, Clone)]
pub struct Config {
    pub model: String,
    pub ollama_host: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: "qwen2.5:7b-instruct".to_string(),
            ollama_host: "http://127.0.0.1:11434/api/generate".to_string(),
        }
    }
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let mut config = Config::default();

        if let Ok(content) = fs::read_to_string(config_path()) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=') {
                    let k = k.trim();
                    let v = v.trim().trim_matches('"');
                    match k {
                        "OLLAMA_MODEL" => config.model = v.to_string(),
                        "OLLAMA_API_BASE_URL" => config.ollama_host = v.to_string(),
                        _ => {}
                    }
                }
            }
        }

        if let Ok(val) = env::var("OLLAMA_MODEL") {
            config.model = val;
        }
        if let Ok(val) = env::var("OLLAMA_API_BASE_URL") {
            config.ollama_host = val;
        }

        Ok(config)
    }

    pub fn save_model(&self, model: &str) -> Result<()> {
        let path = config_path();
        let mut content = String::new();
        if let Ok(existing) = fs::read_to_string(&path) {
            content = existing;
        }
        let mut found = false;
        let mut new_lines: Vec<String> = content
            .lines()
            .map(|line| {
                if line.trim().starts_with("OLLAMA_MODEL=") {
                    found = true;
                    format!("OLLAMA_MODEL={}", model)
                } else {
                    line.to_string()
                }
            })
            .collect();
        if !found {
            new_lines.push(format!("OLLAMA_MODEL={}", model));
        }
        if !content.contains("OLLAMA_API_BASE_URL=") {
            new_lines.push(format!("OLLAMA_API_BASE_URL={}", self.ollama_host));
        }
        fs::write(path, new_lines.join("\n"))?;
        Ok(())
    }

    /// Base URL for Ollama (no /api/generate suffix) for listing models.
    pub fn ollama_base_url(&self) -> &str {
        self.ollama_host
            .trim_end_matches("/api/generate")
            .trim_end_matches('/')
    }
}
