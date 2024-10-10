use std::{collections::HashSet, io::Write};

use rspotify::{model::PlayableItem, prelude::{BaseClient, OAuthClient}, AuthCodeSpotify, Config, Credentials, OAuth};

static APP_SCOPES: [&str; 1] = [
    "user-read-currently-playing",
];

async fn login(spotify: &AuthCodeSpotify) {
    let url = spotify.get_authorize_url(false).unwrap();
    match open::that(&url) {
        Ok(_) => println!("A browser should have opened. Please log in and paste the URL you are redirected to."),
        Err(_) => println!("If a browser did not open, please open the following URL in your browser: {}", url),
    }
    print!("URL: ");
    std::io::stdout().flush().unwrap();
    let stdin = std::io::stdin();
    let mut buffer = String::new();
    stdin.read_line(&mut buffer).unwrap();
    let buffer = buffer.trim();
    let code = spotify.parse_response_code(buffer).unwrap();

    spotify.request_token(&code).await.unwrap();
    println!("Got token: {:?}", spotify.get_token().lock().await);
    spotify.write_token_cache().await.unwrap();

    println!("{:?}", spotify.current_user_playing_item().await.unwrap());
}

#[tokio::main]
async fn main() {
    let conf = Config {
        token_cached: true,
        token_refreshing: true,
        ..Config::default()
    };
    let spotify = AuthCodeSpotify::with_config(
        Credentials::from_env().unwrap(), 
        OAuth::from_env(HashSet::from(APP_SCOPES.map(|s| s.to_owned()))).unwrap(),
        conf,
    );
    match spotify.read_token_cache(true).await {
        Ok(t) => {
            match t {
                Some(t) => *spotify.get_token().lock().await.unwrap() = Some(t),
                None => login(&spotify).await,
            }
            spotify.refresh_token().await.unwrap();
            match spotify.current_user_playing_item().await.unwrap() {
                Some(item) => match item.item {
                    Some(i) => match i {
                        PlayableItem::Track(t) => {
                            let id = t.id.unwrap();
                            if spotify.current_user_saved_tracks_contains(vec![id.clone()]).await.unwrap()[0] {
                                spotify.current_user_saved_tracks_delete(vec![id]).await.unwrap();
                            } else {
                                spotify.current_user_saved_tracks_add(vec![id]).await.unwrap();
                            }
                        },
                        PlayableItem::Episode(e) => println!("Currently playing: {}", e.name),
                    }
                    None => println!("Nothing is currently playing."),
                },
                None => println!("Nothing is currently playing."),
            }
        }
        Err(_) => login(&spotify).await,
    }
}
