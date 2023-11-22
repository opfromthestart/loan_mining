use std::str::FromStr;

use csv::ReaderBuilder;

#[derive(Debug)]
enum Value {
    Number(f32),
    Category(String),
    None,
}

impl FromStr for Value {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() == 0 {
            Ok(Self::None)
        } else if let Ok(n) = s.parse() {
            Ok(Self::Number(n))
        } else {
            Ok(Self::Category(s.into()))
        }
    }
}

fn main() {
    let loaded: Vec<Vec<Value>> = ReaderBuilder::new()
        .has_headers(false)
        .from_path("test.csv")
        .unwrap()
        .into_records()
        .into_iter()
        .map(|d| d.unwrap().into_iter().map(|x| x.parse().unwrap()).collect())
        .collect();
    println!("{loaded:?}");
}
