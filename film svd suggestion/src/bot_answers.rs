use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use crate::{ChatProgress, FilmNames, Next};
use crate::svd::Model;

pub async fn enter_add_film(bot: Bot, chat_id: ChatId, chat_progress: ChatProgress) -> ResponseResult<()> {
    bot.send_message(
        chat_id,
        "Send a name of a film you watched and in the next message score from 1 to 5, where 5 is the best:"
    ).await?;
    
    let mut prog = chat_progress.lock().unwrap();
    prog.get_mut(&chat_id).expect("No such dialogue").1 = Next::AddFilm {};
    
    Ok(())
}

pub async fn add_film(bot: Bot, chat_id: ChatId, chat_progress: ChatProgress, film_names: FilmNames, looking_name: &str) -> ResponseResult<()> {
    let some_film = film_names.iter().find(|(_, film_name)| {
        film_name.starts_with(looking_name)
    });
    
    let Some((&film_id, _)) = some_film else {
        bot.send_message(
            chat_id,
            "There is no such film!"
        ).await?;
        return Ok(())
    };
        
    let potential = film_names[&film_id].as_str();
    if potential == looking_name {
        bot.send_message(
            chat_id,
            "Film is chosen, type the score from 1 to 5"
        ).await?;
        
        let mut prog = chat_progress.lock().expect("No lock");
        let current = prog.get_mut(&chat_id).unwrap();
        current.1 = Next::AddScore { film: film_id }
        
    }
    else {
        bot.send_message(
            chat_id, 
            "There is no such film, did you mean:"
        ).reply_markup(InlineKeyboardMarkup::new(vec![
                vec![InlineKeyboardButton::callback(
                    potential,
                    potential
                )]
            ])
        ).await?;
    }

    Ok(())
}

pub async fn add_score(bot: Bot, chat_id: ChatId, chat_progress: ChatProgress, film: u16, score: u8) -> ResponseResult<()> {
    {
        let mut prog = chat_progress.lock().unwrap();
        let current = prog.get_mut(&chat_id).unwrap();
        if current.0.iter().all(|&(film_exist, _)| { film_exist != film}) {
            current.0.push((film, score))
        }
        else {
            let (pos, _) = current.0.iter().enumerate().find(|(_, (exist_film, _))| {
                *exist_film == film
            }).unwrap();
            current.0[pos].1 = score
        }
        current.1 = Next::AddFilm {};
    }

    bot.send_message(
        chat_id, 
        "Film and score added!"
    ).await?;
    bot.send_message(
        chat_id,
        "Add another film:"
    ).await?;
    
    Ok(())
}

pub async fn cancel_film(bot: Bot, chat_id: ChatId, chat_progress: ChatProgress) -> ResponseResult<()> {
    {
        let mut prog = chat_progress.lock().unwrap();
        prog.get_mut(&chat_id).unwrap().1 = Next::AddFilm {};
    }
    bot.send_message(
        chat_id,
        "You cancelled adding the film, you can enter another:"
    ).await?;
    
    Ok(())
}

pub async fn my_films(bot: Bot, chat_id: ChatId, chat_progress: ChatProgress, film_names: FilmNames) -> ResponseResult<()> {
    let user_scores = {
        let prog = chat_progress.lock().unwrap();
        if let Some(user_scores) = prog.get(&chat_id) {
            user_scores.0.clone()
        }
        else {
            vec![]
        }
    };

    if user_scores.len() == 0 {
        bot.send_message(
            chat_id,
            "You haven't added any films!"
        ).await?;
    }
    else {
        let answer: String = user_scores.into_iter().enumerate().map(|(num, (film_id, score))| {
            format!("\n{}. {}, score: {}", num + 1, film_names[&film_id], score)
        }).collect();

        bot.send_message(
            chat_id,
            format!("Here are the films you added:{}", answer)
        ).await?;
    }
    
    Ok(())
}

pub async fn get_sdv(bot: Bot, chat_id: ChatId, chat_progress: ChatProgress, film_names: FilmNames, model: std::sync::Arc<Model>) -> ResponseResult<()> {
    let answer = request_sdv(chat_id, chat_progress, model).await?;
    let Some(answer) = answer else {
        bot.send_message(
            chat_id,
            "Add films you watched first!"
        ).await?;
        return Ok(())
    };
    
    let bot_write = async |ans: Vec<(u16, f64)>, send: String| {
        let mut score_str = String::default();
        for (num, (film_id, score)) in ans.into_iter().enumerate() {
            if num >= 9 {
                break;
            }
            score_str += format!("{}. {}, expected score: ⭐ {:.2}/5 ⭐\n", num + 1, film_names[&film_id], score).as_str();
        };
        
        bot.send_message(
            chat_id, 
            send + score_str.as_str()
        ).await.expect("No send");
    };
    
    if answer.len() == 0 {
        bot_write(answer, String::from("Sorry, your taste is too hard to match! Try to /clear chat and add less films!")).await;
    }
    else if answer.len() == 1 {
        bot_write(answer, String::from("You taste is hard to match! I could find only 1 matching film:\n")).await;
    }
    else if answer.len() <= 5 {
        bot_write(answer, String::from("I could find several good films for you!\n")).await;
    }
    else {
        bot_write(answer, String::from("Here are you top recommended films:\n")).await;
    }
    
    
    Ok(())
}

async fn request_sdv(chat_id: ChatId, chat_progress: ChatProgress, model: std::sync::Arc<Model>) -> ResponseResult<Option<Vec<(u16, f64)>>> {
    let user_scores = {
        let progress = chat_progress.lock().unwrap();
        if let Some(scores) = progress.get(&chat_id) {
            scores.0.clone()
        }
        else {
            vec![]
        }
    };
    
    if user_scores.is_empty() {
        return Ok(None);
    }
    
    let top_10 = model.predict(&user_scores);
    
    Ok(Some(top_10))
}
