use std::collections::HashMap;
use crate::{CorrFilms, MainTable};

#[derive(Debug, serde::Deserialize)]
struct Fields {
    user: u16,
    film: u16,
    rating: u8
}

pub fn load_main_table() -> Result<HashMap<u16, HashMap<u16, u8>>, Box<dyn std::error::Error>> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(false)
        .flexible(true)
        .from_path("./scores.csv")?;
    
    let mut new_map: HashMap<u16, HashMap<u16, u8>> = HashMap::with_capacity(1700);

    for record_maybe in reader.deserialize::<Fields>() {
        if let Ok(record) = record_maybe {
            match new_map.get(&record.film) {
                None => {
                    new_map.insert(record.film, HashMap::with_capacity(950));
                    new_map.get_mut(&record.film).unwrap()
                }
                _ => {
                    new_map.get_mut(&record.film).unwrap()
                }
            }.insert(record.user, record.rating);
        }
        else {
            panic!("Wrong main table file!");
        }
    }
    
    new_map.shrink_to_fit();
    for (_, value) in &mut new_map {
        value.shrink_to_fit();
    }

    Ok(new_map)
}

pub fn load_film_names() -> Result<HashMap<u16, String>, Box<dyn std::error::Error>> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'|')
        .has_headers(false)
        .flexible(true)
        .from_path("./u.item")?;
    
    let mut films: HashMap<u16, String> = HashMap::with_capacity(1700);
    
    for record_maybe in reader.deserialize::<(u16, String)>() {
        if let Ok(rec) = record_maybe {
            films.insert(rec.0, rec.1);
        }
        else {
            panic!("Wrong films-names file!");
        }
    }
    
    films.shrink_to_fit();
    Ok(films)
}

pub fn create_pearson (main_table: MainTable) -> HashMap<(u16, u16), f32> {
    let mut corr = HashMap::with_capacity(2890000); // 2 890 000 ~ 1700 * 1700
    
    for (&film_1, rank_1) in &*main_table {
        for (&film_2, rank_2) in &*main_table {
            if film_1 != film_2 && corr.get(&(film_2, film_1)).is_none() {
                let mut mid_1 = 0f32;
                let mut mid_2 = 0f32;
                let mut count = 0f32;

                for (user, &score_1) in rank_1 {
                    if let Some(&score_2) = rank_2.get(user) {
                        count += 1f32;
                        mid_1 += score_1 as f32;
                        mid_2 += score_2 as f32;
                    }
                }

                if count > 2f32 {
                    mid_1 /= count;
                    mid_2 /= count;
                    let mut upper = 0f32;
                    let mut bottom_1 = 0f32;
                    let mut bottom_2 = 0f32;
                    for (user, &score_1) in rank_1 {
                        if let Some(&score_2) = rank_2.get(user) {
                            upper += (score_1 as f32 - mid_1) * (score_2 as f32 - mid_2);
                            bottom_1 += (score_1 as f32 - mid_1).powi(2);
                            bottom_2 += (score_2 as f32 - mid_2).powi(2);
                        }
                    }

                    let correlation = upper / (bottom_1.sqrt() * bottom_2.sqrt());
                    corr.insert((film_1, film_2), correlation);
                }
            }
        }
    }
    
    corr.shrink_to_fit();
    corr
}

pub async fn predict_pearson(
    corr_films: CorrFilms,
    user_scores: &Vec<(u16, u8)>,
    all_films: &MainTable,
    k: usize,
    min_similarity: f32,
) -> Vec<(u16, f32)> {
    let mut predictions = Vec::new();
    for &film_id in all_films.keys() {
        if user_scores.iter().find(|&&(film, _)| { film == film_id }).is_some() {
            continue;
        }

        let mut similar_films: Vec<(u16, f32, f32)> = Vec::new();

        for &(rated_film_id, user_rating) in user_scores {
            let similarity = corr_films
                .get(&(film_id, rated_film_id))
                .or_else(|| corr_films.get(&(rated_film_id, film_id)))
                .copied()
                .unwrap_or(0.0);

            if similarity >= min_similarity {
                similar_films.push((rated_film_id, similarity, user_rating as f32));
            }
        }

        if similar_films.is_empty() {
            continue;
        }

        similar_films.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        similar_films.truncate(k);

        let weighted_sum: f32 = similar_films
            .iter()
            .map(|(_, sim, rating)| sim * rating)
            .sum();

        let sum_weights: f32 = similar_films
            .iter()
            .map(|(_, sim, _)| sim)
            .sum();

        let prediction = weighted_sum / sum_weights;

        predictions.push((film_id, prediction));
    }

    predictions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    predictions
}

/*
/// needs film -> user -> score
pub fn correlation_custom(main_table: &MainTable) -> HashMap<(u16, u16), f32> {
    let mut corr: HashMap<(u16, u16), f32> = HashMap::with_capacity(1700);
    
    for (&film1, value1) in main_table {
        for (&film2, value2) in main_table {
            
            if film1 != film2 && corr.get(&(film2, film1)).is_none() {
                let mut found = 0f32;
                corr.insert((film1, film2), 0.);
                let current_score = corr.get_mut(&(film1, film2)).unwrap();
                for (user, &score1) in value1 {
                    if let Some(&score2) = value2.get(user) {
                        *current_score += (score1 as f32 - score2 as f32).abs() / -2.0 + 1.0;
                        found += 1.0;
                    }
                    if found > 0.9 {
                        *current_score /= found;
                    }
                }
            }
            
        }
    }
    
    corr
}
*/
