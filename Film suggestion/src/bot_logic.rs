use teloxide::dispatching::dialogue::GetChatId;
use teloxide::prelude::*;
use crate::bot_answers::*;
use crate::{get_unlocked, ChatProgress, CorrFilms, FilmNames, MainTable, Next};

pub async fn handle_message(bot: Bot, msg: Message, chat_progress: ChatProgress, main_table: MainTable, corr_films: CorrFilms, film_names: FilmNames) -> ResponseResult<()> {
    if let Some(text) = msg.text() {
        let text = text.trim();
        match get_unlocked(chat_progress.clone(), msg.chat.id) {
            Next::None {} => {
                match text {
                    "/start" => {
                        angry_bot(bot.clone(), msg.chat.id, "Welcome to 100k film recommendation bot!").await?;
                        angry_bot(bot, msg.chat.id, "Available commands:\n\
                        /add_film - enter a phase where you can add more films with scores.\n\
                        /get_pearson - if you added at least 1 film + score.").await?
                    }
                    "/add_film" => {
                        enter_add_film(bot, msg.chat.id, chat_progress).await?
                    }
                    "/get_pearson" => {
                        get_pearson(bot, msg.chat.id, chat_progress, corr_films, main_table, film_names).await?;
                    }
                    "/my_scores" => {
                        my_films(bot, msg.chat.id, chat_progress, film_names).await?
                    }
                    "/clear" => {
                        let mut prog = chat_progress.lock().unwrap();
                        prog.remove(&msg.chat.id);
                    }
                    _ => {
                        angry_bot(bot, msg.chat.id, "Available commands:\n\
                        /add_film - enter a phase where you can add more films with scores.\n\
                        /get_pearson - if you added at least 1 film + score.").await?
                    }
                }
            }
            Next::AddFilm {} => {
                if text.starts_with('/') {
                    match text {
                        "/add_film" => {
                            enter_add_film(bot, msg.chat.id, chat_progress).await?
                        }
                        "/get_pearson" => {
                            get_pearson(bot, msg.chat.id, chat_progress, corr_films, main_table, film_names).await?;
                        }
                        "/my_scores" => {
                            my_films(bot, msg.chat.id, chat_progress, film_names).await?
                        }
                        "/clear" => {
                            let mut prog = chat_progress.lock().unwrap();
                            prog.remove(&msg.chat.id);
                        }
                        _ => angry_bot(bot, msg.chat.id, "Wrong command!").await?
                    }
                }
                else {
                    add_film(bot, msg.chat.id, chat_progress, film_names, text).await?
                }
            }
            Next::AddScore { film } => {
                if let Ok(score) = text.parse::<u8>() {
                    if score >= 1 && score <= 5 {
                        add_score(bot, msg.chat.id, chat_progress, film, score).await?
                    }
                    else {
                        angry_bot(bot, msg.chat.id, "Score must be integer between 1 and 5!").await?
                    }
                }
                else if text == "/cancel" {
                    cancel_film(bot, msg.chat.id, chat_progress).await?
                }
                else {
                    angry_bot(bot, msg.chat.id, "Score must be integer between 1 and 5! Or type /cancel").await?
                }
            }
        }
    }
    
    Ok(())
}

pub async fn handle_callback(bot: Bot, q: CallbackQuery, chat_progress: ChatProgress, film_names: FilmNames) -> ResponseResult<()> {
    let chat_id = q.chat_id().unwrap();
    get_unlocked(chat_progress.clone(), chat_id);
    if let Some(text) = q.data {
        add_film(bot, chat_id, chat_progress, film_names, text.trim()).await?;
    }
    
    Ok(())
}

async fn angry_bot(bot: Bot, chat_id: ChatId, send: &str) -> ResponseResult<()> {
    bot.send_message(
        chat_id,
        send
    ).await?;

    Ok(())
}
