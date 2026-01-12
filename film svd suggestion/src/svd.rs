use std::collections::HashMap;
use rand::random_range;
use crate::MainTable;

pub struct Model {
    num_factors: usize,
    film_count: u16,
    global_avg: f64, 
    
    user_bias: HashMap<u16, f64>,
    film_bias: HashMap<u16, f64>, 
    user_factors: HashMap<u16, Vec<f64>>,
    film_factors: HashMap<u16, Vec<f64>>,
    
    implicit_factors: HashMap<u16, Vec<f64>> 
}

impl Model {
    pub fn new(main_table: MainTable, film_count: u16, num_factors: usize) -> Self {
        let mut sum = 0;
        let mut total = 0;
        for (_, hash) in main_table.iter() {
            sum += hash.iter().map(|&(_, r)| r as i32).sum::<i32>();
            total += hash.len();
        }
        let global_avg = sum as f64 / total as f64;

        Self {
            num_factors,
            film_count,
            global_avg,
            user_bias: HashMap::with_capacity(950),
            film_bias: HashMap::with_capacity(1700),
            user_factors: HashMap::with_capacity(950),
            film_factors: HashMap::with_capacity(1700),
            implicit_factors: HashMap::with_capacity(1700)
        }
    }

    pub fn train(&mut self, main_table: MainTable, epoch: u16, learning_rate: f64, reg: f64) {
        println!("Train started");
        self.initialize_params(main_table.clone());

        for i in 0..epoch {
            println!("Epoch number {i}");

            for (&user_id, user_ratings) in main_table.iter() {
                for &(film_id, rating) in user_ratings.iter() {
                    let b_u = self.user_bias[&user_id];
                    let b_i = self.film_bias[&film_id];

                    let p_tilde = self.compute_implicit_vector(Some(user_id), user_ratings);

                    let q_i = &self.film_factors[&film_id];

                    let prediction = self.global_avg
                        + b_u + b_i
                        + Self::dot_product(&p_tilde, q_i);


                    let error = rating as f64 - prediction;

                    *self.user_bias.get_mut(&user_id).unwrap() +=
                        learning_rate * (error - reg * b_u);

                    *self.film_bias.get_mut(&film_id).unwrap() +=
                        learning_rate * (error - reg * b_i);

                    let p_u = self.user_factors.get_mut(&user_id).unwrap();
                    for k in 0..self.num_factors {
                        p_u[k] += learning_rate * (error * q_i[k] - reg * p_u[k]);
                    }

                    let q_i = self.film_factors.get_mut(&film_id).unwrap();
                    for k in 0..self.num_factors {
                        q_i[k] += learning_rate * (error * p_tilde[k] - reg * q_i[k]);
                    }

                    let norm = (user_ratings.len() as f64).sqrt().recip();
                    for &(film, _) in user_ratings {
                        let y_j = self.implicit_factors.get_mut(&film).unwrap();
                        for k in 0..self.num_factors {
                            y_j[k] += learning_rate * (error * norm * q_i[k] - reg * y_j[k]);
                        }
                    }
                }
            }
        }
        println!("Training finished!");
    }

    fn initialize_params(&mut self, main_table: MainTable) {
        for &user_id in main_table.keys() {
            self.user_bias.insert(user_id, 0.0);
            self.user_factors.insert(
                user_id,
                (0..self.num_factors).map(|_| random_range(-0.01..0.01)).collect()
            );
        }

        for film_id in 1..=self.film_count {
            self.film_bias.entry(film_id).or_insert(0.0);
            self.film_factors.entry(film_id).or_insert_with(|| {
                (0..self.num_factors).map(|_| random_range(-0.01..0.01)).collect()
            });
            self.implicit_factors.entry(film_id).or_insert_with(|| {
                (0..self.num_factors).map(|_| random_range(-0.01..0.01)).collect()
            });
        }
    }

    // p_u + |I_u|^(-0.5) * sum(y_j)
    fn compute_implicit_vector(
        &self,
        user_id: Option<u16>,
        rated_films: &[(u16, u8)]
    ) -> Vec<f64> {
        let mut result = vec![0.0; self.num_factors];

        if let Some(uid) = user_id {
            if let Some(p_u) = self.user_factors.get(&uid) {
                result = p_u.clone();
            }
        }

        if rated_films.is_empty() {
            return result;
        }

        let norm = (rated_films.len() as f64).sqrt().recip();

        for &(film_id, rating) in rated_films {
            if let Some(y_j) = self.implicit_factors.get(&film_id) {
                let weight = (rating as f64 - 3.0) / 2.0;

                for k in 0..self.num_factors {
                    result[k] += norm * weight * y_j[k];
                }
            }
        }

        result
    }

    fn dot_product(a: &[f64], b: &[f64]) -> f64 {
        a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
    }

    pub fn predict(&self, answers: &Vec<(u16, u8)>) -> Vec<(u16, f64)> {
        let p_tilde = self.compute_implicit_vector(None, answers);

        let rated_film_ids: Vec<u16> = answers.iter().map(|(id, _)| *id).collect();

        let mut predict = Vec::new();

        for film_id in 1..=self.film_count {
            if !rated_film_ids.contains(&film_id) {
                let b_i = *self.film_bias.get(&film_id).unwrap_or(&0.0);

                let current = if let Some(q_i) = self.film_factors.get(&film_id) {
                    self.global_avg + b_i + Self::dot_product(&p_tilde, q_i)
                } else {
                    self.global_avg + b_i
                };

                let current = current.clamp(1.0, 5.0);
                predict.push((film_id, current));
            }
        }

        predict.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        predict.iter()
            .take(10)
            .map(|&(film_id, score)| (film_id, score))
            .collect()
    }
}
