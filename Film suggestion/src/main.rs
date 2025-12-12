mod storage;
mod bot_logic;
mod bot_answers;

use teloxide::types::BotCommand;
use storage::*;
use bot_logic::*;
use std::sync::{Arc, Mutex};
use std::collections::{HashMap};
use teloxide::{Bot, dptree, };
use teloxide::prelude::*;
/// film -> HashMap(user) -> score
type MainTable = Arc<HashMap<u16, HashMap<u16, u8>>>;

/// (film, film) -> score
type CorrFilms = Arc<HashMap<(u16, u16), f32>>;

/// film -> name
type FilmNames = Arc<HashMap<u16, String>>;

/// chat -> (Vec(film, score), state)
type ChatProgress = Arc<Mutex<HashMap<ChatId, (Vec<(u16, u8)>, Next)>>>;


/// Shows the next expected message
#[derive(Clone, Debug)]
enum Next {
    None {},
    AddFilm {},
    AddScore { film: u16 }
}

fn get_unlocked(chat_progress: ChatProgress, k: ChatId) -> Next {
    let mut prog = chat_progress.lock().expect("No lock");
    if let Some(ok) = prog.get(&k) {
        ok.1.clone()
    }
    else {
        prog.insert(k, (vec![], Next::None {}));
        prog[&k].1.clone()
    }
}


#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    log::info!("Loading needed files...");
    let film_names: FilmNames = Arc::new(load_film_names().expect("No load films"));
    let main_table: MainTable = Arc::new(load_main_table().unwrap());
    let corr_films: CorrFilms = Arc::new(create_pearson(main_table.clone()));
    let chat_progress: ChatProgress = Arc::new(Mutex::new(HashMap::new()));

    unsafe {
        std::env::set_var("RUST_LOG".to_string(), "info".to_string());
    }
    pretty_env_logger::init();
    let token = std::fs::read_to_string(
        "token.env".to_string()
    )
        .expect("No token file provided!")
        .split('\n')
        .nth(0)
        .expect("No nth")
        .trim()
        .to_string();

    let bot = Bot::new(token);
    bot.set_my_commands(vec![
        BotCommand::new("/start", "Launch a bot"),
        BotCommand::new("/add_film", "Enter a part to add films"),
        BotCommand::new("/get_pearson", "Get your films recommendations"),
        BotCommand::new("/my_scores", "Check what films you already added"),
        BotCommand::new("/clear", "Clear the data from this dialogue")
    ]).await.unwrap();
    
    log::info!("Bot is launched.");

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(handle_message))
        .branch(Update::filter_callback_query().endpoint(handle_callback));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![main_table, corr_films, chat_progress, film_names])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
