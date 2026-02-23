use anyhow::Result;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub model: String,
    pub ollama_host: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: "qwen2.5:7b-instruct".to_string(),
            ollama_host: "http://192.168.0.107:11434/api/generate".to_string(),
        }
    }
}

impl Config {
    /// let config = Config::from_env()?;
    /// println!("Using model: {}", config.model);
    /// ```
    pub fn from_env() -> Result<Self> {
        let mut config = Config::default();

        if let Ok(val) = env::var("OLLAMA_MODEL") {
            config.model = val;
        }

        if let Ok(val) = env::var("OLLAMA_API_BASE_URL") {
            config.ollama_host = val;
        }

        Ok(config)
    }
}
