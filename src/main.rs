use std::error::Error;
use tokio;
use tokio::time;
use chrono::{Utc, Duration};
use serde_json;
use serde::Deserialize;
use sqlx;
use sqlx::{sqlite, Sqlite, Pool, Executor};

/* Global Variable */
const API_KEY: &'static str = "RGAPI-a5a10c2d-92ef-40a9-8cc5-2a26e3921e07";


/* JSON Structs - API Response */
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct AccountResponse {
    puuid: String,
    #[serde(rename = "gameName")]
    game_name: String,
    #[serde(rename = "tagLine")]
    tag_line: String,
}

#[allow(dead_code)]
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

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct BannedChampion {
    #[serde(rename = "pickTurn")]
    pick_turn: i32,
    #[serde(rename = "championId")]
    champion_id: i64,
    #[serde(rename = "teamId")]
    team_id: i64,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Observer {
    #[serde(rename = "encryptionKey")]
    encryption_key: String,
}

#[allow(dead_code)]
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

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Perks {
    #[serde(rename = "perkIds")]
    perk_ids: Vec<i64>,
    #[serde(rename = "perkStyle")]
    perk_style: i64,
    #[serde(rename = "perkSubStyle")]
    perk_sub_style: i64,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct GameCustomizationObject {
    category: String,
    content: String,
}

/* JSON Structs - Database */
#[derive(Debug, sqlx::FromRow)]
struct Leaguer {
    id: i32,
    name: String,
    tag_line: String,
    puuid: String,
    in_game: bool,
}

/* API Endpoints */
fn account_url(username: String, tag_line: String) -> String {
    format!("https://americas.api.riotgames.com/riot/account/v1/accounts/by-riot-id/{username}/{tag_line}")
}

fn in_game_url(puuid: &str) -> String {
    format!("https://na1.api.riotgames.com/lol/spectator/v5/active-games/by-summoner/{puuid}")
}

/* The actual function to check who's in game */
async fn check_in_game(db: &Pool<Sqlite>) {
    let mut next = Utc::now();

    loop {
        next = next.checked_add_signed(Duration::seconds(30)).unwrap();
        let diff = (next - Utc::now()).to_std().unwrap();

        // ----------- pull leaguers from database ---------------
        let players: Vec<Leaguer> = sqlx::query_as("SELECT * FROM leaguers")
            .fetch_all(db).await.unwrap();


        // ----------- check in game --------------
        let client = reqwest::Client::new();
        
        for p in players.iter() {
            let in_game = client.get(in_game_url(&p.puuid))
                .header("X-Riot-Token", API_KEY)
                .send()
                .await.unwrap();

            match in_game.status() {
                reqwest::StatusCode::OK => {
                    if !p.in_game {
                        sqlx::query(format!("UPDATE leaguers SET in_game = 1 WHERE id = {}", p.id).as_str())
                            .execute(db).await.unwrap();
                        println!("{}#{} joined a game", p.name, p.tag_line);
                    } else {
                        println!("{}#{} still in game", p.name, p.tag_line);
                    }
                },

                reqwest::StatusCode::NOT_FOUND =>  {
                    if p.in_game {
                        sqlx::query(format!("UPDATE leaguers SET in_game = 0 WHERE id = {}", p.id).as_str())
                            .execute(db).await.unwrap();
                        println!("{}#{} game ended", p.name, p.tag_line);
                    } else {
                        println!("{}#{} not in game", p.name, p.tag_line);
                    }
                },

                _ => {
                    println!("something else happen\n{}", in_game.text().await.unwrap());
                },
            }
        }

        time::sleep(diff).await;
    }
}

async fn add_leaguer(username: &String, tag_line: &String, db: &Pool<Sqlite>) -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();

    let resp = client.get(account_url(username.clone(), tag_line.clone()))
        .header("X-Riot-Token", API_KEY)
        .send()
        .await?;
    
    match resp.status() {
        reqwest::StatusCode::OK => {
            let r: AccountResponse = serde_json::from_str(&resp.text().await?).unwrap();
            println!("{}#{}: {}", r.game_name, r.tag_line, r.puuid);

            sqlx::query("INSERT INTO leaguers (name, tag_line, puuid, in_game) VALUES (?1, ?2, ?3, ?4)")
                .bind(username)
                .bind(tag_line)
                .bind(&r.puuid)
                .bind(false)
                .execute(db)
                .await.unwrap();
        },
        reqwest::StatusCode::NOT_FOUND => println!("account not found: {}#{}", username, tag_line),
        _ => {
            println!("error finding puuid: {}#{}: {}", username, tag_line, resp.text().await?);
        },
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let opt = sqlite::SqliteConnectOptions::new()
        .filename("test.db")
        .create_if_missing(true);

    let connection: Pool<Sqlite> = sqlite::SqlitePool::connect_with(opt).await.unwrap();

    connection.execute("
        CREATE TABLE IF NOT EXISTS leaguers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT,
            tag_line TEXT,
            puuid TEXT,
            in_game BOOL
        )
    ").await.unwrap();

    add_leaguer(&"beanlord".to_string(), &"bean".to_string(), &connection).await?;
    add_leaguer(&"zinkithink".to_string(), &"lista".to_string(), &connection).await?;
    check_in_game(&connection).await;

    Ok(())
}
