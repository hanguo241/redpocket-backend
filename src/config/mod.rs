use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub server_host: String,
    pub server_port: u16,
    pub signer_private_key: String,
    pub relayer_private_key: String,
    pub jwt_secret: String,
    pub share_url_host: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set"),
            server_host: env::var("SERVER_HOST")
                .unwrap_or_else(|_| "0.0.0.0".to_string()),
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .expect("SERVER_PORT must be a number"),
            signer_private_key: env::var("SIGNER_PRIVATE_KEY")
                .expect("SIGNER_PRIVATE_KEY must be set"),
            relayer_private_key: env::var("RELAYER_PRIVATE_KEY")
                .unwrap_or_default(),
            jwt_secret: env::var("JWT_SECRET")
                .expect("JWT_SECRET must be set"),
            share_url_host: env::var("SHARE_URL_HOST")
                .unwrap_or_else(|_| "https://redpacket.com".to_string()),
        }
    }

    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.server_host, self.server_port)
    }
}
