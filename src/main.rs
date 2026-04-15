use std::error::Error;
use tokio;
use tokio::time;
use chrono::{Utc, Duration};
use serde_json;
use serde::Deserialize;


const API_KEY: &'static str = "RGAPI-1715e66f-1312-4123-8397-e60155c16c9b";

#[derive(Debug, Deserialize)]
struct AccountResponse {
    puuid: String,
    #[serde(rename = "gameName")]
    game_name: String,
    #[serde(rename = "tagLine")]
    tag_line: String,
}

fn account_url(username: &str, tag_line: &str) -> String {
    format!("https://americas.api.riotgames.com/riot/account/v1/accounts/by-riot-id/{username}/{tag_line}")
}

fn in_game_url(puuid: &str) -> String {
    format!("https://na1.api.riotgames.com/lol/spectator/v5/active-games/by-summoner/{puuid}")
}

async fn check_in_game(puuid: &str) {
    let mut next = Utc::now();
    loop {
        next = next.checked_add_signed(Duration::seconds(60)).unwrap();
        let diff = (next - Utc::now()).to_std().unwrap();

        // todo: logic
        let client = reqwest::Client::new();

        let in_game = client.get(in_game_url(puuid))
            .header("X-Riot-Token", API_KEY)
            .send()
            .await
            .unwrap();

        match in_game.status() {
            reqwest::StatusCode::OK => println!("in game"),
            reqwest::StatusCode::NOT_FOUND => println!("not in game"),
            _ => println!("something else happen"),
        }

    println!("{:#?}", in_game.text().await.unwrap());

        time::sleep(diff).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    let username: &str = "beanlord";
    let tagline: &str = "bean";

    let client = reqwest::Client::new();
    let resp = client.get(account_url(username, tagline))
        .header("X-Riot-Token", API_KEY)
        .send()
        .await?;

    match resp.status() {
        reqwest::StatusCode::OK => (),
        reqwest::StatusCode::NOT_FOUND => println!("account not found"),
        _ => println!("who knows what we do with this"),
    }

    let r: AccountResponse = serde_json::from_str(&resp.text().await?).unwrap();
    println!("{}#{}: {}", r.game_name, r.tag_line, r.puuid);

    check_in_game(r.puuid.as_str()).await;

    Ok(())
}
