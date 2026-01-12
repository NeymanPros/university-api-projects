use std::collections::HashMap;

#[derive(Debug, serde::Deserialize)]
struct Fields {
    user: u16,
    film: u16,
    rating: u8
}

pub fn load_main_table() -> Result<HashMap<u16, Vec<(u16, u8)>>, Box<dyn std::error::Error>> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(false)
        .flexible(true)
        .from_path("./scores.csv")?;

    let mut new_map: HashMap<u16, Vec<(u16, u8)>> = HashMap::with_capacity(1700);

    for record_maybe in reader.deserialize::<Fields>() {
        if let Ok(record) = record_maybe {
            match new_map.get(&record.user) {
                None => {
                    new_map.insert(record.user, Vec::new());
                    new_map.get_mut(&record.user).unwrap()
                }
                _ => {
                    new_map.get_mut(&record.user).unwrap()
                }
            }.push((record.film, record.rating));
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
