#[derive(Clone)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub paystack_secret_key: String,
}

const DEFAULT_PORT: u16 = 3000;

impl Config {
    pub fn from_env() -> Self {
        let port = std::env::var("PORT")
            .unwrap_or_else(|_| DEFAULT_PORT.to_string())
            .parse::<u16>()
            .expect("PORT must be a valid u16");

        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in the environment");

        let paystack_secret_key = std::env::var("PAYSTACK_SECRET_KEY")
            .expect("PAYSTACK_SECRET_KEY must be set in the environment");

        Config {
            port,
            database_url,
            paystack_secret_key,
        }
    }
}
