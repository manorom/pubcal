use hyper::Uri;
use serde::{de, Deserialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::net::IpAddr;
use uuid::Uuid;

#[derive(Debug, Deserialize, Clone)]
pub struct Calendar {
    urlpath: String,
    upstream_user: String,
    pub collection_id: Uuid,
}

impl Calendar {
    pub fn collection_uri(&self, base_url: Uri) -> Uri {
        let path = format!("{}/{}/", base_url.path(), self.collection_id);
        Uri::builder()
            .scheme(base_url.scheme().unwrap().clone())
            .authority(base_url.authority().unwrap().clone())
            .path_and_query(path)
            .build()
            .unwrap()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Server {
    pub bind_addr: IpAddr,
    pub bind_port: u16,
    #[serde(deserialize_with = "deserialize_hyper_uri")]
    pub upstream_base_url: Uri,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: Server,
    credentials: HashMap<String, String>,
    calendars: Vec<Calendar>,
}

impl Config {
    pub fn validate(&self) -> anyhow::Result<()> {
        // check that we have credentials to actually access every calendar
        for calendar in &self.calendars {
            anyhow::ensure!(
                self.credentials.contains_key(&calendar.upstream_user),
                "Calendar with path {} references user for which no credentials were given",
                calendar.urlpath
            );
        }

        Ok(())
    }
    pub fn load(main_config: &str) -> anyhow::Result<Self> {
        let mut contents = String::new();
        File::open(main_config)?.read_to_string(&mut contents)?;
        let config: Config = toml::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }
    pub fn match_request(&self, uri: &Uri) -> Option<(&Calendar, &str)> {
        let cal: &Calendar = self.calendars.iter().find(|c| c.urlpath == uri.path())?;
        let credentials: &str = self
            .credentials
            .get(&cal.upstream_user)
            .expect("Matched Calendar without UpstreamUser entry");
        Some((cal, credentials))
    }
}

fn deserialize_hyper_uri<'de, D>(deserializer: D) -> Result<Uri, D::Error>
where
    D: de::Deserializer<'de>,
{
    let buf: &str = de::Deserialize::deserialize(deserializer)?;
    Uri::try_from(buf).map_err(serde::de::Error::custom)
}
