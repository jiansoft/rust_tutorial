use crate::internal::logging;
use anyhow::*;
use config::{Config as config_config, File as config_file};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, fs, io, path::PathBuf, result::Result::Ok, str::FromStr};

const CONFIG_PATH: &str = "app.json";

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct App {
    #[serde(default)]
    pub afraid: Afraid,
    pub postgresql: PostgreSQL,
    pub bot: Bot,
}

const AFRAID_TOKEN: &str = "AFRAID_TOKEN";
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Afraid {
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub path: String,
}

const POSTGRESQL_HOST: &str = "POSTGRESQL_HOST";
const POSTGRESQL_PORT: &str = "POSTGRESQL_PORT";
const POSTGRESQL_USER: &str = "POSTGRESQL_USER";
const POSTGRESQL_PASSWORD: &str = "POSTGRESQL_PASSWORD";
const POSTGRESQL_DB: &str = "POSTGRESQL_DB";

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct PostgreSQL {
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub port: i32,
    #[serde(default)]
    pub user: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub db: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Bot {
    pub telegram: Telegram,
}

const TELEGRAM_TOKEN: &str = "TELEGRAM_TOKEN";
const TELEGRAM_ALLOWED: &str = "TELEGRAM_ALLOWED";

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Telegram {
    pub allowed: HashMap<i64, String>,
    pub token: String,
}

pub static SETTINGS: Lazy<App> = Lazy::new(|| App::get().expect("Config error"));

impl App {
    pub fn new() -> Self {
        //讀取設定檔
        let config_txt = read_config_file();
        //取得文字檔的內容
        let text_content = config_txt.unwrap_or_else(|_| Default::default());

        if text_content.is_empty() {
            return Default::default();
        }

        //轉成Config 物件
        let from_json = serde_json::from_str::<App>(text_content.as_str());
        match from_json {
            Err(why) => {
                logging::error_file_async(format!(
                    "I can't read the config context because {:?}",
                    why
                ));
                Default::default()
            }
            Ok(_config) => _config.override_with_env(),
        }
    }

    fn get() -> Result<Self> {
        let config_path = config_path();
        if config_path.exists() {
            let config: App = config_config::builder()
                .add_source(config_file::from(config_path))
                .build()?
                .try_deserialize()?;
            return Ok(config.override_with_env());
        }

        Ok(App::from_env())
    }

    /// 從 env 中讀取設定值
    fn from_env() -> Self {
        let tg_allowed = env::var(TELEGRAM_ALLOWED).expect(TELEGRAM_ALLOWED);
        let mut allowed_list: HashMap<i64, String> = Default::default();
        if !tg_allowed.is_empty() {
            if let Ok(allowed) = serde_json::from_str::<HashMap<i64, String>>(&tg_allowed) {
                allowed_list = allowed;
            }
        }

        App {
            afraid: Afraid {
                token: env::var(AFRAID_TOKEN).expect(AFRAID_TOKEN),
                url: "".to_string(),
                path: "".to_string(),
            },
            postgresql: PostgreSQL {
                host: env::var(POSTGRESQL_HOST).expect(POSTGRESQL_HOST),
                port: i32::from_str(
                    &env::var(POSTGRESQL_PORT).unwrap_or_else(|_| "5432".to_string()),
                )
                .unwrap_or(5432),
                user: env::var(POSTGRESQL_USER).expect(POSTGRESQL_USER),
                password: env::var(POSTGRESQL_PASSWORD).expect(POSTGRESQL_PASSWORD),
                db: env::var(POSTGRESQL_DB).expect(POSTGRESQL_DB),
            },
            bot: Bot {
                telegram: Telegram {
                    allowed: allowed_list,
                    token: env::var(TELEGRAM_TOKEN).expect(TELEGRAM_TOKEN),
                },
            },
        }
    }

    /// 將來至於 env 的設定值覆蓋掉 json 上的設定值
    fn override_with_env(mut self) -> Self {
        if let Ok(token) = env::var(AFRAID_TOKEN) {
            self.afraid.token = token;
        }

        if let Ok(host) = env::var(POSTGRESQL_HOST) {
            self.postgresql.host = host;
        }

        if let Ok(port) = env::var(POSTGRESQL_PORT) {
            self.postgresql.port = i32::from_str(&port).unwrap_or(5432);
        }

        if let Ok(user) = env::var(POSTGRESQL_USER) {
            self.postgresql.user = user;
        }

        if let Ok(password) = env::var(POSTGRESQL_PASSWORD) {
            self.postgresql.password = password;
        }

        if let Ok(db) = env::var(POSTGRESQL_DB) {
            self.postgresql.db = db;
        }

        if let Ok(tg_allowed) = env::var(TELEGRAM_ALLOWED) {
            match serde_json::from_str::<HashMap<i64, String>>(&tg_allowed) {
                Ok(allowed) => {
                    self.bot.telegram.allowed = allowed;
                }
                Err(why) => {
                    logging::error_file_async(format!(
                        "Failed to serde_json because: {:?} \r\n {}",
                        why, &tg_allowed
                    ));
                }
            }
        }

        if let Ok(token) = env::var(TELEGRAM_TOKEN) {
            self.bot.telegram.token = token
        }

        self
    }
}

/// 回傳設定檔的路徑
fn config_path() -> PathBuf {
    PathBuf::from(CONFIG_PATH)
}

/// 讀取預設的設定檔
fn read_config_file() -> Result<String, io::Error> {
    let p = config_path();
    read_text_file(p)
}

/// 回傳指定路徑的文字檔的內容
pub(crate) fn read_text_file(path: PathBuf) -> Result<String, io::Error> {
    fs::read_to_string(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time};

    #[tokio::test]
    async fn test_init() {
        dotenv::dotenv().ok();
        logging::info_file_async(format!(
            "SETTINGS.postgresql: {:#?}\r\nSETTINGS.secret: {:#?}\r\n",
            SETTINGS.postgresql, SETTINGS.bot
        ));
        let mut map: HashMap<i64, String> = HashMap::new();
        map.insert(123, "QQ".to_string());
        map.insert(456, "QQ".to_string());
        let json_str = serde_json::to_string(&map).expect("TODO: panic message");

        logging::info_file_async(format!("serde_json: {}\r\n", &json_str));
        match serde_json::from_str::<HashMap<i64, String>>(&json_str) {
            Ok(json) => {
                logging::info_file_async(format!("json: {:?}\r\n", json));
            }
            Err(why) => {
                logging::error_file_async(format!(
                    "Failed to serde_json because: {:?} \r\n {}",
                    why, &json_str
                ));
            }
        }
        thread::sleep(time::Duration::from_secs(1));
    }
}
