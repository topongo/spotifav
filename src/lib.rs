use std::{collections::HashSet, fs::{create_dir_all, File}, io::Write};

use rspotify::{model::PlayableItem, prelude::{BaseClient, OAuthClient}, AuthCodeSpotify, Config, Credentials, OAuth};
use serde::Deserialize;

static APP_SCOPES: [&str; 1] = [
    "user-read-currently-playing",
];

async fn login(spotify: &AuthCodeSpotify) -> Result<(), Box<dyn std::error::Error>> {
    let url = spotify.get_authorize_url(false)?;
    match open::that(&url) {
        Ok(_) => println!("A browser should have opened. Please log in and paste the URL you are redirected to."),
        Err(_) => println!("If a browser did not open, please open the following URL in your browser: {}", url),
    }
    print!("URL: ");
    std::io::stdout().flush()?;
    let stdin = std::io::stdin();
    let mut buffer = String::new();
    stdin.read_line(&mut buffer)?;
    let buffer = buffer.trim();
    let code = spotify.parse_response_code(buffer).unwrap();

    spotify.request_token(&code).await?;
    spotify.write_token_cache().await?;

    Ok(())
}

fn read_configs() -> Result<AuthConfig, Box<dyn std::error::Error>> {
    let configs = directories::ProjectDirs::from("org", "prabo", "spotifav")
        .ok_or("Failed to get project directories")?
        .config_dir()
        .join("config.toml");
    if !configs.exists() {
        create_dir_all(configs.parent().unwrap())?;
        File::create(&configs)?;
        println!("Please fill in the following informations in the file at: {}", configs.display());
        println!("  client_id = \"<your client id>\"");
        println!("  client_secret = \"<your client secret>\"");
        println!("  redirect_uri = \"http://localhost:8888\"");
        println!("More information can be found at: https://developer.spotify.com/documentation/web-api/tutorials/code-flow/");
        println!("Generally create a new app at: https://developer.spotify.com/dashboard/");
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Config file not found").into());
    }
    let conf: AuthConfig = toml::from_str(&std::fs::read_to_string(configs)?)?;
    Ok(conf)
}

#[derive(Deserialize, Debug)]
struct AuthConfig {
    #[serde(with = "MockCredentials")]
    pub creds: Credentials,
    #[serde(with = "MockOAuth")]
    pub oauth: OAuth,
}

#[derive(Deserialize)]
#[serde(remote = "Credentials")]
struct MockCredentials {
    pub id: String,
    pub secret: Option<String>,
}

fn get_scopes() -> HashSet<String> {
    HashSet::from(APP_SCOPES.map(|s| s.to_owned()))
}

fn scrape_from_remote() -> String {
    OAuth::default().state
}

#[derive(Deserialize)]
#[serde(remote = "OAuth")]
struct MockOAuth {
    pub redirect_uri: String,
    #[serde(default = "get_scopes")]
    pub scopes: HashSet<String>,
    pub proxies: Option<String>,
    #[serde(default = "scrape_from_remote", skip_serializing)]
    pub state: String,
}

pub async fn do_toggle() -> Result<(), Box<dyn std::error::Error>> {
    let conf = Config {
        token_cached: true,
        token_refreshing: true,
        ..Config::default()
    };
    let auth_conf = match Credentials::from_env() {
        Some(c) => match OAuth::from_env(HashSet::from(APP_SCOPES.map(|s| s.to_owned()))) {
            Some(o) => AuthConfig {
                creds: c,
                oauth: o,
            },
            None => read_configs()?,
        },
        None => read_configs()?, 
    };
    let spotify = AuthCodeSpotify::with_config(
        auth_conf.creds,
        auth_conf.oauth,
        conf,
    );
    match spotify.read_token_cache(true).await {
        Ok(t) => {
            match t {
                Some(t) => *spotify.get_token().lock().await.expect("cannot lock spotify token mutex") = Some(t),
                None => login(&spotify).await?,
            }
            spotify.refresh_token().await?;
            match spotify.current_user_playing_item().await? {
                Some(item) => match item.item {
                    Some(i) => match i {
                        PlayableItem::Track(t) => {
                            let id = t.id.ok_or("Failed to get track id")?;
                            if spotify.current_user_saved_tracks_contains(vec![id.clone()]).await?[0] {
                                spotify.current_user_saved_tracks_delete(vec![id]).await?;
                            } else {
                                spotify.current_user_saved_tracks_add(vec![id]).await?;
                            }
                        },
                        PlayableItem::Episode(e) => println!("Currently playing: {}", e.name),
                    }
                    None => println!("Nothing is currently playing."),
                },
                None => println!("Nothing is currently playing."),
            }
        }
        Err(_) => login(&spotify).await?,
    }
    Ok(())
}

