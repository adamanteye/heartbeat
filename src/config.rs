use serde::Deserialize;

#[derive(Deserialize)]
pub struct Listen {
    pub address: String,
    pub port: u16,
}

#[derive(Deserialize)]
pub struct Static {
    pub dir: String,
    pub token: String,
}

#[derive(Deserialize)]
pub struct Config {
    pub listen: Listen,
    pub store: Static,
}
