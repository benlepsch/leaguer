use std::error::Error;
use tokio;
use tokio::time;
use chrono::{Utc, Duration};
use serde_json;
use serde::Deserialize;


/* Global Variable */
const API_KEY: &'static str = "RGAPI-56c4953d-f9ed-4708-91c9-2757ba3d4fda";


/* JSON Structs */
#[derive(Debug, Deserialize)]
struct AccountResponse {
    puuid: String,
    #[serde(rename = "gameName")]
    game_name: String,
    #[serde(rename = "tagLine")]
    tag_line: String,
}

#[derive(Debug, Deserialize)]
struct CurrentGameInfo {
    #[serde(rename = "gameId")]
    game_id: i64,
    #[serde(rename = "gameType")]
    game_type: String,
    #[serde(rename = "gameStartTime")]
    game_start_time: i64,
    #[serde(rename = "mapId")]
    map_id: i64,
    #[serde(rename = "gameLength")]
    game_length: i64,
    #[serde(rename = "platformId")]
    platform_id: String,
    #[serde(rename = "gameMode")]
    game_mode: String,
    #[serde(rename = "bannedChampions")]
    banned_champions: Vec<BannedChampion>,
    #[serde(rename = "gameQueueConfigId")]
    game_queue_config_id: i64,
    observers: Observer,
    participants: Vec<CurrentGameParticipant>,
}

#[derive(Debug, Deserialize)]
struct BannedChampion {
    #[serde(rename = "pickTurn")]
    pick_turn: i32,
    #[serde(rename = "championId")]
    champion_id: i64,
    #[serde(rename = "teamId")]
    team_id: i64,
}

#[derive(Debug, Deserialize)]
struct Observer {
    #[serde(rename = "encryptionKey")]
    encryption_key: String,
}

#[derive(Debug, Deserialize)]
struct CurrentGameParticipant {
    #[serde(rename = "championId")]
    champion_id: i64,
    #[serde(rename = "riotId")]
    riot_id: String,
    perks: Perks,
    #[serde(rename = "profileIconId")]
    profile_icon_id: i64,
    bot: bool,
    #[serde(rename = "teamId")]
    team_id: i64,
    puuid: String,
    #[serde(rename = "spell1Id")]
    spell_1_id: i64,
    #[serde(rename = "spell2Id")]
    spell_2_id: i64,
    #[serde(rename = "gameCustomizationObjects")]
    game_customization_objects: Vec<GameCustomizationObject>,
}

#[derive(Debug, Deserialize)]
struct Perks {
    #[serde(rename = "perkIds")]
    perk_ids: Vec<i64>,
    #[serde(rename = "perkStyle")]
    perk_style: i64,
    #[serde(rename = "perkSubStyle")]
    perk_sub_style: i64,
}

#[derive(Debug, Deserialize)]
struct GameCustomizationObject {
    category: String,
    content: String,
}

/* API Endpoints */
fn account_url(username: String, tag_line: String) -> String {
    format!("https://americas.api.riotgames.com/riot/account/v1/accounts/by-riot-id/{username}/{tag_line}")
}

fn in_game_url(puuid: &str) -> String {
    format!("https://na1.api.riotgames.com/lol/spectator/v5/active-games/by-summoner/{puuid}")
}

/* The actual function to check who's in game */
async fn check_in_game(puuids: Vec<String>) {
    let mut next = Utc::now();
    loop {
        next = next.checked_add_signed(Duration::seconds(30)).unwrap();
        let diff = (next - Utc::now()).to_std().unwrap();

        let client = reqwest::Client::new();
        
        for p in puuids.iter() {
            let in_game = client.get(in_game_url(p))
                .header("X-Riot-Token", API_KEY)
                .send()
                .await
                .unwrap();

            match in_game.status() {
                reqwest::StatusCode::OK => {
                    let r: CurrentGameInfo = serde_json::from_str(&in_game.text().await.unwrap()).unwrap();

                    for player in r.participants.iter() {
                        if player.puuid == *p {
                            println!("{} in {} game", player.riot_id, r.game_mode);
                            break;
                        }
                    }
                },
                reqwest::StatusCode::NOT_FOUND => println!("not in game"),
                _ => {
                    println!("something else happen\n{}", in_game.text().await.unwrap());
                },
            }
        }

        time::sleep(diff).await;
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut players: Vec<(String, String)> = Vec::new();
    let mut puuids: Vec<String> = Vec::new();

    players.push(("beanlord".to_string(), "bean".to_string()));
    players.push(("zinkithink".to_string(), "lista".to_string()));

    let client = reqwest::Client::new();
    
    for p in players.iter() {
        let resp = client.get(account_url(p.0.clone(), p.1.clone()))
            .header("X-Riot-Token", API_KEY)
            .send()
            .await?;

        match resp.status() {
            reqwest::StatusCode::OK => {
                let r: AccountResponse = serde_json::from_str(&resp.text().await?).unwrap();
                println!("{}#{}: {}", r.game_name, r.tag_line, r.puuid);
                puuids.push(r.puuid);
            },
            reqwest::StatusCode::NOT_FOUND => println!("account not found: {}#{}", p.0, p.1),
            _ => {
                println!("error finding puuid: {}#{}: {}", p.0, p.1, resp.text().await?);
            },
        }
    }

    check_in_game(puuids).await;

    Ok(())
}
